"""Official references for the converted models (tag `converted` in ref/models.txt), on every
case set, several models at a time. The repo's tokenizer loads through AutoTokenizer, or, where
that fails, straight from its tokenizer.json; the template is the repo's own. No code from a
model repository is run.

    .venv/bin/python ref/render_converted.py [SET...] [ORG__MODEL...]     (default: every set but dev, every model)
"""
import copy, json, os, sys, warnings
from multiprocessing import Pool

warnings.filterwarnings("ignore")
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "..", "tools"))
ONLY = [a for a in sys.argv[1:] if "__" in a]
SETS = [a for a in sys.argv[1:] if "__" not in a] or ["heldout1", "heldout2", "heldout3", "heldout_a2", "agent_loops"]


def one(name):
    import render_ref  # pins the clock and the tokenizer backend
    import adopt_templates as A
    from transformers import AutoTokenizer, PreTrainedTokenizerFast
    os.chdir(os.path.join(HERE, ".."))
    d = os.path.join(HERE, "models", name)
    text_of = lambda v: v if isinstance(v, str) else (v or {}).get("content")
    try:
        try:
            tok = AutoTokenizer.from_pretrained(d)
        except Exception:
            cfg = json.load(open(os.path.join(d, "tokenizer_config.json")))
            special = {k: text_of(cfg[k]) for k in ["bos_token", "eos_token", "unk_token", "pad_token", "sep_token", "cls_token", "mask_token"] if cfg.get(k)}
            extra = [text_of(x) for x in cfg.get("additional_special_tokens", []) or []]
            tok = PreTrainedTokenizerFast(tokenizer_file=os.path.join(d, "tokenizer.json"), additional_special_tokens=extra, **special)
        if tok.chat_template is None:
            t = A.template(name)
            tok.chat_template = t if isinstance(t, str) else [{"name": k, "template": v} for k, v in t.items()]
        done = []
        for s in SETS:
            cases = json.load(open(os.path.join(HERE, f"cases_{s}.json")))
            res = {}
            for c in cases:
                try:
                    kw = dict(c["kwargs"])
                    text = tok.apply_chat_template(copy.deepcopy(c["messages"]), tools=copy.deepcopy(c["tools"]), tokenize=False, **kw)
                    enc = tok.apply_chat_template(copy.deepcopy(c["messages"]), tools=copy.deepcopy(c["tools"]), tokenize=True, **kw)
                    ids = enc["input_ids"] if hasattr(enc, "keys") else enc
                    res[c["id"]] = {"ids": list(ids), "text": text}
                except Exception as e:
                    res[c["id"]] = {"error": f"{type(e).__name__}: {str(e)[:300]}"}
            os.makedirs(os.path.join(HERE, "out", s), exist_ok=True)
            json.dump(res, open(os.path.join(HERE, "out", s, name + ".json"), "w"), ensure_ascii=False)
            done.append(f"{s} {sum('error' in v for v in res.values())}/{len(res)} errors")
        return name, "; ".join(done)
    except Exception as e:
        return name, f"FAILED {type(e).__name__}: {str(e)[:120]}"


if __name__ == "__main__":
    names = [l.split("|")[1].strip().replace("/", "__") for l in open(os.path.join(HERE, "models.txt")) if l.startswith("converted")]
    # Models whose references come another way: the repository's own tokenizer class run in the
    # sandbox (ref/sandbox), or a tokenizer exported to give one answer (ref/render_exported.py).
    exported = ["nvidia__NVIDIA-Nemotron-Parse-2.0", "nvidia__Nemotron-3.5-Parse"]
    names = [n for n in names if n not in exported and not os.path.isdir(os.path.join(HERE, "sandbox", "out", n))]
    if ONLY: names = [n for n in names if n in ONLY]
    failed = 0
    with Pool(32) as pool:
        for name, status in pool.imap_unordered(one, names):
            if status.startswith("FAILED"):
                failed += 1
                print(name, status, flush=True)
    print(len(names) - failed, "of", len(names), "models rendered on", SETS)
