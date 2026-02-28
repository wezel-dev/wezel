use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Base URL of the burrow API to push build timings to.
    pub burrow_url: String,
}

/// Walk up from `start` looking for a `.wezel/config.toml`.
/// Returns (project_wezel_dir, parsed config) if found.
pub fn discover(start: &Path) -> Option<(PathBuf, Config)> {
    log::debug!("discovering config from {}", start.display());
    let mut dir = start.to_path_buf();
    loop {
        let candidate = dir.join(".wezel").join("config.toml");
        if candidate.is_file() {
            log::debug!("found config at {}", candidate.display());
            let wezel_dir = dir.join(".wezel");
            let config = load(&candidate)?;
            log::debug!("loaded config: burrow_url={}", config.burrow_url);
            return Some((wezel_dir, config));
        }
        if !dir.pop() {
            log::debug!("no .wezel/config.toml found in any ancestor");
            return None;
        }
    }
}

fn load(path: &Path) -> Option<Config> {
    let contents = fs::read_to_string(path).ok()?;
    toml::from_str(&contents).ok()
}
