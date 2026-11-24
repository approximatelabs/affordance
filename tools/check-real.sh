#!/usr/bin/env bash
# The command against real model servers: what a user does, checked on vLLM and SGLang serving a
# small model. Each check is a sentence a user would say. It is a heavy check, run by hand when
# wanted: it needs Docker, three GPUs and the model on disk, and takes several minutes to start
# the servers. No gate, script or CI job runs it.
#
#   MODEL=/path/to/Qwen3-4B-Instruct ./tools/check-real.sh
#
# MODEL: a directory with the weights and tokenizer.json. PIA: the installed pia launcher, to
# check it too (the launcher that sets the app directory; this script sets the binary).
set -uo pipefail
cd "$(dirname "$0")/.."
MODEL=${MODEL:-$HOME/rlvr-runs/models/qwen3-4b-instruct}
VLLM_IMAGE=${VLLM_IMAGE:-vllm/vllm-openai:v0.33.0}
SGLANG_IMAGE=${SGLANG_IMAGE:-lmsysorg/sglang:v0.5.23}
A=$PWD/target/release/affordance
WORK=$(mktemp -d)
failed=0; ONLY=${ONLY:-}
ok() { echo "ok    $1"; }
bad() { echo "FAIL  $1"; failed=$((failed + 1)); }

