"""Write into views/index.json what an agent needs to read each model's replies, as measured
from the model's own format by tools/protocol_survey.rs:

    cargo run --release --features reference-tests --example protocol_survey > target/protocol_survey.json
    .venv/bin/python tools/measure_agents.py

Each model's entry gets an `agent` row: `stop` (the tokens that end a reply), `in_think`
(the prompt ends inside open thinking), `syntax` (the reply parser that reads back the tool
calls the format renders, or null), `alternate` (user and assistant turns must alternate),
`system_role` (false: the format has no system turn), `tools` (false: the format refuses tools), `parts` (the format reads content as a list of parts), `media` (the token the format writes where an image, an audio clip or a video goes) and `special` (the tokenizer's named
tokens, which the template reads by name). A hand-written view's syntax, stop and in_think
stay in views/reply_protocols.json, by view; its row here has the rest.
"""
import json, os, sys
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
os.chdir(ROOT)
index = json.load(open("views/index.json"))
# A survey of some models (tools/add_model.py passes one) leaves the others' rows as they are.
survey = json.load(open(sys.argv[1] if len(sys.argv) > 1 else "target/protocol_survey.json"))
partial = len(sys.argv) > 1
protocols = json.load(open("views/reply_protocols.json"))
written = unreadable = 0
for model, entry in index.items():
    if model.startswith("_"): continue
    s = survey.get(model)
    if partial and s is None: continue
    entry.pop("agent", None)
    if not s or "error" in s:
        unreadable += 1
        continue
    row = {}
    if f"{entry['view']}::{entry['entry']}" not in protocols:
        row["stop"] = s.get("stop", [])
        row["in_think"] = bool(s.get("in_think"))
        row["syntax"] = (s.get("parsers") or [None])[0]
        if s.get("id_name"): row["id_name"] = s["id_name"]
    if "tools_error" in s: row["tools"] = False
    if s.get("content_parts"): row["parts"] = True
    if s.get("alternate"): row["alternate"] = True
    if s.get("system_role") is False: row["system_role"] = False
    if s.get("media"): row["media"] = s["media"]
    if s.get("special"): row["special"] = s["special"]
    # A model's picture metrics come from its processor (tools/measure_media.py), not from
    # the survey: they stay.
    kept = (entry.get("agent") or {}).get("media_metrics")
    if kept and (row.get("media") or {}).get("image"): row["media_metrics"] = kept
    entry["agent"] = row
    written += 1
open("views/index.json", "w").write(json.dumps(index, indent=1) + "\n")
print(f"{written} models have an agent row; {unreadable} could not be surveyed")
