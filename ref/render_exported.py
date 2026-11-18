"""Official references for models whose tokenizer does not give the same ids on every load.
Several of Nemotron Parse's added tokens normalise (NFKC) to the same text: 14 Hebrew letters
each have a presentation form that becomes the letter, and Nemotron-3.5-Parse lists a few tokens
twice under two ids. Which of the two ids a letter gets is
decided by the order of a hash table that the tokenizers library seeds afresh on each load, so
the same text gives different ids from one Python process to the next, and the saved file does
not reload to either choice reliably.

The exported file here keeps, for each such text, the token that is written as that text (the
letter itself) and drops the others, so it gives one answer. The references are that file's
ids, after checking that they equal the live tokenizer's on every rendered text once each
dropped id is read as the kept one.

    .venv/bin/python ref/render_exported.py [ORG__MODEL...]
"""
import copy, json, os, sys, warnings
warnings.filterwarnings("ignore")
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE); sys.path.insert(0, os.path.join(HERE, "..", "tools"))
os.chdir(os.path.join(HERE, ".."))
import render_ref  # pins the clock and the tokenizer backend
import adopt_templates as A
from transformers import AutoTokenizer
from tokenizers import Tokenizer

# The models this script is for; ref/render_converted.py leaves them alone.
MODELS = ["nvidia__NVIDIA-Nemotron-Parse-2.0", "nvidia__Nemotron-3.5-Parse"]
SETS = ["dev", "heldout1", "heldout2", "heldout3", "heldout_a2", "agent_loops"]


def export(tok):
    live = tok.backend_tokenizer
    doc = json.loads(live.to_str())
    doc["truncation"] = None; doc["padding"] = None
    groups = {}
    for a in doc["added_tokens"]:
        key = live.normalizer.normalize_str(a["content"]) if a["normalized"] and live.normalizer else a["content"]
        groups.setdefault(key, []).append(a)
    same = {}  # dropped id -> kept id
    for key, group in groups.items():
        # (Two tokens can also have the very same text; the lower id is kept.)
        kept = sorted((a for a in group if a["content"] == key), key=lambda a: a["id"])[:1]
        if len(group) < 2 or not kept: continue
        for a in group:
            if a is not kept[0]: same[a["id"]] = kept[0]["id"]
    # An added token that the vocabulary does not name is numbered by its place in the list when
    # the file loads, so the vocabulary names every one: the ids stay as they are.
    for a in doc["added_tokens"]: doc["model"]["vocab"].setdefault(a["content"], a["id"])
    doc["added_tokens"] = [a for a in doc["added_tokens"] if a["id"] not in same]
    return Tokenizer.from_str(json.dumps(doc)), same


for name in (sys.argv[1:] or MODELS) if __name__ == "__main__" else []:
    d = "ref/models/" + name
    tok = AutoTokenizer.from_pretrained(d, trust_remote_code=False)
    if tok.chat_template is None: tok.chat_template = A.template(name)
    hf, dropped = export(tok)
    out, differ, total = {}, 0, 0
    for s in SETS:
        res = {}
        for c in json.load(open(f"ref/cases_{s}.json")):
            try:
                kw = dict(c["kwargs"])
                text = tok.apply_chat_template(copy.deepcopy(c["messages"]), tools=copy.deepcopy(c["tools"]), tokenize=False, **kw)
                enc = tok.apply_chat_template(copy.deepcopy(c["messages"]), tools=copy.deepcopy(c["tools"]), tokenize=True, **kw)
                ids = list(enc["input_ids"] if hasattr(enc, "keys") else enc)
                ours = hf.encode(text, add_special_tokens=False).ids
                res[c["id"]] = {"ids": ours, "text": text}
                total += 1
                want = [dropped.get(i, i) for i in ids]
                if ours != want and not differ:
                    k = next((j for j in range(min(len(ours), len(want))) if ours[j] != want[j]), min(len(ours), len(want)))
                    print("   first difference:", c["id"], len(ours), len(want), "live", ids[max(0, k - 2):k + 3], "export", ours[max(0, k - 2):k + 3], [hf.id_to_token(i) for i in ours[max(0, k - 2):k + 3]], [tok.convert_ids_to_tokens(i) for i in ids[max(0, k - 2):k + 3]], dropped)
                differ += ours != want
            except Exception as e:
                res[c["id"]] = {"error": f"{type(e).__name__}: {str(e)[:300]}"}
        out[s] = res
    print(f"{name}: {len(dropped)} added tokens dropped; the export differs from the live tokenizer on {differ} of {total} rendered texts")
    if differ: continue
    hf.save(f"ref/tokenizers/{name}.json")
    json.dump(tok.special_tokens_map, open(f"ref/tokenizers/{name}.special.json", "w"), ensure_ascii=False)
    for s, res in out.items():
        os.makedirs(f"ref/out/{s}", exist_ok=True)
        json.dump(res, open(f"ref/out/{s}/{name}.json", "w"), ensure_ascii=False)
