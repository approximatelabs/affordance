#!/usr/bin/env bash
# PR gate: no downloaded models, tokenizer files, reference outputs or running services.
set -euo pipefail
cd "$(dirname "$0")/.."
command -v node >/dev/null
command -v python3 >/dev/null
# Every test; those that need the reference data under ref/ are ignored without it.
# Everything the later offline steps need, for every feature.
cargo fetch --locked
cargo test --profile ci --lib --bins --tests -- --test-threads="${AFFORDANCE_TEST_THREADS:-8}"
cargo fmt --all --check
cargo clippy --offline --all-features --all-targets -- -D warnings
cargo clippy --offline --no-default-features --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --offline --no-deps --all-features
cargo run --profile ci --offline --bin affordance -- lint
./tools/check-package.sh
