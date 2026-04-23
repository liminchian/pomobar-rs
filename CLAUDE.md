# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`pomobar-rs` is a Waybar plugin implementing a Pomodoro timer. It is structured as a **client-server application** communicating over a Unix domain socket at `/tmp/pomobar.sock`.

Two binaries are produced:
- `pomobar` — long-running server daemon managing timer state
- `pomobar-cli` — lightweight CLI client for sending commands and querying status

## Commands

```shell
# Build
cargo build --release

# Run tests
cargo test --verbose

# Lint
cargo clippy

# Format check
cargo fmt --check

# Run server (dev)
cargo run --bin pomobar

# Run client (dev)
cargo run --bin pomobar-cli -- status
cargo run --bin pomobar-cli -- toggle
cargo run --bin pomobar-cli -- reset
```

## Architecture

### State Machine (`src/models.rs`)

The core of the application is a **typestate pattern** state machine. `Pomobar<S>` is generic over the current state, making invalid transitions a compile error.

States:
- `Idle` → start of session, no timer running
- `Work` (25 min) → active focus session; holds `started_at: DateTime<Utc>` and `cycles: u32`
- `Paused` → work interrupted; holds remaining `Duration` and cycle count
- `ShortBreak` (5 min) → after each work session (except every 4th cycle)
- `LongBreak` (15 min) → after every 4th completed work cycle

Valid transitions:
```
Idle    --start()-->  Work
Work    --pause()-->  Paused
Work    --finish()--> ShortBreak | LongBreak  (LongBreak if cycles % 4 == 0)
Paused  --resume()--> Work
ShortBreak --finish()--> Work
LongBreak  --finish()--> Work (resets cycle count to 0)
```

`PomobarDispatcher` is a plain enum wrapping all state variants. It is the runtime representation used by the server and provides:
- `get_remaining_time()` — returns `Option<Duration>` (None in Idle)
- `to_view()` — serialises current state to Waybar-compatible JSON with `text` (MM:SS), `tooltip`, `class`, and `percentage` fields

### Server (`src/bin/server.rs`)

Runs a `tokio` async event loop. Internal events flow through an `mpsc` channel:
- `Tick` — emitted every 1 second by a spawned timer task
- `Toggle` — start or pause, received from a socket client
- `Reset` — return to Idle, received from a socket client
- `Status` — respond with current JSON state, received from a socket client

The server deletes and recreates `/tmp/pomobar.sock` on startup. State transitions and Waybar output are handled entirely in the main event loop.

### Client (`src/bin/client.rs`)

Connects to the socket, writes a single command string (`"toggle"`, `"reset"`, or `"status"`), reads the response, and exits. Built with `clap` subcommands.

## Key Conventions

- **Error handling**: uses `anyhow` throughout; propagate with `?` rather than unwrapping.
- **Async**: all async code uses `tokio` with the `#[tokio::main]` macro.
- **Logging**: use `tracing` macros (`trace!`, `info!`, `warn!`, `error!`) — the server initialises a subscriber at TRACE level.
- **Serialisation**: `serde` with `serde_json` is used for all socket communication and Waybar output.
- **Time**: `chrono` with the `serde` feature for timestamps in state structs.

## CI

GitHub Actions (`.github/workflows/rust.yml`) runs on push/PR to `main`:
1. `cargo build --verbose`
2. `cargo test --verbose`

Ubuntu latest is the only CI target.
