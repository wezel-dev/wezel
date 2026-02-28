use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

const FLUSH_LOCK: &str = ".flush.lock";
const SOURCE_MARKER: &str = "# >>> wezel pheromone >>>";
const SOURCE_END: &str = "# <<< wezel pheromone <<<";

fn wezel_dir() -> PathBuf {
    dirs::home_dir()
        .expect("could not determine home directory")
        .join(".wezel")
}

fn aliases_toml_path() -> PathBuf {
    wezel_dir().join("aliases.toml")
}

fn pheromone_dir() -> PathBuf {
    dirs::data_local_dir()
        .expect("could not determine local data directory")
        .join("pheromone")
}

fn events_dir() -> PathBuf {
    pheromone_dir().join("events")
}

// ── Shell detection ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shell {
    Zsh,
    Bash,
    Fish,
}

impl Shell {
    fn detect() -> Option<Self> {
        let shell = std::env::var("SHELL").ok()?;
        if shell.contains("zsh") {
            Some(Shell::Zsh)
        } else if shell.contains("bash") {
            Some(Shell::Bash)
        } else if shell.contains("fish") {
            Some(Shell::Fish)
        } else {
            None
        }
    }

    fn rc_path(self) -> PathBuf {
        let home = dirs::home_dir().expect("could not determine home directory");
        match self {
            Shell::Zsh => home.join(".zshrc"),
            Shell::Bash => {
                // Prefer .bashrc; fall back to .bash_profile
                let bashrc = home.join(".bashrc");
                if bashrc.exists() {
                    bashrc
                } else {
                    home.join(".bash_profile")
                }
            }
            Shell::Fish => home.join(".config/fish/conf.d/wezel.fish"),
        }
    }

    fn init_script_path(self) -> PathBuf {
        let dir = wezel_dir();
        match self {
            Shell::Zsh => dir.join("init.zsh"),
            Shell::Bash => dir.join("init.bash"),
            Shell::Fish => dir.join("init.fish"),
        }
    }

    fn source_block(self) -> String {
        let path = self.init_script_path();
        let p = path.display();
        match self {
            Shell::Zsh | Shell::Bash => {
                format!("{SOURCE_MARKER}\n[[ -f \"{p}\" ]] && source \"{p}\"\n{SOURCE_END}")
            }
            Shell::Fish => {
                format!(
                    "{SOURCE_MARKER}\nif test -f \"{p}\"\n    source \"{p}\"\nend\n{SOURCE_END}"
                )
            }
        }
    }

    fn alias_line(self, tool: &str) -> String {
        match self {
            Shell::Zsh | Shell::Bash => format!("alias {tool}=\"wezel exec -- {tool}\""),
            Shell::Fish => format!("alias {tool} \"wezel exec -- {tool}\""),
        }
    }

    fn render_init_script(self, aliases: &BTreeSet<String>) -> String {
        let mut out =
            String::from("# Managed by wezel — do not edit, aliases are stored in aliases.toml\n");
        for tool in aliases {
            out.push_str(&self.alias_line(tool));
            out.push('\n');
        }
        out
    }
}

// ── aliases.toml ─────────────────────────────────────────────────────────────

#[derive(Debug, Default, Serialize, Deserialize)]
struct AliasesFile {
    #[serde(default)]
    aliases: BTreeSet<String>,
}

fn load_aliases() -> anyhow::Result<AliasesFile> {
    let path = aliases_toml_path();
    if !path.exists() {
        return Ok(AliasesFile::default());
    }
    let contents = fs::read_to_string(&path)?;
    let file: AliasesFile = toml::from_str(&contents)?;
    Ok(file)
}

fn save_aliases(file: &AliasesFile) -> anyhow::Result<()> {
    let dir = wezel_dir();
    fs::create_dir_all(&dir)?;
    let contents = toml::to_string_pretty(file)?;
    fs::write(aliases_toml_path(), contents)?;
    Ok(())
}

// ── Ensure shell hook ────────────────────────────────────────────────────────

fn ensure_shell_hook(shell: Shell) -> anyhow::Result<()> {
    let rc = shell.rc_path();

    // For fish, ensure parent dir exists
    if shell == Shell::Fish {
        if let Some(parent) = rc.parent() {
            fs::create_dir_all(parent)?;
        }
    }

    let contents = if rc.exists() {
        fs::read_to_string(&rc)?
    } else {
        String::new()
    };

    if contents.contains(SOURCE_MARKER) {
        return Ok(());
    }

    let mut file = fs::OpenOptions::new().create(true).append(true).open(&rc)?;

    writeln!(file)?;
    writeln!(file, "{}", shell.source_block())?;

    println!("Installed source hook in {}", rc.display());
    Ok(())
}

