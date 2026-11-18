"""Runs INSIDE the sandbox (ref/sandbox/run.sh): a model's own tokenizer class, loaded with
trust_remote_code, renders every case set and encodes probe texts. It can read /model and
/cases and write /out; it has no network. Output: /out/<set>.json as render_ref.py writes them
({case: {ids, text} | {error}}), /out/probes.json ({text: ids}) and /out/special.json."""
import copy, datetime, json, os, sys, warnings
# The tiktoken library keeps the vocabularies it reads in a folder it writes to. The folder is
# in the container's own temporary space, seeded with the public vocabulary files fetched
# beforehand (there is no network here to fetch them).
import shutil
if os.path.isdir("/tiktoken_seed"):
    shutil.copytree("/tiktoken_seed", "/tmp/tiktoken_cache", dirs_exist_ok=True)
else:
    os.makedirs("/tmp/tiktoken_cache", exist_ok=True)
warnings.filterwarnings("ignore")
import transformers.utils.chat_template_utils as ctu
from transformers import AutoTokenizer

FIXED_NOW = datetime.datetime(2026, 10, 5, 12, 0, 0)


class _FixedDT(datetime.datetime):
    @classmethod
    def now(cls, tz=None):
        return FIXED_NOW


ctu.datetime = _FixedDT
# A tokenizer class can sit in a file that also imports the model's image and video libraries.
# Those are not installed; empty stand-ins let the file load, and the tokenizer never calls them.
import importlib.util, types
for missing in ["torch", "decord", "torchvision", "cv2"]:
    if importlib.util.find_spec(missing) is None:
        stand_in = types.ModuleType(missing)
        stand_in.__getattr__ = lambda name: type(name, (), {})
        sys.modules[missing] = stand_in
# There is no network here. A file that downloads something as it loads (ERNIE's fetches a font
# for drawing on video frames) gets an empty reply.
import requests
requests.get = lambda *a, **k: types.SimpleNamespace(content=b"", status_code=200)
tok = AutoTokenizer.from_pretrained("/model", trust_remote_code=True)
if tok.chat_template is None:
    for name in ["chat_template.jinja", "chat_template.json"]:
        p = os.path.join("/model", name)
        if os.path.exists(p):
            raw = open(p).read()
            tok.chat_template = raw if name.endswith(".jinja") else json.loads(raw)["chat_template"]
            break
texts = set()
for f in sorted(os.listdir("/cases")):
    if not (f.startswith("cases_") and f.endswith(".json")): continue
    s = f[len("cases_"):-len(".json")]
    res = {}
    for c in json.load(open(os.path.join("/cases", f))):
        try:
            kw = dict(c["kwargs"])
            text = tok.apply_chat_template(copy.deepcopy(c["messages"]), tools=copy.deepcopy(c["tools"]), tokenize=False, **kw)
            enc = tok.apply_chat_template(copy.deepcopy(c["messages"]), tools=copy.deepcopy(c["tools"]), tokenize=True, **kw)
            ids = enc["input_ids"] if hasattr(enc, "keys") else enc
            res[c["id"]] = {"ids": [int(i) for i in ids], "text": text}
            texts.add(text)
        except Exception as e:
            res[c["id"]] = {"error": f"{type(e).__name__}: {str(e)[:300]}"}
    json.dump(res, open(f"/out/{s}.json", "w"), ensure_ascii=False)
    print(s, len(res), "cases,", sum("error" in v for v in res.values()), "errors", flush=True)
probes = sorted(texts) + [
    "hello world", " leading space", "trailing space ", "  two  spaces  ", "\n\nnewlines\n", "tabs\tand\ttabs", "CamelCase snake_case kebab-case",
    "numbers 1234567890 3.14159 1e-9", "naïve café ß ﬁ", "你好，世界。こんにちは 한국어", "العربية עברית", "emoji 😀🇩🇪👨‍👩‍👧", "don't we'll I'm they've",
    "<html><body>&amp;</body></html>", "{\"json\": [1, 2, {\"a\": null}]}", "x" * 300, " " * 40, "\r\n windows \r\n", "​ zero width   nbsp", "",
]
out = {}
for t in probes:
    try:
        out[t] = [int(i) for i in tok.encode(t, add_special_tokens=False)]
    except Exception as e:
        out[t] = {"error": f"{type(e).__name__}: {str(e)[:200]}"}
json.dump(out, open("/out/probes.json", "w"), ensure_ascii=False)
special = {}
for k, v in tok.special_tokens_map.items():
    special[k] = v if isinstance(v, (str, list)) else str(v)
json.dump(special, open("/out/special.json", "w"), ensure_ascii=False)
added = {str(i): {"content": t.content, "special": bool(t.special)} for i, t in getattr(tok, "added_tokens_decoder", {}).items()}
# A tiktoken-backed class keeps its own table of special tokens (every reserved id has a name).
own = getattr(tok, "special_tokens", None)
own = {str(k): int(v) for k, v in own.items()} if isinstance(own, dict) and all(isinstance(v, int) for v in own.values()) else None
json.dump({"class": type(tok).__name__, "vocab_size": len(tok), "added": added, "special_ids": own}, open("/out/info.json", "w"), ensure_ascii=False)
print("done", type(tok).__name__, len(tok), flush=True)
