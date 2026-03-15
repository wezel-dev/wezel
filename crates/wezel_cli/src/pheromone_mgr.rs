//! Pheromone binary update manager.
//!
//! Queries burrow for the latest pheromone versions, downloads updated
//! binaries for the current platform, and places them in `pheromone_dir`.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Minimal subset of the `/api/pheromones` response we need.
#[derive(Debug, Deserialize)]
struct PheromoneEntry {
    name: String,
    #[serde(rename = "githubRepo")]
    github_repo: String,
    version: String,
    platforms: Vec<String>,
}

/// Returns the target triple for the current platform, or `None` if unknown.
fn current_target() -> Option<&'static str> {
    // Use compile-time target_arch / target_os / target_env to produce a
    // best-effort Rust target triple string.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return Some("aarch64-apple-darwin");
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return Some("x86_64-apple-darwin");
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return Some("x86_64-unknown-linux-gnu");
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return Some("aarch64-unknown-linux-gnu");
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return Some("x86_64-pc-windows-msvc");
    #[allow(unreachable_code)]
    None
}

fn version_file(pheromone_dir: &Path, name: &str) -> PathBuf {
    pheromone_dir.join(format!("{name}.version"))
}

fn installed_version(pheromone_dir: &Path, name: &str) -> Option<String> {
    std::fs::read_to_string(version_file(pheromone_dir, name))
        .ok()
        .map(|s| s.trim().to_string())
}

fn write_version(pheromone_dir: &Path, name: &str, version: &str) -> std::io::Result<()> {
    std::fs::write(version_file(pheromone_dir, name), version)
}

/// Check burrow for updated pheromone versions and download any that are newer.
///
/// * `server_url` — base URL of burrow (e.g. `http://localhost:3001`)
/// * `pheromone_dir` — directory where binaries are stored
pub fn update_pheromones(server_url: &str, pheromone_dir: &Path) {
    let Some(target) = current_target() else {
        log::debug!("pheromone_mgr: unknown platform, skipping update");
        return;
    };

    let url = format!("{}/api/pheromones", server_url.trim_end_matches('/'));
    let agent = ureq::Agent::new();
    let entries: Vec<PheromoneEntry> = match agent.get(&url).call() {
        Ok(r) => match r.into_json() {
            Ok(v) => v,
            Err(e) => {
                log::warn!("pheromone_mgr: failed to parse pheromones: {e}");
                return;
            }
        },
        Err(e) => {
            log::warn!("pheromone_mgr: failed to fetch pheromones: {e}");
            return;
        }
    };

    if let Err(e) = std::fs::create_dir_all(pheromone_dir) {
        log::warn!("pheromone_mgr: cannot create pheromone dir: {e}");
        return;
    }

    for entry in &entries {
        if !entry.platforms.iter().any(|p| p == target) {
            log::debug!(
                "pheromone_mgr: {} not available for {target}, skipping",
                entry.name
            );
            continue;
        }

        let current = installed_version(pheromone_dir, &entry.name);
        if current.as_deref() == Some(&entry.version) {
            log::debug!(
                "pheromone_mgr: {} is up to date ({})",
                entry.name,
                entry.version
            );
            continue;
        }

        log::info!(
            "pheromone_mgr: updating {} {} → {}",
            entry.name,
            current.as_deref().unwrap_or("(not installed)"),
            entry.version
        );

        let bin_name = if cfg!(target_os = "windows") {
            format!("{}-{}.exe", entry.name, target)
        } else {
            format!("{}-{}", entry.name, target)
        };
        let download_url = format!(
            "https://github.com/{}/releases/download/v{}/{}",
            entry.github_repo, entry.version, bin_name
        );

        let dest = pheromone_dir.join(&entry.name);
        match download_binary(&agent, &download_url, &dest) {
            Ok(()) => {
                if let Err(e) = write_version(pheromone_dir, &entry.name, &entry.version) {
                    log::warn!("pheromone_mgr: failed to write version file: {e}");
                }
                log::info!("pheromone_mgr: updated {} to {}", entry.name, entry.version);
            }
            Err(e) => {
                log::warn!("pheromone_mgr: failed to download {}: {e}", entry.name);
            }
        }
    }
}

fn download_binary(agent: &ureq::Agent, url: &str, dest: &Path) -> anyhow::Result<()> {
    use std::io::Read;
    let resp = agent.get(url).call()?;
    let mut bytes = Vec::new();
    resp.into_reader().read_to_end(&mut bytes)?;

    // Write to a temp file then rename for atomicity.
    let tmp = dest.with_extension("tmp");
    std::fs::write(&tmp, &bytes)?;

    // Make executable on Unix.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&tmp)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&tmp, perms)?;
    }

    std::fs::rename(&tmp, dest)?;
    Ok(())
}
