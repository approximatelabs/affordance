#!/usr/bin/env bash
# PR gate, as CI runs it: no downloaded models, reference outputs or running services; the few
# tokenizers the tests load come from tools/fetch-test-tokenizers.sh (fetched if missing).
set -euo pipefail
cd "$(dirname "$0")/.."
command -v node >/dev/null
command -v python3 >/dev/null
# Every test; those that need the full reference data under ref/ are ignored without it.
# Everything the later offline steps need, for every feature.
cargo fetch --locked
[ -f ref/tokenizers/zai-org__GLM-5.3.json ] || ./tools/fetch-test-tokenizers.sh
cargo test --profile ci --features test-tokenizers --lib --bins --tests -- --test-threads="${AFFORDANCE_TEST_THREADS:-8}"
cargo fmt --all --check
cargo clippy --offline --all-features --all-targets -- -D warnings
cargo clippy --offline --no-default-features --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --offline --no-deps --all-features
./target/ci/affordance lint
./tools/check-package.sh
