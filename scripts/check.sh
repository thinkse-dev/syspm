#!/bin/sh
# Run the same quality checks as CI (.github/workflows/ci.yml), in the same order.
set -eu
cd "$(dirname "$0")/.."

actionlint
dprint check
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
