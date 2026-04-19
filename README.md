# RustLog

A Rust CLI and library for **filtering log files** (substring or regex, single or multi-pattern), **streaming** with bounded memory, optional **`tail -f`–style** follow mode, **TOML config**, optional **file sink**, **transform pipeline**, optional **WebSocket dashboard**, and an optional **Kafka** producer (behind `--features kafka`).

---

## Features

- **Streaming I/O** — one-shot read and tail modes process line-by-line (`stream_file_lines_once` / `tail_file_async`).
- **Matching** — `LineMatcher`: plain contains, regex, `any` / `all` modes (see TOML `[filters]`).
- **Tail semantics** — async tail **seeks to EOF** before following; **adaptive idle backoff** (8 ms → 512 ms).
- **Output** — tracing to stderr/stdout via `RUST_LOG`; optional **append-only file** (`-o` / `[output].file`).
- **Config** — `-C/--config` merges `[source]`, `[filters]`, `[output]`, `[[transforms]]`, `[kafka]`, `[web]` with CLI overrides.
- **Transforms** — pluggable steps (e.g. trim, prepend, strip prefix, regex replace).
- **Web UI** — optional Axum + WebSocket dashboard (`--web` or `[web]`).
- **Kafka** — optional `rdkafka` integration when built with `--features kafka` (needs system `librdkafka`).

---

## Getting started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition **2024**)
- For Kafka builds: system **librdkafka** (e.g. `brew install librdkafka` on macOS)

### Clone and build

```bash
git clone https://github.com/harshsh-dev/RustLog.git
cd RustLog/rustlog
cargo build --release
```

### Simple filter (two positionals)

```bash
RUST_LOG=info cargo run --release -- ./sample/sample.log ERROR
```

### Tailing

```bash
RUST_LOG=info cargo run --release -- ./sample/sample.log ERROR --tail
```

Append in another terminal:

```bash
echo "ERROR: Out of memory!" >> ./sample/sample.log
```

Only bytes **after** the process attached at EOF are followed (same idea as `tail -f`).

### Write matches to a file (`-o` / `--out`)

```bash
cargo run --release -- ./sample/sample.log ERROR -o ./matches.log
```

CLI `-o` overrides `[output].file` from TOML when both are present.

### Config file (`-C` / `--config`)

```bash
cargo run --release -- -C ./rustlog.toml
```

Example `rustlog.toml`:

```toml
[source]
path = "./sample/sample.log"

[filters]
patterns = ["ERROR", "WARN"]
use_regex = false
mode = "any"

[output]
stdout = true
# file = "./archive.log"

[web]
enabled = false
# bind = "127.0.0.1:8080"

[kafka]
enabled = false
```

With config, you can omit positionals if `[source].path` and `[filters]` (or match-all empty patterns) are set; see `config::ResolvedConfig` in the crate.

### Web dashboard

```bash
cargo run --release -- ./sample/sample.log ERROR --web 127.0.0.1:8080
```

### Kafka (optional feature)

```bash
cargo build --release --features kafka
```

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
├── README.md
└── rustlog/
    ├── Cargo.toml
    ├── sample/
    │   └── sample.log
    ├── src/
    │   ├── lib.rs            # `run()` — wiring, Ctrl+C, reader + sinks + optional web/Kafka
    │   ├── main.rs           # `rustlog::run()`
    │   ├── args.rs           # Clap CLI
    │   ├── config.rs         # TOML load + merge with CLI
    │   ├── matcher.rs        # Line matching (substring / regex)
    │   ├── filter.rs         # Legacy helpers + unit tests
    │   ├── reader.rs         # Blocking helpers + `tail_file`
    │   ├── reader_async.rs   # Async tail + one-shot stream
    │   ├── sink.rs           # Stdout tracing + optional file append
    │   ├── transform.rs      # Transform pipeline
    │   ├── kafka_sink.rs     # Optional Kafka (feature `kafka`)
    │   └── web_dashboard.rs  # Axum + WebSocket
    └── tests/
        ├── cli_tests.rs
        ├── filter_tests.rs
        ├── tail_tests.rs
        ├── tail_async.rs
        ├── stream_once_async.rs
        ├── web_router.rs
        └── kafka_cli_error.rs
```

---

## Built with

| Crate | Role |
|--------|------|
| `tokio` | Async runtime, I/O, channels, signals |
| `clap` | CLI |
| `anyhow` / `toml` / `serde` | Errors and config |
| `regex` | Pattern matching |
| `tracing` / `tracing-subscriber` | Diagnostics |
| `axum` / `tower-http` | Web dashboard |
| `rdkafka` (optional) | Kafka producer |

Dev: `assert_cmd`, `predicates`, `tempfile`, `tower`, `http-body-util`.

---

## Roadmap

| Area | Status |
|------|--------|
| CLI filter + tail | Done |
| Streaming / bounded memory | Done |
| TOML config + CLI merge | Done |
| File output (`-o`, `[output].file`) | Done |
| Transform pipeline | Done |
| WebSocket dashboard | Done |
| Kafka (optional feature) | Done (opt-in build) |
| JSON field filters / structured logs | Planned |
| Terminal colors by level | Planned |
| Filesystem `notify` instead of polling | Planned |

---

## Contributing

Pull requests are welcome. Fork, branch, and open a PR with a short description of the change.

---

## Author

**Harsh Sharma**  
[GitHub](https://github.com/HarshSharma009) · [Repository](https://github.com/harshsh-dev/RustLog) · [LinkedIn](https://www.linkedin.com/in/harsh-sharma-8a850b173/) · [harshsharma.ext@gmail.com](mailto:harshsharma.ext@gmail.com)

---

## License

GNU General Public License (see the license file in the repository).
