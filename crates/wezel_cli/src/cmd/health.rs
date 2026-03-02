use std::fs;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use url::Url;

use crate::config;
use crate::pheromones_dir;

pub fn health_cmd() -> anyhow::Result<()> {
    // 1. List available pheromones
    let pdir = pheromones_dir();
    println!("pheromones dir: {}", pdir.display());
    match fs::read_dir(&pdir) {
        Ok(entries) => {
            let mut found = false;
            for entry in entries.filter_map(Result::ok) {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with("pheromone-") {
                    println!("  ✓ {name}");
                    found = true;
                }
            }
            if !found {
                println!("  (none)");
            }
        }
        Err(_) => {
            println!("  ⚠ directory not found");
        }
    }

    // 2. Check config
    let cwd = std::env::current_dir().unwrap_or_default();
    println!();
    match config::discover(&cwd) {
        Some((wezel_dir, config)) => {
            println!("config: {} ✓", wezel_dir.join("config.toml").display());

            // 3. Ping burrow
            println!();
            print!("burrow ({}): ", config.burrow_url);
            match ping_burrow(&config.burrow_url) {
                Ok(()) => println!("reachable ✓"),
                Err(e) => println!("⚠ unreachable — {e}"),
            }
        }
        None => {
            println!("config: ⚠ no .wezel/config.toml found (run `wezel setup`)");
        }
    }

    Ok(())
}

fn ping_burrow(base_url: &str) -> anyhow::Result<()> {
    let parsed = Url::parse(base_url)?;
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("no host in URL"))?;
    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| anyhow::anyhow!("no port in URL"))?;
    let addr = format!("{host}:{port}")
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow::anyhow!("could not resolve {host}"))?;
    TcpStream::connect_timeout(&addr, Duration::from_secs(5))?;
    Ok(())
}
