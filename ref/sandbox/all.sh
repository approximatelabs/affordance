#!/usr/bin/env bash
# The official references for every model whose tokenizer is a Python class in its repository:
# run each class in the sandbox (run.sh), then build the standard tokenizer from the model's
# data file and keep it only if it gives the class's ids on every text (build_tokenizer.py).
# Needs the repositories under ref/models (tokenizer data files and *.py) and Docker.
set -euo pipefail
cd "$(dirname "$0")/../.."
models="moonshotai__Kimi-K2-Instruct moonshotai__Kimi-K2-Thinking moonshotai__Kimi-Linear-48B-A3B-Instruct
moonshotai__Kimi-K2-Base moonshotai__Kimi-VL-A3B-Instruct moonshotai__Moonlight-16B-A3B-Instruct
moonshotai__Kimi-K2.5 moonshotai__Kimi-K2.6 moonshotai__Kimi-K2.7-Code
internlm__internlm3-8b-instruct internlm__Intern-S1 internlm__Intern-S1-Pro
XingChen-AGI__Xing4.0-29B-Intent microsoft__Phi-3-small-8k-instruct baidu__ERNIE-4.5-VL-28B-A3B-PT"
for m in $models; do
  echo "$m: $(ref/sandbox/run.sh "$m" 2>&1 | grep '^done' || echo 'the class did not load')"
done
# shellcheck disable=SC2086
.venv/bin/python ref/sandbox/build_tokenizer.py $models
