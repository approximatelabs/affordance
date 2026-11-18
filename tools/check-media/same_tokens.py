"""Did the server feed the model the tokens Affordance sent?

    same_tokens.py CAPTURE_DIR URL

Replays the first request Affordance sent (recorded with AFFORDANCE_CAPTURE_DIR) and asks
SGLang for the prompt tokens it processed (`return_logprob` from position 0). They must be
the ids sent, with each media placeholder repeated as many times as the server expanded it.
Prints one line and exits 1 when they differ. Standard library only.
"""
import glob, json, sys, urllib.request

capture, url = sys.argv[1], sys.argv[2]
first = sorted(glob.glob(f"{capture}/*/*-0-0.request.json"))
if not first: sys.exit("no captured request")
body = json.load(open(first[0]))["data"]["body"]
body.pop("rid", None)
body.update(stream=False, return_logprob=True, logprob_start_len=0, sampling_params={"max_new_tokens": 1, "temperature": 0})
request = urllib.request.Request(f"{url}/generate", json.dumps(body).encode(), {"content-type": "application/json"})
try:
    out = json.loads(urllib.request.urlopen(request, timeout=120).read())
except Exception as error:
    print(f"the server refused the replayed request: {error}"); sys.exit(1)
used = [t[1] for t in out["meta_info"]["input_token_logprobs"]]
sent = body["input_ids"]
# Every difference must be one placeholder becoming the file's tokens: a run of one repeated
# token added beside a sent token of the same id, or one sent token replaced by two or more.
import difflib
problem = None
for op, a0, a1, b0, b1 in difflib.SequenceMatcher(None, sent, used, autojunk=False).get_opcodes():
    if op == "equal": continue
    run = used[b0:b1]
    one_token = len(set(run)) == 1
    if op == "insert" and one_token and run[0] in (sent[a0 - 1] if a0 else None, sent[a0] if a0 < len(sent) else None): continue
    # (A video's placeholder becomes its frames, each with its own markers.)
    if op == "replace" and a1 - a0 == 1 and len(run) >= 2: continue
    problem = f"sent[{a0}:{a1}] = {sent[a0:a1][:6]}, used[{b0}:{b1}] = {run[:6]}"
    break
same = problem is None
print(f"{'same' if same else 'DIFFERENT'}: sent {len(sent)} tokens, the server used {len(used)}" + ("" if same else f"; first difference: {problem}"))
sys.exit(0 if same else 1)