// ── Sync aliases.toml → init script ─────────────────────────────────────────

fn sync_init_script(shell: Shell, aliases: &AliasesFile) -> anyhow::Result<()> {
    let dir = wezel_dir();
    fs::create_dir_all(&dir)?;
    let script = shell.render_init_script(&aliases.aliases);
    fs::write(shell.init_script_path(), script)?;
    Ok(())
}

// ── Alias command ────────────────────────────────────────────────────────────

fn alias_cmd(tool: Option<&str>, remove: bool) -> anyhow::Result<()> {
    let shell = Shell::detect()
        .ok_or_else(|| anyhow::anyhow!("Could not detect shell from $SHELL env var"))?;

    let mut aliases = load_aliases()?;

    match tool {
        None => {
            // No tool given — just ensure the shell hook is in place and sync.
            ensure_shell_hook(shell)?;
            sync_init_script(shell, &aliases)?;
            if aliases.aliases.is_empty() {
                println!("Shell hook is set up. No aliases configured yet.");
            } else {
                println!(
                    "Shell hook is set up. {} alias(es) active: {}",
                    aliases.aliases.len(),
                    aliases
                        .aliases
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }
        Some(tool) => {
            if remove {
                if aliases.aliases.remove(tool) {
                    save_aliases(&aliases)?;
                    sync_init_script(shell, &aliases)?;
                    println!("Removed alias for `{tool}`.");
                } else {
                    println!("No alias for `{tool}` found.");
                }
            } else {
                ensure_shell_hook(shell)?;
                if aliases.aliases.insert(tool.to_string()) {
                    save_aliases(&aliases)?;
                    sync_init_script(shell, &aliases)?;
                    println!("Added alias for `{tool}`.");
                } else {
                    // Still sync in case init script is out of date.
                    sync_init_script(shell, &aliases)?;
                    println!("Alias for `{tool}` already present.");
                }
            }
        }
    }

    Ok(())
}

// ── Flush machinery ──────────────────────────────────────────────────────────

struct FlushLock {
    path: PathBuf,
}

impl FlushLock {
    fn try_acquire(dir: &std::path::Path) -> Option<Self> {
        let path = dir.join(FLUSH_LOCK);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => Some(Self { path }),
            Err(_) => None,
        }
    }
}

impl Drop for FlushLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn flush_events() -> anyhow::Result<()> {
    let events_dir = events_dir();
    if !events_dir.exists() {
        return Ok(());
    }

    let Some(_lock) = FlushLock::try_acquire(&events_dir) else {
        return Ok(());
    };

    let entries: Vec<PathBuf> = fs::read_dir(&events_dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
        .collect();

    if entries.is_empty() {
        return Ok(());
    }

    let mut events: Vec<serde_json::Value> = Vec::with_capacity(entries.len());
    for path in &entries {
        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
            let _ = fs::remove_file(path);
            continue;
        };
        events.push(value);
    }

    if events.is_empty() {
        return Ok(());
    }

    let url = std::env::var("BURROW_URL").unwrap_or_else(|_| "http://localhost:3001".into());

    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(5))
        .build();

    match agent
        .post(&format!("{url}/api/events"))
        .send_json(serde_json::Value::Array(events))
    {
        Ok(_) => {
            for path in &entries {
                let _ = fs::remove_file(path);
            }
        }
        Err(_) => {}
    }

    Ok(())
}

fn post() -> anyhow::Result<()> {
    flush_events()
}

// ── CLI ──────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "wezel", about = "Lightweight build observer")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage tool aliases. Without arguments, ensures the shell hook is installed.
    Alias {
        /// The tool to alias (e.g. cargo, go, npm). Omit to just set up the shell hook.
        tool: Option<String>,
        /// Remove the alias instead of installing it.
        #[arg(long)]
        remove: bool,
    },
    Post {
        args: Vec<String>,
    },
    Pre {
        args: Vec<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Alias { tool, remove } => alias_cmd(tool.as_deref(), remove),
        Command::Post { .. } => post(),
        Command::Pre { .. } => Ok(()),
    }
}
