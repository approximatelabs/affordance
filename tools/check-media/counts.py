"""Did the server count the prompt as layout did?

    counts.py RUN_DIR [EXPECTED_PICTURE_TOKENS]

Reads the run's trace (.affordance/*/trace.jsonl). For each model call, the prompt tokens
Affordance laid out must be the prompt tokens the server reports, pictures included: a picture
is laid out at the number of tokens its model's processor gives it. Prints one line and exits
1 when a call differs. Standard library only.
"""
import glob, json, sys

calls = []
for path in glob.glob(f"{sys.argv[1]}/.affordance/*/trace.jsonl"):
    for line in open(path):
        event = json.loads(line)
        if event.get("t") == "generate" and isinstance(event.get("model_meta"), dict) and not event.get("aborted"):
            calls.append((event["prompt_tokens"], event["model_meta"].get("prompt_tokens")))
if not calls: sys.exit("no model call in the trace")
wrong = [(ours, theirs) for ours, theirs in calls if ours != theirs]
print(f"{'same' if not wrong else 'DIFFERENT'}: {len(calls)} calls, laid out {[c[0] for c in calls]}, the server counted {[c[1] for c in calls]}")
sys.exit(1 if wrong else 0)
