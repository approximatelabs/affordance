"""Independent HF references for the middle layer. Python is a reference adapter.

Inputs (including body text and tool schemas) are constructed here, without Rust
rendering or decoding. Stored references are tests only, never product tables.
"""
import json
from pathlib import Path
from transformers import AutoTokenizer

ROOT = Path(__file__).resolve().parent.parent
NEMOTRON = ROOT / "ref/middle_models/nvidia__NVIDIA-Nemotron-3-Nano-30B-A3B-BF16"
INDEX = json.loads((ROOT / "views/template_apps/index.json").read_text())
OUT = ROOT / "ref/middle_out"
OUT.mkdir(exist_ok=True)

def tokenizer(model):
    return AutoTokenizer.from_pretrained(NEMOTRON if model.startswith("nvidia__") else ROOT / "ref/models" / model, local_files_only=True)

def kwargs(family):
    kw = {"add_generation_prompt": True}
    if family in ["qwen3_base", "qwen36", "glm53", "mimo25", "nemotron3"]:
        kw["enable_thinking"] = False
    if family == "qwen36": kw["preserve_thinking"] = True
    if family == "llama31": kw.update(tools_in_user_message=False, date_string="26 Jul 2024")
    return kw

LIVE_TOOL = {"type": "function", "function": {"name": "change__live", "description": "Choose the live action.",
    "parameters": {"type": "object", "properties": {"text": {"type": "string"}}, "required": ["text"]}}}
LOG = [
    {"kind": "input", "content": "Remember the goal."},
    {"kind": "screen", "content": "Old screen."},
    {"kind": "model", "content": "", "calls": [{"type": "function", "function": {"name": "change__live", "arguments": {"text": "old"}}}]},
    {"kind": "action", "name": "change__live", "result": "ok"},
    {"kind": "screen", "content": "Another old screen."},
    {"kind": "metric", "prefill_ms": 7},
]

def turns(log, strategy, current):
    result = []
    for e in log:
        if e["kind"] in ["screen", "input"]:
            if e["kind"] != "screen" or strategy == "unrolled":
                result.append({"role": "user", "content": e["content"]})
        elif e["kind"] == "model":
            t = {"role": "assistant", "content": e["content"]}
            if "calls" in e: t["tool_calls"] = e["calls"]
            result.append(t)
        elif e["kind"] == "action":
            result.append({"role": "tool", "content": e["result"], "name": e["name"]})
    return result + [{"role": "user", "content": current}]

cases = []
for family, spec in INDEX.items():
    tok = tokenizer(spec["model"])
    for strategy in ["last-turn", "unrolled"]:
        for log in [[], LOG]:
            for text in ["Live screen.", "Live screen: café 🙂"]:
                msgs = [{"role": "system", "content": "Strategy test."}] + turns(log, strategy, text)
                ids = tok.apply_chat_template(msgs, tools=[LIVE_TOOL], tokenize=True, **kwargs(family))
                if hasattr(ids, "keys"): ids = ids["input_ids"]
                cases.append({"family": family, "model": spec["model"], "state": {"strategy": strategy, "log": log, "text": text}, "ids": list(ids)})
(OUT / "strategies.json").write_text(json.dumps(cases, ensure_ascii=False, indent=2)+"\n")
print(f"{len(cases)} independent middle-layer references")

def random_numbers(seed):
    result = []
    for _ in range(1024):
        seed ^= (seed << 13) & (2**64 - 1)
        seed ^= seed >> 7
        seed ^= (seed << 17) & (2**64 - 1)
        result.append(seed % 1_000_000)
    return result

BIG_TOOLS = [
    ("change__scratchpad", "Replace your scratchpad.", {"text":{"type":"string"}}),
    ("scroll__notebook", "Scroll the notebook by a number of rows.", {"by":{"type":"integer"}}),
    ("change__answer", "Replace the answer field.", {"text":{"type":"string"}}),
    ("click__submit", "Submit the value currently in the answer field.", {}),
]
BIG_TOOLS = [{"type":"function","function":{"name":name,"description":desc,"parameters":{"type":"object","properties":props,"required":list(props)}}} for name,desc,props in BIG_TOOLS]
big = []
for family in ["qwen36", "nemotron3"]:
    spec = INDEX[family]
    tok = tokenizer(spec["model"])
    for strategy in ["last-turn", "unrolled"]:
        for log in [[], LOG]:
            for seed in [20261008, 20261009]:
                ns = random_numbers(seed)
                body = ("Goal: Find the biggest of the 1,024 notebook numbers. Write it in Answer, then press Submit.\n"
                    + "Scratchpad: \nNotebook (1024 numbers), starts at row 0:\n"
                    + "\n".join(f"{i}: {n}" for i,n in enumerate(ns)) + "\nAnswer: \nSubmit")
                msgs = [{"role":"system","content":"Use the available tools to interact with the app."}] + turns(log,strategy,body)
                ids = tok.apply_chat_template(msgs,tools=BIG_TOOLS,tokenize=True,**kwargs(family))
                if hasattr(ids,"keys"): ids=ids["input_ids"]
                big.append({"family":family,"model":spec["model"],"state":{"numbers":ns,"budget":30000,"strategy":strategy,"log":log},"ids":list(ids)})
(OUT / "find_big_number.json").write_text(json.dumps(big,ensure_ascii=False,indent=2)+"\n")
print(f"{len(big)} independent 1,024-number references")

# A separate audit for the newly ported NVIDIA template, beyond app composition.
tok = tokenizer(INDEX["nemotron3"]["model"])
audit = []
for i in range(120):
    text = ["plain", " café 🙂 ", "line one\nline two", "", "<think>hidden</think>done", "<think>unfinished"][i % 6]
    tool = {"type":"function","function":{"name":"choose","description":" Choose a value. ","parameters":{
        "type":"object","properties":{"value":{"type":"string","description":" Value ","enum":["café","other"],"default":"café"}},
        "required":["value"],"additionalProperties":False},"strict":True}}
    msgs = ([{"role":"system","content":"Audit."}] if i % 3 else []) + [{"role":"user","content":"Question " + str(i)}]
    assistant = {"role":"assistant","content":text}
    if i % 4 == 0: assistant["reasoning_content"] = " thinking "
    if i % 2 == 0:
        assistant["tool_calls"] = [{"type":"function","function":{"name":"choose","arguments":{"value":"café","count":2,"flags":[True,None]}}}]
    msgs.append(assistant)
    if i % 2 == 0:
        msgs.append({"role":"tool","content":"ok"})
        if i % 5 == 0: msgs.append({"role":"tool","content":"second result"})
    if i % 3 == 0: msgs.append({"role":"user","content":"Continue."})
    kw = {"add_generation_prompt":bool(i % 2),"enable_thinking":bool(i % 4),"truncate_history_thinking":bool(i % 5)}
    case = {"messages":msgs,"tools":[tool] if i % 7 else [],"kwargs":kw}
    ids=tok.apply_chat_template(msgs,tools=case["tools"],tokenize=True,**kw)
    if hasattr(ids,"keys"):ids=ids["input_ids"]
    audit.append({"case":case,"ids":list(ids)})
(OUT / "nemotron_audit.json").write_text(json.dumps(audit,ensure_ascii=False,indent=2)+"\n")
print(f"{len(audit)} independent NVIDIA template audit cases")
