use std::collections::BTreeSet;
use std::fs;

use serde::{Deserialize, Serialize};

use crate::shell::{Shell, ensure_shell_hook, sync_init_script};
use crate::wezel_dir;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AliasesFile {
    #[serde(default)]
    pub aliases: BTreeSet<String>,
}

fn aliases_toml_path() -> std::path::PathBuf {
    wezel_dir().join("aliases.toml")
}

pub fn load_aliases() -> anyhow::Result<AliasesFile> {
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

pub fn alias_cmd(tool: Option<&str>, remove: bool) -> anyhow::Result<()> {
    let shell = Shell::detect()
        .ok_or_else(|| anyhow::anyhow!("Could not detect shell from $SHELL env var"))?;

    let mut aliases = load_aliases()?;

    match tool {
        None => {
            ensure_shell_hook(shell)?;
            sync_init_script(shell, &aliases.aliases)?;
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
                    sync_init_script(shell, &aliases.aliases)?;
                    println!("Removed alias for `{tool}`.");
                } else {
                    println!("No alias for `{tool}` found.");
                }
            } else {
                ensure_shell_hook(shell)?;
                if aliases.aliases.insert(tool.to_string()) {
                    save_aliases(&aliases)?;
                    sync_init_script(shell, &aliases.aliases)?;
                    println!("Added alias for `{tool}`.");
                } else {
                    sync_init_script(shell, &aliases.aliases)?;
                    println!("Alias for `{tool}` already present.");
                }
            }
        }
    }

    Ok(())
}
