#!/usr/bin/env bash
# Complete local framework gate. Reference downloads are a separate setup step.
set -euo pipefail
cd "$(dirname "$0")/.."
for dataset in dev heldout1 heldout2 heldout3 agent_loops; do
  if [[ ! -d "ref/out/$dataset" ]]; then
    echo "Missing ref/out/$dataset; follow docs/DEVELOPING.md before the full gate." >&2
    exit 1
  fi
done
command -v node >/dev/null
command -v python3 >/dev/null
# The audit needs the network and cargo-audit; its result is reported, after the tests run.
security=0
python3 tools/check-security.py || security=$?
cargo test --release --features reference-tests -- --test-threads="${AFFORDANCE_TEST_THREADS:-2}"
cargo check --offline --all-features --all-targets
cargo check --offline --no-default-features --all-targets
cargo fmt --all --check
cargo clippy --offline --all-features --all-targets -- -D warnings
./target/release/affordance lint
./tools/check-package.sh
if [[ $security -ne 0 ]]; then echo "The dependency audit failed (above)." >&2; exit 1; fi
