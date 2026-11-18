#!/usr/bin/env bash
# A picture, a video and a sound clip reach a real model through its token ids: for one model
# served by SGLang, an agent is shown a blue picture, a green video and a clip of the word
# "banana", and must name each. This checks the placeholder token recorded for the model
# (`agent.media` in views/index.json): the server puts the file where that token is. It also
# asks the server which prompt tokens it processed, and they must be the ones sent. It is a
# heavy check, run by hand: it needs Docker, a GPU and the model's weights. No gate, script or
# CI job runs it.
#
#   MODEL=/path/to/snapshot TEMPLATE=Qwen/Qwen3.5-0.8B KINDS="image video" ./tools/check-media.sh
#
# MODEL: a directory with the weights and tokenizer.json (a Hugging Face snapshot directory
# works; HUB is the cache it links into). TEMPLATE: the model's registered name. KINDS: which
# of image, video and audio to check (default: the kinds the model's row lists). CONTEXT: a
# context length for the server, when the model's own is not wanted.
set -uo pipefail
cd "$(dirname "$0")/.."
: "${MODEL:?set MODEL to the model directory}" "${TEMPLATE:?set TEMPLATE to the registered name}"
SGLANG_IMAGE=${SGLANG_IMAGE:-lmsysorg/sglang:v0.5.23}
# The directory the server is given: the Hugging Face cache a snapshot is in (its files link
# into the cache's blobs), or the model's own directory.
case "$MODEL" in */models--*/snapshots/*) HUB=${HUB:-${MODEL%%/models--*}};; *) HUB=${HUB:-$MODEL};; esac
GPU=${GPU:-0}; PORT=${PORT:-8192}
A=$PWD/target/release/affordance
# The tokenizer: the model's own file, or the one built for a model that ships a Python class.
TOKENIZER=$MODEL/tokenizer.json; [ -f "$TOKENIZER" ] || TOKENIZER=$PWD/ref/tokenizers/${TEMPLATE/\//__}.json
KINDS=${KINDS:-$(grep -A40 "\"${TEMPLATE/\//__}\": {" views/index.json | grep -m1 -A4 '"media"' | grep -o '"\(image\|video\|audio\)"' | tr -d '"' | tr '\n' ' ')}
WORK=$(mktemp -d); failed=0
cleanup() { [ -n "${KEEP:-}" ] || docker rm -f media-sglang-$PORT >/dev/null 2>&1; rm -rf "$WORK"; }
trap cleanup EXIT
up() { [ "$(curl -s -m 3 -o /dev/null -w '%{http_code}' "http://127.0.0.1:$PORT/v1/models")" = 200 ]; }
if ! up; then
  docker rm -f media-sglang-$PORT >/dev/null 2>&1
  docker run -d --shm-size 16g -v "$HUB:$HUB:ro" -e HF_HUB_OFFLINE=1 --name media-sglang-$PORT --gpus "device=$GPU" -p "127.0.0.1:$PORT:8000" "$SGLANG_IMAGE" \
    sglang serve --model-path "$MODEL" --served-model-name local --host 0.0.0.0 --port 8000 ${CONTEXT:+--context-length $CONTEXT} ${SGLANG_ARGS:-} >/dev/null
  for _ in $(seq 1 120); do up && break; docker ps -q -f name=media-sglang-$PORT | grep -q . || break; sleep 5; done
fi
if ! up; then echo "FAIL  SGLang did not start with $TEMPLATE:"; docker logs media-sglang-$PORT 2>&1 | grep -v "^$" | tail -4 | cut -c1-240; exit 1; fi
declare -A ASK=([frames]="These are frames of a video. What single colour fills them? Give it with the answer tool." [trimmed]="What is the secret word spoken in this audio? Give it with the answer tool." [image]="What single colour fills this picture? Give it with the answer tool." [sized]="What single colour fills this picture? Give it with the answer tool." [video]="What single colour fills this video? Give it with the answer tool." [audio]="What is the secret word spoken in this audio? Give it with the answer tool.")
declare -A WANT=([image]=blue [sized]=blue [frames]=green [trimmed]=banana [video]=green [audio]=banana)
# A picture is checked twice: as it is, and under a style rule that gives it 256 tokens
# (`sized`). A sound clip is checked as it is and trimmed by a rule to 60 tokens (`trimmed`,
# which may cut the word off: only its count is checked). In each the server must count the
# prompt as layout did.
# A model that takes pictures is also shown the video as its frames (`frames`: four pictures, each within 280 tokens,
# cut from the video with ffmpeg), which needs no video support in the model.
for kind in $(echo $KINDS | sed 's/image/image sized frames/; s/audio/audio trimmed/'); do
  from=$kind; [ $kind = sized ] && from=image; [ $kind = trimmed ] && from=audio
  if [ $kind = frames ] && ! command -v ffmpeg >/dev/null; then echo "skip  frames: no ffmpeg"; continue; fi
  mkdir -p "$WORK/$kind" && cp tools/check-media/$from/app.tsx tools/check-media/affordance.yaml "$WORK/$kind/"
  if [ $kind = sized ]; then
    echo 'img { min-tokens: 256tok; max-tokens: 256tok; }' > "$WORK/$kind/sized.tss"
    sed -i '1i import "./sized.tss";' "$WORK/$kind/app.tsx"
  fi
  if [ $kind = frames ]; then
    cp tools/check-media/frames/frames.tss "$WORK/$kind/"
    grep -o 'data:video/mp4;base64,[A-Za-z0-9+/=]*' tools/check-media/video/app.tsx | head -1 | cut -d, -f2 | base64 -d > "$WORK/$kind/clip.mp4"
    ffmpeg -v error -i "$WORK/$kind/clip.mp4" "$WORK/$kind/shot%02d.png"
    shots=$(for f in "$WORK/$kind"/shot*.png; do printf '"data:image/png;base64,%s", ' "$(base64 -w0 "$f")"; done)
    echo "      shots: [${shots%, }]" >> "$WORK/$kind/affordance.yaml"
  fi
  if [ $kind = trimmed ]; then
    echo 'audio { max-tokens: 60tok; }' > "$WORK/$kind/sized.tss"
    sed -i '1i import "./sized.tss";' "$WORK/$kind/app.tsx"
  fi
  got=$(cd "$WORK/$kind" && AFFORDANCE_CAPTURE_DIR="$WORK/$kind.capture" timeout 120 "$A" run --set model.url=http://127.0.0.1:$PORT --set "model.template=$TEMPLATE" --set "model.tokenizer=$TOKENIZER" \
    --set "agents[0].props.task=${ASK[$kind]}" 2>"$WORK/$kind.err" | grep -o '"answer": *"[^"]*"' | head -1)
  # The server reports the prompt tokens it processed: they must be the ones sent.
  tokens=$(${PYTHON:-python3} tools/check-media/same_tokens.py "$WORK/$kind.capture" "http://127.0.0.1:$PORT" 2>&1 | tail -1)
  case "$tokens" in same*) ;; *) echo "FAIL  $TEMPLATE, $kind: the server did not use the tokens sent ($tokens)"; failed=$((failed + 1));; esac
  counted=$(${PYTHON:-python3} tools/check-media/counts.py "$WORK/$kind" 2>&1 | tail -1)
  case "$counted" in same*) ;; *) echo "FAIL  $TEMPLATE, $kind: the server did not count the prompt as layout did ($counted)"; failed=$((failed + 1));; esac
  # A file the model has no size rule for is not laid out at a guess: its alt text stood in.
  if grep -q "was not shown to the model" "$WORK/$kind.err"; then
    echo "none  $TEMPLATE, $kind: $(grep -m1 -o "not shown to the model.*" "$WORK/$kind.err" | cut -c1-140)"; continue
  fi
  if [ $kind = trimmed ]; then case "$counted" in same*) echo "ok    $TEMPLATE counts a trimmed clip as layout does ($counted)";; esac
  elif echo "$got" | grep -qi "${WANT[$kind]}"; then echo "ok    $TEMPLATE names the ${WANT[$kind]} $kind ($counted)"
  else echo "FAIL  $TEMPLATE, $kind: wanted ${WANT[$kind]}, got ${got:-no answer}"; tail -2 "$WORK/$kind.err" | cut -c1-240
    docker logs media-sglang-$PORT 2>&1 | grep -i -E "error|exception|traceback" -A3 | tail -8 | cut -c1-240; failed=$((failed + 1)); fi
done
exit $failed
