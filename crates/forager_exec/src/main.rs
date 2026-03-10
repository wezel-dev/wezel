use std::collections::HashMap;
use std::process;

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|a| a == "--schema") {
        println!(
            "{}",
            serde_json::json!({
                "name": "exec",
                "description": "Executes a shell command; produces no measurements",
                "inputs": {
                    "cmd": { "type": "string", "description": "Shell command to run" },
                    "env": { "type": "object", "description": "Extra environment variables" },
                    "cwd": { "type": "string", "description": "Working directory override" }
                },
                "output": null
            })
        );
        return Ok(());
    }

    let inputs_path =
        std::env::var("FORAGER_INPUTS").context("FORAGER_INPUTS not set")?;
    let out_path = std::env::var("FORAGER_OUT").context("FORAGER_OUT not set")?;

    let inputs_raw = std::fs::read_to_string(&inputs_path)
        .with_context(|| format!("reading {inputs_path}"))?;
    let inputs: serde_json::Value =
        serde_json::from_str(&inputs_raw).context("parsing FORAGER_INPUTS")?;

    let cmd = inputs["cmd"]
        .as_str()
        .context("inputs.cmd is required for forager-exec")?;

    let env_vars: HashMap<String, String> = inputs
        .get("env")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let cwd = inputs.get("cwd").and_then(|v| v.as_str());

    let mut child = process::Command::new("sh");
    child.arg("-c").arg(cmd);
    for (k, v) in &env_vars {
        child.env(k, v);
    }
    if let Some(dir) = cwd {
        child.current_dir(dir);
    }

    // Write the null measurement output before running (exec always produces no measurement).
    let envelope = wezel_types::ForagerPluginEnvelope { measurement: None };
    std::fs::write(&out_path, serde_json::to_string(&envelope)?)
        .with_context(|| format!("writing {out_path}"))?;

    let status = child.status().context("failed to spawn command")?;
    process::exit(status.code().unwrap_or(1));
}
