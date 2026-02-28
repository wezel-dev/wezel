mod cmd;
mod flush;
mod shell;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

use cmd::alias_cmd;
use flush::flush_events;

pub(crate) fn wezel_dir() -> PathBuf {
    dirs::home_dir()
        .expect("could not determine home directory")
        .join(".wezel")
}

fn handler_path(tool: &str) -> PathBuf {
    wezel_dir().join("bin").join(format!("pheromone-{tool}"))
}

fn exec_cmd(args: &[String]) -> anyhow::Result<ExitCode> {
    if args.is_empty() {
        anyhow::bail!("Usage: wezel exec -- <tool> [args...]");
    }

    let tool = &args[0];
    let tool_args = &args[1..];

    let handler = handler_path(tool);
    let (program, program_args): (&std::ffi::OsStr, &[String]) = if handler.is_file() {
        (handler.as_os_str(), tool_args)
    } else {
        (std::ffi::OsStr::new(tool.as_str()), tool_args)
    };

    let status = std::process::Command::new(program)
        .args(program_args)
        .status();

    let _ = flush_events();

    match status {
        Ok(s) => {
            let code = s.code().unwrap_or(1) as u8;
            Ok(ExitCode::from(code))
        }
        Err(e) => {
            eprintln!("wezel: failed to execute `{tool}`: {e}");
            Ok(ExitCode::from(127))
        }
    }
}

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
    /// Run a tool, recording pre/post build events.
    Exec {
        /// The tool and its arguments (use `--` before them).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Alias { tool, remove } => match alias_cmd(tool.as_deref(), remove) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("wezel: {e}");
                ExitCode::FAILURE
            }
        },
        Command::Exec { args } => match exec_cmd(&args) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("wezel: {e}");
                ExitCode::FAILURE
            }
        },
    }
}
