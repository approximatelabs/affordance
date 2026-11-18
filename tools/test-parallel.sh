#!/usr/bin/env bash
# The whole test suite: the same tests as `cargo test --release --features reference-tests`
# (plus the doc tests), with the timing-sensitive ones run after the rest.
#
#   ./tools/test-parallel.sh       (extra cargo arguments via CARGO_TEST_ARGS; SLOWEST=N lists N tests)
#
# The integration tests are built as one program (tests/suite.rs) and run as one process for
# each file under tests/, side by side: a test file keeps a process to itself (some measure
# their process's threads, and hundreds of tests in one process slow each other). The
# library's unit tests, the command's and tests/closures_freed.rs are programs of their own.
# A run that fails prints its failures.
set -uo pipefail
cd "$(dirname "$0")/.."
args=(--release --features reference-tests ${CARGO_TEST_ARGS:-})
logs=$(mktemp -d)
trap 'rm -rf "$logs"' EXIT
# Every recovery image a test writes is read back and compared with the agent it was taken from
# (src/runtime/checkpoint.rs).
export AFFORDANCE_VERIFY_IMAGES=1
if ! cargo test "${args[@]}" --no-run --message-format=json > "$logs/build.json" 2> "$logs/build.err"; then
  cat "$logs/build.err" >&2
  exit 1
fi
jq -r 'select(.profile.test == true and .executable != null) | .executable' "$logs/build.json" > "$logs/programs"
suite=$(grep -E "/suite-[0-9a-f]+$" "$logs/programs")
start=$(date +%s)
# One run of a program: its log and its failure if any. `$1`: a name for the log; `$2`: the
# program; the rest: its arguments. Each test's time is printed (the test harness's
# `--report-time`, which a stable toolchain allows with RUSTC_BOOTSTRAP).
one() {
  RUSTC_BOOTSTRAP=1 "$2" -Z unstable-options --report-time "${@:3}" > "$logs/$1.log" 2>&1 || echo "$1" >> "$logs/failed"
}
# The test files, as the modules of the suite program.
mapfile -t modules < <("$suite" --list 2>/dev/null | sed -n 's/^\([a-z0-9_]*\)::.*: test$/\1/p' | sort -u)
# One process for the tests of some files. A file's tests are those whose names begin with its
# name: the filter `tools::` would also take `chat_tools::`, so files whose names end with a
# wanted file's name are skipped.
run_files() { # run_files LOG FILE... [-- extra arguments]
  local log=$1; shift
  local wanted=() extra=() skips=()
  while [ $# -gt 0 ] && [ "$1" != "--" ]; do wanted+=("$1"); shift; done
  [ $# -gt 0 ] && { shift; extra=("$@"); }
  for m in "${modules[@]}"; do
    for w in "${wanted[@]}"; do
      [[ "$m" != "$w" && "$m" == *"$w" && ! " ${wanted[*]} " == *" $m "* ]] && skips+=("--skip=$m::")
    done
  done
  one "$log" "$suite" "${wanted[@]/%/::}" "${skips[@]}" "${extra[@]}"
}
# Tests that measure time run after the rest, on a quiet machine: beside the others they measure
# the others' load. Two files of them (how quickly the runtime answers), and single tests: one
# that compares two timings, and the one that starts Docker containers. The doc tests run
# beside the programs.
quiet=(pause_latency reliability)
quiet_tests=(
  runtime::a_docker_machine_is_one_container_per_agent
  js_semantics::integer_keys_insert_in_linear_time
  # Two requests in a row use one connection. The HTTP library hands a finished connection
  # back to its pool from a background task; on a machine with no core to spare the second
  # request can start first.
  http_connector::ordinary_http_is_independent_of_mcp_and_reuses_then_closes_idle_connections
  # A run given less than a second of real time, and the start times of 400 agents compared.
  runtime::a_pause_freezes_the_wait_clock_and_mail_is_durable
  channel_scaling::starting_an_agent_does_not_slow_down_with_the_agents_already_running
)
(cargo test "${args[@]}" --doc > "$logs/doc.log" 2>&1 || echo doc >> "$logs/failed") &
for program in $(grep -v -E "/suite-[0-9a-f]+$" "$logs/programs"); do
  one "$(basename "${program%-*}")" "$program" &
done
for m in "${modules[@]}"; do
  [[ " ${quiet[*]} " == *" $m "* ]] && continue
  run_files "$m" "$m" -- "${quiet_tests[@]/#/--skip=}" &
done
wait
for m in "${quiet[@]}"; do run_files "$m" "$m" & done
one quiet_tests "$suite" --exact "${quiet_tests[@]}"
wait
if [ -s "$logs/failed" ]; then
  while read -r n; do
    echo "FAILED: $n"
    grep -E "^test .* FAILED|panicked at|^error" -A6 "$logs/$n.log" | head -60
  done < "$logs/failed"
  exit 1
fi
# The slowest tests, so a slow run says where its time went.
echo "slowest: $(grep -h -oE "^test .* ok <[0-9.]+s>" "$logs"/*.log | sed -E 's/^test (.*) \.\.\. ok <([0-9.]+)s>/\2 \1/' | sort -rn | head -"${SLOWEST:-4}" | awk '{printf "%s %.1fs  ", $2, $1}')"
grep -h "^test result" "$logs"/*.log | awk -v t="$(( $(date +%s) - start ))" \
  '{p+=$4; f+=$6; i+=$8} END {printf "test result: ok. %d passed; %d failed; %d ignored (%ds)\n", p, f, i, t}'
