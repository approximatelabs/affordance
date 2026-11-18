"""Baseline: Hugging Face apply_chat_template(tokenize=True) = Jinja2 render + fast tokenizer.
Single thread. For each model, the cases of the given sets that render without error, each timed
REPS times (best-of and median per case). Writes bench/results_hf.json.

Also times the two halves separately: Jinja render only (tokenize=False) and tokenize only.
"""
import copy, json, os, sys, time, warnings
warnings.filterwarnings("ignore")
import transformers.utils.import_utils as _iu
_iu.is_mistral_common_available = lambda: False
import transformers.utils.chat_template_utils as ctu
import datetime
from transformers import AutoTokenizer

FIXED = datetime.datetime(2026, 10, 5, 12)
class _DT(datetime.datetime):
    @classmethod
    def now(cls, tz=None):
        return FIXED
ctu.datetime = _DT

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REPS = int(os.environ.get("REPS", "5"))


def main():
    sets = sys.argv[1].split(",") if len(sys.argv) > 1 else ["dev", "heldout3"]
    models = sys.argv[2].split(",") if len(sys.argv) > 2 else None
    idx = json.load(open(f"{ROOT}/views/index.json"))
    out = {}
    for name in idx:
        if name.startswith("_") or (models and name not in models):
            continue
        d = f"{ROOT}/ref/models/{name}"
        if not os.path.exists(f"{d}/tokenizer_config.json") or name in ("deepseek-ai__DeepSeek-V3.2", "deepseek-ai__DeepSeek-V4-Flash", "deepseek-ai__DeepSeek-V4-Pro"):
            continue  # no Jinja template: not a Jinja baseline
        tok = AutoTokenizer.from_pretrained(d)
        res = {}
        for s in sets:
            cases = json.load(open(f"{ROOT}/ref/cases_{s}.json"))
            ref = json.load(open(f"{ROOT}/ref/out/{s}/{name}.json"))
            for c in cases:
                if "ids" not in ref[c["id"]]:
                    continue
                msgs, tools, kw = c["messages"], c["tools"], c["kwargs"]
                full, jin, tk = [], [], []
                for _ in range(REPS):
                    m, t = copy.deepcopy(msgs), copy.deepcopy(tools)   # outside the timed region
                    t0 = time.perf_counter_ns()
                    ids = tok.apply_chat_template(m, tools=t, tokenize=True, **kw)
                    t1 = time.perf_counter_ns()
                    text = tok.apply_chat_template(m, tools=t, tokenize=False, **kw)
                    t2 = time.perf_counter_ns()
                    tok(text, add_special_tokens=False)
                    t3 = time.perf_counter_ns()
                    full.append(t1 - t0); jin.append(t2 - t1); tk.append(t3 - t2)
                n = len(ref[c["id"]]["ids"])
                res[f"{s}/{c['id']}"] = {"tokens": n, "full_ns": sorted(full)[REPS // 2], "jinja_ns": sorted(jin)[REPS // 2], "tok_ns": sorted(tk)[REPS // 2]}
        out[name] = res
        tot = sum(v["full_ns"] for v in res.values())
        print(f"{name:45s} {len(res):4d} cases  mean {tot / len(res) / 1000:8.1f} us/render", flush=True)
    json.dump(out, open(os.environ.get("AFFORDANCE_BENCH_OUTPUT", f"{ROOT}/bench/results_hf.json"), "w"))


if __name__ == "__main__":
    main()
