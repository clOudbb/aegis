#!/usr/bin/env sh
set -eu

cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-features
cargo doc --no-deps --all-features
