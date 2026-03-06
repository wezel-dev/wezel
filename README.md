# Wezel — Your build, always at its best.

Wezel is an open-source build observability toolsuite. It tracks how your builds behave over time, surfaces which scenarios hurt your team most, and alerts you the moment a commit causes a regression — before it becomes your new normal.

## Why Wezel

Build time creep is invisible until it's unbearable. A change that adds 15 seconds to your most common build scenario compounds across every developer, every day. By the time it's noticeable, it's baked into your baseline.

Wezel catches regressions at the commit level, while they're still easy to revisit.

## Getting Started

### Prerequisites

- Rust (see `rust-toolchain.toml` for the pinned version)
- PostgreSQL (for Burrow)
- Docker / Docker Compose (optional, for local stack)

### Run the local stack


```sh
docker compose up
```

### Build from source

```sh
cargo build --workspace
```

### Install Pheromone

Add the hook to your shell (example for `zsh`):

```sh
eval "$(pheromone init zsh)"
```

Then configure your Anthill endpoint:

```sh
wezel config set endpoint http://your-anthill-instance
```

From that point on, every build you run is observed and flushed automatically.

## Self-hosting

Wezel is designed to be fully self-hosted. No telemetry, no vendor lock-in. Run Burrow and Anthill on your own infrastructure and keep your build data on your own machines.

See [`ARCHITECTURE.md`](ARCHITECTURE.md) for a deeper dive into design decisions and component interactions.

## License

Dual-licensed under [Apache 2.0](LICENSE-APACHE) and [AGPL](LICENSE-AGPL). See the respective files for details.