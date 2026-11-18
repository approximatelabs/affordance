"""Reference outputs for models that ship a Python encoder instead of a Jinja template
(DeepSeek-V3.2: encoding_dsv32.py; DeepSeek-V4-Flash/Pro: encoding_dsv4.py).

Official path: encode_messages(messages, thinking_mode, ...) -> str, then the model's
tokenizer with add_special_tokens=False (as in the repos' README and vLLM).

Adapter from our case format (our choice, documented in the report):
  - tools: attached to the first system message, or a new empty system message is inserted
    (the encoders read tools from the system message; same rule as V4.1's Jinja port).
  - thinking_mode = "thinking" if kwargs.thinking or kwargs.enable_thinking else "chat".
  - V4 reasoning_effort: passed only if it is "max" or "high" (the encoder asserts on others).
  - add_generation_prompt: not an encoder input (the encoder always ends a user turn with
    <｜Assistant｜>), so it is ignored, and the view must ignore it too.
"""
import copy, importlib.util, json, os, sys, warnings
warnings.filterwarnings("ignore")
from transformers import AutoTokenizer
from tokenizers import Tokenizer

HERE = os.path.dirname(os.path.abspath(__file__))
MODELS = {
    "deepseek-ai__DeepSeek-V3.2": ("encoding/encoding_dsv32.py", "v32"),
    "deepseek-ai__DeepSeek-V4-Flash": ("encoding/encoding_dsv4.py", "v4"),
    "deepseek-ai__DeepSeek-V4-Pro": ("encoding/encoding_dsv4.py", "v4"),
}


def load(path):
    spec = importlib.util.spec_from_file_location("enc_" + str(abs(hash(path))), path)
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


def adapt(case, kind):
    msgs = copy.deepcopy(case["messages"])
    kw = case["kwargs"]
    if case["tools"]:
        if msgs and msgs[0]["role"] == "system":
            msgs[0]["tools"] = copy.deepcopy(case["tools"])
        else:
            msgs.insert(0, {"role": "system", "content": "", "tools": copy.deepcopy(case["tools"])})
    args = {"thinking_mode": "thinking" if (kw.get("thinking") or kw.get("enable_thinking")) else "chat"}
    if kind == "v4" and kw.get("reasoning_effort") in ("max", "high"):
        args["reasoning_effort"] = kw["reasoning_effort"]
    return msgs, args


def main():
    set_name = sys.argv[1] if len(sys.argv) > 1 else "dev"
    cases = json.load(open(os.path.join(HERE, f"cases_{set_name}.json")))
    outd = os.path.join(HERE, "out", set_name)
    os.makedirs(outd, exist_ok=True)
    for name, (encf, kind) in MODELS.items():
        d = os.path.join(HERE, "models", name)
        enc = load(os.path.join(d, encf))
        tok = AutoTokenizer.from_pretrained(d)
        tdir = os.path.join(HERE, "tokenizers")
        open(os.path.join(tdir, name + ".json"), "w").write(tok.backend_tokenizer.to_str())
        json.dump(tok.special_tokens_map, open(os.path.join(tdir, name + ".special.json"), "w"), ensure_ascii=False)
        res = {}
        for c in cases:
            try:
                msgs, args = adapt(c, kind)
                text = enc.encode_messages(msgs, **args)
                ids = tok(text, add_special_tokens=False)["input_ids"]
                res[c["id"]] = {"ids": list(ids), "text": text}
            except Exception as e:
                res[c["id"]] = {"error": f"{type(e).__name__}: {str(e)[:300]}"}
        json.dump(res, open(os.path.join(outd, name + ".json"), "w"), ensure_ascii=False)
        print(f"OK {name}: {len(res)} cases, {sum('error' in v for v in res.values())} encoder errors")


if __name__ == "__main__":
    main()