export AFFORDANCE_VERIFY_IMAGES=1   # every recovery image is read back and compared (checkpoint.rs)
cargo build --release --bin affordance 2>&1 | tail -1
# KEEP=1 leaves the servers running for the next run, which then starts in seconds.
up() { [ "$(curl -s -m 3 -o /dev/null -w '%{http_code}' "http://127.0.0.1:$1/v1/models")" = 200 ]; }
cleanup() { [ -n "${KEEP:-}" ] || docker rm -f real-vllm real-sglang real-noparser >/dev/null 2>&1; rm -rf "$WORK"; }
trap cleanup EXIT

run="-d --shm-size 16g -v $MODEL:/m:ro -e HF_HUB_OFFLINE=1"
if ! (up 8181 && up 8182 && up 8183); then
docker rm -f real-vllm real-sglang real-noparser >/dev/null 2>&1
docker run $run --name real-vllm --gpus device=0 -p 127.0.0.1:8181:8000 "$VLLM_IMAGE" \
  /m --served-model-name local --max-model-len 16384 --enable-auto-tool-choice --tool-call-parser hermes >/dev/null
docker run $run --name real-sglang --gpus device=1 -p 127.0.0.1:8182:8000 "$SGLANG_IMAGE" \
  sglang serve --model-path /m --served-model-name local --host 0.0.0.0 --port 8000 --context-length 16384 --tool-call-parser qwen >/dev/null
docker run $run --name real-noparser --gpus device=2 -p 127.0.0.1:8183:8000 "$SGLANG_IMAGE" \
  sglang serve --model-path /m --served-model-name local --host 0.0.0.0 --port 8000 --context-length 16384 >/dev/null
fi
for port in 8181 8182 8183; do
  for _ in $(seq 1 120); do up $port && break; sleep 5; done
  up $port || { echo "the server on port $port did not start"; exit 1; }
done

# The first agent, as `affordance init` writes it, in a directory with three files to count.
agent() { # agent NAME --set... : runs it; leaves its report in $WORK/NAME.out and its stderr in NAME.err
  local name=$1; shift
  rm -rf "$WORK/$name"; "$A" init "$WORK/$name" >/dev/null
  touch "$WORK/$name/one.txt" "$WORK/$name/two.txt" "$WORK/$name/three.txt"
  (cd "$WORK/$name" && timeout 180 "$A" run "$@" >"$WORK/$name.out" 2>"$WORK/$name.err")
}
# "The first agent runs a shell command and answers", on one binding. A small model sometimes
# answers in prose; it gets a second run.
answers() { # answers LABEL --set...
  local label=$1; shift
  for _ in 1 2; do
    agent a "$@"
    if grep -q '"answer"' "$WORK/a.out" && grep -q '"t":"tool"\|"t":"exec"\|"t":"call"' "$WORK/a/.affordance/first/trace.jsonl"; then
      ok "the first agent runs a command and answers: $label"; return
    fi
  done
  bad "the first agent runs a command and answers: $label"; tail -5 "$WORK/a.err"; grep -A3 '"stopped"' "$WORK/a.out" | head -5
  grep -o '"problems":\[[^]]*\]' "$WORK/a/.affordance/first/trace.jsonl" 2>/dev/null | sort | uniq -c
}

V=http://127.0.0.1:8181; S=http://127.0.0.1:8182
TOK=$MODEL/tokenizer.json
answers "vLLM, chat completions"            --set model.url=$V/v1
answers "vLLM, OpenAI Responses"            --set model.kind=responses --set model.url=$V/v1
answers "vLLM, Anthropic Messages"          --set model.kind=anthropic --set model.url=$V
answers "vLLM, token ids (completions)"     --set model.kind=completions --set model.url=$V --set model.model=null --set model.name=local --set model.template=qwen3 --set model.tokenizer=$TOK
answers "SGLang, chat completions"          --set model.url=$S/v1
answers "SGLang, token ids (generate)"      --set model.kind=sglang --set model.url=$S --set model.model=null --set model.template=qwen3 --set model.tokenizer=$TOK

# "If the server does not parse tool calls, the run says so."
agent n --set model.url=http://127.0.0.1:8183/v1 --set 'agents[0].max_turns=2'
if grep -q "wrote a tool call as text" "$WORK/n.err"; then ok "a server without its tool-call parser is reported"; else bad "a server without its tool-call parser is reported"; tail -3 "$WORK/n.err"; fi

# "If no server is running, the run says so and stops when its retry time has passed."
start=$SECONDS
agent d --set model.url=http://127.0.0.1:9/v1 --set model.retry.for_s=5
if grep -q "the model call failed" "$WORK/d.err" && grep -q "not tried again" "$WORK/d.err" && [ $((SECONDS - start)) -lt 60 ]; then
  ok "an unreachable server is reported, and the run stops after its retry time"
else bad "an unreachable server is reported, and the run stops after its retry time"; tail -3 "$WORK/d.err"; fi

# "A run continues after it is restored."
agent r --set model.url=$V/v1
if (cd "$WORK/r" && timeout 60 "$A" run --restore 2>/dev/null | grep -q '"complete": true'); then ok "a finished run restores from its journal"; else bad "a finished run restores from its journal"; fi

# "affordance serve answers an OpenAI client."
# The demo is copied out of the checkout first: it must run with the library the command bundles.
cp -r views/demo/memory_server "$WORK/serve"
(cd "$WORK/serve" && timeout 120 "$A" serve --listen 127.0.0.1:8184 --set model.url=$V/v1 >/dev/null 2>"$WORK/serve.err" &)
sleep 3
reply=$(curl -s -m 60 http://127.0.0.1:8184/v1/chat/completions -H 'content-type: application/json' \
  -d '{"model":"x","messages":[{"role":"user","content":"Reply with the single word: ready"}]}')
if echo "$reply" | grep -qi '"content":"[^"]*ready'; then ok "serve answers an OpenAI client"; else bad "serve answers an OpenAI client"; echo "$reply" | head -c 300; fi

# "affordance host answers a frontend's prompt."
cp -r views/demo/host "$WORK/host"
out=$( (echo '{"type":"event","name":"user","payload":{"text":"Reply with the single word: ready"}}'; sleep 20; echo '{"type":"close"}') \
  | (cd "$WORK/host" && timeout 60 "$A" host --set model.url=$V/v1 2>/dev/null) )
if echo "$out" | grep -qi '"reply":{"text":"[^"]*ready'; then ok "host answers a frontend's prompt"; else bad "host answers a frontend's prompt"; echo "$out" | tail -3 | cut -c1-300; fi

# "pia answers using a tool" (pi with Affordance as its agent core), when its launcher is given.
if [ -n "${PIA:-}" ]; then
  mkdir -p "$WORK/pi" "$WORK/piwork"; echo a >"$WORK/piwork/a.txt"; echo b >"$WORK/piwork/b.txt"
  echo "{\"providers\":{\"local\":{\"baseUrl\":\"$S/v1\",\"api\":\"openai-completions\",\"apiKey\":\"none\",\"models\":[{\"id\":\"local\",\"contextWindow\":16384,\"maxTokens\":2048}]}}}" >"$WORK/pi/models.json"
  echo '{"defaultProvider":"local","defaultModel":"local"}' >"$WORK/pi/settings.json"
  out=$(cd "$WORK/piwork" && PI_CODING_AGENT_DIR="$WORK/pi" PI_AFFORDANCE_BIN="$A" timeout 120 "$PIA" --provider local --model local \
    -p "How many files are in this directory? Use a tool to check." </dev/null 2>/dev/null)
  if echo "$out" | grep -q "a.txt"; then ok "pia answers using a tool"; else bad "pia answers using a tool"; echo "$out" | tail -c 300; fi
fi

echo
if [ $failed -eq 0 ]; then echo "all real-server checks passed"; else echo "$failed real-server checks FAILED"; exit 1; fi
