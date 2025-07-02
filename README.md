# evict-icloud

a rust cli tool that frees up local storage by evicting downloaded icloud files from your file system while keeping them in the cloud.

## what it does

this tool recursively scans directories and uses macos's built-in `brctl evict` command to remove local copies of icloud files. the files remain accessible in icloud drive but no longer take up local disk space.

## prerequisites

* macos with icloud drive enabled
* rust toolchain (1.70+)

## installation & usage

```bash
# build the project
cargo build --release

# evict files from a directory (replace ~/documents with your target)
./target/release/evict-icloud ~/documents

# use multiple parallel processes for faster execution
./target/release/evict-icloud ~/documents -c 8

# see what would be evicted without actually doing it
./target/release/evict-icloud ~/documents --dry-run
```

## options

* `-c, --concurrency <n>` - number of parallel processes (defaults to cpu cores)
* `-d, --dry-run` - preview files that would be evicted

## development

```bash
# install cargo-watch for live reloading during development
cargo install cargo-watch

# run with auto-reload on file changes
cargo watch -x run -- "~/documents" --dry-run
```
