# evict-icloud (Rust)

A blazing-fast Rust CLI that evicts (removes local copies of) downloaded iCloud files
inside a directory tree using the native `brctl evict` command.

## Prerequisites

* macOS with iCloud Drive and the `brctl` utility available (ships by default)
* Rust toolchain (1.70+ recommended)

## Build & Run

```bash
cd rust/
cargo build --release
# run
./target/release/evict-icloud ~/Documents -c 8
```

Dry-run:

```bash
./target/release/evict-icloud ~/Documents --dry-run
```

## Flags

* `-c, --concurrency <N>` — number of parallel `brctl` processes (defaults to CPU cores)
* `-d, --dry-run` — list files that would be evicted without executing

## Development

```bash
cargo install cargo-watch
cargo watch -x run -- "~/Documents" --dry-run
```

## License

MIT 