"""Reference outputs: official Jinja template + official tokenizer (HF transformers
apply_chat_template, tokenize=True). Writes ref/out/<cases>/<model>.json:
  {case_id: {"ids": [...], "text": str} | {"error": str}}
Also checks that the raw `tokenizers` tokenizer.json gives the same ids on the
rendered text (so the Rust side can load tokenizer.json directly).

The clock is pinned (strftime_now) so templates that print today's date are
deterministic. The view-language renderer gets the same pinned clock.
"""
import copy, datetime, json, os, sys, warnings

warnings.filterwarnings("ignore")
# Pin the reference path to each repo's shipped template + tokenizer. When mistral-common is
# installed, transformers 5.18 silently swaps Mistral repos to its MistralCommonBackend (a
# different encoder). The dev references were made without it; keep it off for every set.
import transformers.utils.import_utils as _iu
_iu.is_mistral_common_available = lambda: False
import transformers.utils.chat_template_utils as ctu
from transformers import AutoTokenizer
from tokenizers import Tokenizer

FIXED_NOW = datetime.datetime(2026, 10, 5, 12, 0, 0)


class _FixedDT(datetime.datetime):
    @classmethod
    def now(cls, tz=None):
        return FIXED_NOW


ctu.datetime = _FixedDT

HERE = os.path.dirname(os.path.abspath(__file__))
PYENC = {"deepseek-ai__DeepSeek-V3.2", "deepseek-ai__DeepSeek-V4-Flash", "deepseek-ai__DeepSeek-V4-Pro"}


def model_dirs():
    for line in open(os.path.join(HERE, "models.txt")):
        line = line.strip()
        if line and not line.startswith("#"):
            fam, repo = [x.strip() for x in line.split("|")]
            yield fam, repo, os.path.join(HERE, "models", repo.replace("/", "__"))


def render_model(d, cases):
    tok = AutoTokenizer.from_pretrained(d)
    # Export the exact backend tokenizer transformers built (it can differ from the
    # repo's tokenizer.json, e.g. legacy SentencePiece flags). The Rust renderer loads this.
    tdir = os.path.join(HERE, "tokenizers"); os.makedirs(tdir, exist_ok=True)
    hf_json = tok.backend_tokenizer.to_str()
    open(os.path.join(tdir, os.path.basename(d) + ".json"), "w").write(hf_json)
    hf = Tokenizer.from_str(hf_json)
    # The special-token strings HF passes into every template (bos_token, eos_token, ...).
    json.dump(tok.special_tokens_map, open(os.path.join(tdir, os.path.basename(d) + ".special.json"), "w"), ensure_ascii=False)
    raw = Tokenizer.from_file(os.path.join(d, "tokenizer.json")) if os.path.exists(os.path.join(d, "tokenizer.json")) else None
    res, rawmis = {}, []
    for c in cases:
        kw = dict(c["kwargs"])
        try:
            text = tok.apply_chat_template(copy.deepcopy(c["messages"]), tools=copy.deepcopy(c["tools"]), tokenize=False, **kw)
            enc = tok.apply_chat_template(copy.deepcopy(c["messages"]), tools=copy.deepcopy(c["tools"]), tokenize=True, **kw)
            ids = enc["input_ids"] if hasattr(enc, "keys") else enc
            res[c["id"]] = {"ids": list(ids), "text": text}
            assert hf.encode(text, add_special_tokens=False).ids == list(ids), "exported backend tokenizer differs"
            if raw is not None:
                rids = raw.encode(text, add_special_tokens=False).ids
                if rids != list(ids):
                    res[c["id"]]["ids_raw"] = rids
                    rawmis.append(c["id"])
        except Exception as e:
            res[c["id"]] = {"error": f"{type(e).__name__}: {str(e)[:300]}"}
    return res, rawmis


def main():
    set_name = sys.argv[1] if len(sys.argv) > 1 else "dev"
    only = sys.argv[2:]
    cases = json.load(open(os.path.join(HERE, f"cases_{set_name}.json")))
    outd = os.path.join(HERE, "out", set_name)
    os.makedirs(outd, exist_ok=True)
    for fam, repo, d in model_dirs():
        name = os.path.basename(d)
        if only and name not in only:
            continue
        if name in PYENC:
            continue  # reference comes from the repo's Python encoder (render_ref_pyenc.py)
        if not os.path.exists(os.path.join(d, "tokenizer_config.json")):
            print(f"SKIP {repo}: no HF tokenizer (see non-HF reference)")
            continue
        try:
            res, rawmis = render_model(d, cases)
        except Exception as e:
            print(f"FAIL {repo}: {type(e).__name__}: {str(e)[:200]}")
            continue
        json.dump(res, open(os.path.join(outd, name + ".json"), "w"), ensure_ascii=False)
        nerr = sum("error" in v for v in res.values())
        print(f"OK {repo}: {len(res)} cases, {nerr} template errors, repo tokenizer.json differs from HF on {len(rawmis)} cases")


if __name__ == "__main__":
    main()
