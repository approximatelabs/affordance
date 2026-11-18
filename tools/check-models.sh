#!/usr/bin/env bash
# Every model in views/index.json, hand-written and converted, on every case set, and every
# tokenizer under ref/tokenizers through the tests that walk tokenizers. The test suite covers
# the hand-written models and a sample of the converted ones; this is the whole inventory.
# Run it after adding or regenerating converted views, after a change to the converter or its
# runtime (tools/jinja2tsx.py, views/common/jinja.tsx), and before a release. A few minutes.
#
#   ./tools/check-models.sh
set -uo pipefail
cd "$(dirname "$0")/.."
cargo build --release --bin affordance 2>&1 | tail -1
failed=0
./target/release/affordance check --threads "$(nproc)" --set dev,heldout1,heldout2,heldout3,heldout_a2,agent_loops \
  | grep -E "^==|^TOTAL|LOAD ERROR|MISSING" || failed=1
[ "${PIPESTATUS[0]}" -eq 0 ] || failed=1
# (One run for each: the integration tests are one program, tests/suite.rs, and a file's tests
# are named `file::test`.)
for tests in "--test suite spill::" "--test suite safe_text::" "--lib fast_splits_match_onig_on_case_texts" "--test suite any_model::"; do
  # shellcheck disable=SC2086
  out=$(AFFORDANCE_ALL_MODELS=1 cargo test --release --features reference-tests $tests 2>&1) || failed=1
  echo "$out" | grep -E "^test result: .* [1-9][0-9]* passed|FAILED|panicked|^---- " || true
done
if [ $failed -eq 0 ]; then echo "all models checked"; else echo "model check FAILED"; exit 1; fi
