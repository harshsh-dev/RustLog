# RustLog

A small, fast CLI for **keyword filtering** on log files, with optional **`tail -f`–style** follow mode. Written in Rust with a clear split between a reusable library (`rustlog::run`) and a thin binary entrypoint. The longer-term idea is to grow toward streaming sinks (Kafka), config-driven rules, and richer tooling—without sacrificing efficiency on large files.

---

## Features

- **Streaming filter mode** — reads line-by-line with **O(1)** memory in file size (no whole-file `Vec`).
- **Tail mode (`--tail`)** — opens the file, **seeks to end of file**, then reads new lines (same idea as `tail -f`).
- **Adaptive idle polling** — backoff from 8 ms up to 512 ms when no new data, so quiet logs use less CPU and active logs wake up quickly.
- **Filter in the tail task** — only matching lines are sent on the async channel (less traffic and fewer allocations than filtering only in the consumer).
- **Graceful shutdown** — Tokio `ctrl_c` handler stops the tail loop cleanly.
- **Diagnostics** — [`tracing`](https://docs.rs/tracing) + [`tracing-subscriber`](https://docs.rs/tracing-subscriber) with `RUST_LOG` / env filter.
- **Tests** — unit tests, integration tests, and CLI checks via `assert_cmd` and `tempfile`.

---

## Getting started

### Prerequisites

- [Rust toolchain](https://www.rust-lang.org/tools/install) (stable; this crate uses **edition 2024**)

### Clone and run

```bash
git clone https://github.com/HarshSharma009/RustLog.git
cd RustLog/rustlog
cargo build --release
RUST_LOG=info cargo run --release -- ./sample/sample.log ERROR
```

### Example output

You should see lines from the sample file that contain the keyword (via the `filtered` tracing target):

```text
ERROR: Failed to connect
ERROR: Timeout
```

### Tailing (`--tail`)

```bash
RUST_LOG=info cargo run --release -- ./sample/sample.log ERROR --tail
```

In another terminal, append a line:

```bash
echo "ERROR: Out of memory!" >> ./sample/sample.log
```

Only **new** bytes after the process attached at EOF are streamed—same semantics as `tail -f`.

### Tests

```bash
cd RustLog/rustlog
cargo test
cargo clippy --all-targets
```

---

## Project layout

```text
RustLog/
├── README.md                 # This file
└── rustlog/
    ├── Cargo.toml
    ├── sample/
    │   └── sample.log
    ├── src/
    │   ├── lib.rs            # `run()`, tracing setup, orchestration
    │   ├── main.rs           # Thin `#[tokio::main]` → `rustlog::run()`
    │   ├── args.rs           # CLI (`clap`)
    │   ├── filter.rs         # Keyword match helper + `filter_lines` (tests / small batches)
    │   ├── reader.rs         # Streaming `for_each_matching_line`, blocking `tail_file`
    │   └── reader_async.rs   # Async `tail_file_async` (seek EOF + backoff)
    └── tests/
        ├── cli_tests.rs
        ├── filter_tests.rs
        ├── tail_tests.rs
        └── tail_async.rs
```

---

## Built with

| Crate | Role |
|--------|------|
| [`clap`](https://docs.rs/clap) | CLI parsing |
| [`tokio`](https://docs.rs/tokio) | Async runtime (`rt-multi-thread`, `signal`, `fs`, `io-util`, `sync`, `time`, …) |
| [`anyhow`](https://docs.rs/anyhow) | Error propagation in `run()` |
| [`tracing`](https://docs.rs/tracing) / [`tracing-subscriber`](https://docs.rs/tracing-subscriber) | Structured logs and env filter |
| [`assert_cmd`](https://docs.rs/assert_cmd) / [`predicates`](https://docs.rs/predicates) / [`tempfile`](https://docs.rs/tempfile) | **Dev-only** integration / CLI tests |

---

## Roadmap

| Feature | Status |
|---------|--------|
| Basic CLI keyword filter | Done |
| Streaming read (bounded memory) | Done |
| Real-time tail (`tail -f` semantics + async) | Done |
| Kafka / other sinks | Planned |
| Configurable rules (e.g. `.toml`) | Planned |
| Plugins / transforms | Planned |
| Optional web UI (e.g. Axum + WebSocket) | Optional |

---

## Possible enhancements

- Colorized levels (e.g. ERROR in red) in the terminal
- JSON log parsing and field-based filters
- `--out <file>` to write matches to a file
- Filesystem **watch** integration (e.g. `notify`) to reduce or remove idle polling

---

## Learning goals

This repo is structured to exercise:

- Ownership, borrowing, and sensible buffer reuse
- A **library + binary** crate layout (`pub` API vs CLI)
- Sync I/O vs async I/O (`BufRead` / `AsyncBufRead`)
- Channels, shutdown flags, and cooperative cancellation

---

## Contributing

Pull requests are welcome. Fork, branch, and open a PR with a short description of the change.

---

## Author

**Harsh Sharma**  
[GitHub](https://github.com/HarshSharma009) | [LinkedIn](https://www.linkedin.com/in/harsh-sharma-8a850b173/)  
[harshsharma.ext@gmail.com](mailto:harshsharma.ext@gmail.com)

---

## License

GNU General Public License (see repository license file).
