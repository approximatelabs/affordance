#!/usr/bin/env bash
# Run a model repository's own tokenizer code where it can do no harm, to get the official
# output for models whose tokenizer is a Python class and not a tokenizer.json:
#
#   ref/sandbox/run.sh ORG__MODEL        (reads ref/models/ORG__MODEL, writes ref/sandbox/out/ORG__MODEL)
#
# The container has no network, a read-only file system, no capabilities and an unprivileged
# user. It sees the model's files, the case sets and this folder's script, read-only, and one
# output folder. The Python it runs is this checkout's .venv (mounted read-only).
set -euo pipefail
cd "$(dirname "$0")/../.."
model=$1
out=$PWD/ref/sandbox/out/$model
mkdir -p "$out" ref/sandbox/cases
cp ref/cases_*.json ref/sandbox/cases/
py=$(readlink -f .venv/bin/python)
uvpy=$HOME/.local/share/uv/python
docker run --rm --network none --read-only --cap-drop ALL --security-opt no-new-privileges \
  --user "$(id -u):$(id -g)" --tmpfs /tmp:rw,size=2g --memory 8g --pids-limit 256 \
  -e HOME=/tmp -e TIKTOKEN_CACHE_DIR=/tmp/tiktoken_cache -v "$PWD/ref/sandbox/tiktoken_cache:/tiktoken_seed:ro" -e HF_HOME=/tmp/hf -e HF_HUB_OFFLINE=1 -e TRANSFORMERS_OFFLINE=1 -e PYTHONDONTWRITEBYTECODE=1 \
  -v "$uvpy:$uvpy:ro" -v "$PWD/.venv:$PWD/.venv:ro" \
  -v "$PWD/ref/models/$model:/model:ro" -v "$PWD/ref/sandbox/cases:/cases:ro" \
  -v "$PWD/ref/sandbox/official.py:/official.py:ro" -v "$out:/out" \
  ubuntu:24.04 "$PWD/.venv/bin/python" /official.py 2>&1
