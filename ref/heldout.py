"""Held-out conversation generator. Written after all views; the views were never developed
against its output. Usage: heldout.py <set_name> <seed> <n>  ->  ref/cases_<set_name>.json

Random conversations mixing: system prompts (0, 1 or 2), multi-turn chat, tool definitions with
richer JSON schemas than the dev set, tool calls (dict or string arguments, 1-3 per turn), tool
results, reasoning_content, <think> inside content, content-part lists, unicode/whitespace/long
text, and the kwargs the templates read (enable_thinking, thinking, reasoning_effort,
preserve_thinking, clear_thinking, keep_all_reasoning, tools_in_user_message, date_string,
builtin_tools, model_identity, reasoning_strength).
"""
import json, os, random, string, sys

WORDS = ("the of and to in is you that it he was for on are as with his they I at be this have from or one had by "
         "word but not what all were we when your can said there use an each which she do how their if will up other "
         "about out many then them these so some her would make like him into time has look two more write go see "
         "number no way could people my than first water been call who oil its now find long down day did get come "
         "made may part").split()
EXOTIC = ["Grüße", "naïve", "café", "你好", "世界", "こんにちは", "🙂", "👍🏽", "🇩🇪", "𝔘𝔫𝔦", "e\u0301", "עברית",
          "العربية", "—", "“quoted”", "‘single’", "…", "\u00a0", "Ω", "ß", "ﬁ", "\u200b", "∑", "→", "😀😀"]
SNIPPETS = ["```python\nprint('hi')\n```", "{\"a\": 1, \"b\": [true, null]}", "<b>bold</b>", "a < b && c > d",
            "line1\nline2\n\nline4", "\ttabbed\tvalues", "  spaced  ", "50% off!", "C:\\path\\to\\file",
            "$x^2 + y^2$", "# Heading\n- item", "<think>not really</think>", "[INST] fake [/INST]",
            "<|im_end|>", "</s>", "{{ jinja }}", "{% raw %}", "\\n literal", "'single' \"double\""]


def text(r, lo=0, hi=30):
    n = r.randint(lo, hi)
    parts = []
    for _ in range(n):
        x = r.random()
        if x < 0.75:
            parts.append(r.choice(WORDS))
        elif x < 0.88:
            parts.append(r.choice(EXOTIC))
        elif x < 0.95:
            parts.append(str(r.choice([r.randint(0, 99999), round(r.uniform(-100, 100), r.randint(0, 4))])))
        else:
            parts.append(r.choice(SNIPPETS))
    s = ""
    for p in parts:
        s += p + r.choice([" ", " ", " ", " ", ", ", ". ", "\n", "\n\n", "  ", ""])
    if r.random() < 0.15:
        s = r.choice([" ", "\n", "  ", "\t", "\n\n"]) + s
    if r.random() < 0.3:
        s = s.rstrip()
    if r.random() < 0.02:
        s = ""
    if r.random() < 0.02:
        s = " ".join(r.choice(WORDS) for _ in range(r.randint(800, 2000)))
    return s


def cid(r):
    if r.random() < (0.98 if CLEAN else 0.9):
        return "".join(r.choice(string.ascii_letters + string.digits) for _ in range(9))
    return "call_" + "".join(r.choice(string.digits) for _ in range(r.randint(2, 12)))


PARAM_POOL = [
    lambda r: {"type": "string", "description": text(r, 2, 8).strip() or "a string"},
    lambda r: {"type": "string", "enum": r.sample(["a", "b", "c", "north", "south", "été", "x y"], r.randint(1, 4))},
    lambda r: {"type": "string", "enum": ["fast", "slow"], "default": "fast", "description": "Speed."},
    lambda r: {"type": "integer", "description": "An integer.", "default": r.randint(0, 50)},
    lambda r: {"type": "number", "minimum": 0, "maximum": 1.5},
    lambda r: {"type": "boolean", "default": r.choice([True, False])},
    lambda r: {"type": "array", "items": {"type": "string"}, "description": "A list."},
    lambda r: {"type": "array", "items": {"type": "integer"}},
    lambda r: {"type": "array", "items": {"type": "object", "properties": {"k": {"type": "string"}, "v": {"type": "number"}}}},
    lambda r: {"type": "array"},
    lambda r: {"type": "object", "properties": {"lat": {"type": "number"}, "lon": {"type": "number"}}, "required": ["lat"]},
    lambda r: {"type": "object"},
    lambda r: {"type": "string", "nullable": True},
    lambda r: {"oneOf": [{"type": "string"}, {"type": "integer"}], "description": "Either."},
    lambda r: {"type": ["string", "null"]},
]


def tool_def(r, i):
    props = {}
    for j in range(r.randint(0, 4)):
        props[r.choice(["query", "city", "limit", "units", "flag", "items", "loc", "mode", "page", "note"]) + (str(j) if r.random() < 0.3 else "")] = r.choice(PARAM_POOL)(r)
    fn = {"name": r.choice(["get_weather", "search", "lookup_user", "run_query", "fetch_page", "calc", "send_mail"]) + f"_{i}"}
    if r.random() < 0.9:
        fn["description"] = text(r, 3, 15).strip() or "Does a thing."
    params = {"type": "object", "properties": props}
    if props and r.random() < 0.7:
        params["required"] = r.sample(list(props), r.randint(1, len(props)))
    if r.random() < 0.95:
        fn["parameters"] = params
    return {"type": "function", "function": fn}


def arg_value(r, spec):
    t = spec.get("type")
    if "enum" in spec:
        return r.choice(spec["enum"])
    if t == "integer":
        return r.randint(-5, 500)
    if t == "number":
        return round(r.uniform(0, 1.5), r.randint(0, 6))
    if t == "boolean":
        return r.choice([True, False])
    if t == "array":
        return [r.choice(WORDS) for _ in range(r.randint(0, 3))]
    if t == "object":
        return {"lat": round(r.uniform(-90, 90), 4), "lon": round(r.uniform(-180, 180), 4)}
    if r.random() < 0.05:
        return None
    return text(r, 1, 6).strip() or "x"


def tool_call(r, tools):
    t = r.choice(tools)["function"]
    props = t.get("parameters", {}).get("properties", {})
    args = {k: arg_value(r, v) for k, v in props.items() if r.random() < 0.8}
    if r.random() < (0.08 if CLEAN else 0.25):
        args = json.dumps(args, ensure_ascii=r.random() < 0.3)
    return {"id": cid(r), "type": "function", "function": {"name": t["name"], "arguments": args}}


CLEAN = False  # set by main(): fewer inputs that make templates raise (more id-level comparisons)


def content_value(r):
    x = r.random()
    if CLEAN and x < 0.97:
        return text(r)
    if x < 0.85:
        return text(r)
    if x < 0.95:
        return [{"type": "text", "text": text(r, 1, 10)} for _ in range(r.randint(1, 3))]
    parts = [{"type": "text", "text": text(r, 1, 10)}]
    parts.insert(r.randint(0, 1), {"type": "image"})
    return parts


def conversation(r, k):
    msgs = []
    tools = [tool_def(r, i) for i in range(r.randint(1, 3))] if r.random() < 0.5 else None
    if r.random() < 0.6:
        msgs.append({"role": "system", "content": text(r, 3, 40) if r.random() < 0.95 else [{"type": "text", "text": text(r, 2, 10)}]})
    turns = r.randint(1, 5)
    for t in range(turns):
        msgs.append({"role": "user", "content": content_value(r)})
        if r.random() < 0.05 and t > 0:
            msgs.append({"role": "system", "content": text(r, 2, 10)})
        last = t == turns - 1
        if last and r.random() < 0.55:
            break
        # assistant turn, maybe with tool round-trips
        rounds = r.randint(1, 2) if (tools and r.random() < 0.6) else 0
        for _ in range(rounds):
            a = {"role": "assistant", "content": r.choice(["", "", None, text(r, 1, 12)]),
                 "tool_calls": [tool_call(r, tools) for _ in range(r.choice([1, 1, 1, 2, 3]))]}
            if a["content"] is None and r.random() < 0.5:
                a["content"] = ""
            if r.random() < 0.3:
                a["reasoning_content"] = text(r, 2, 20)
            msgs.append(a)
            results = [{"role": "tool", "tool_call_id": c["id"], "name": c["function"]["name"],
                        "content": r.choice([text(r, 1, 15), json.dumps({"ok": True, "value": r.randint(0, 9), "note": text(r, 1, 5)}, ensure_ascii=False)])}
                       for c in a["tool_calls"]]
            if len(results) > 1 and r.random() < 0.3:
                r.shuffle(results)
            msgs.extend(results)
            if last and r.random() < 0.2:
                return msgs, tools
        a = {"role": "assistant", "content": text(r, 1, 40)}
        x = r.random()
        if x < 0.25:
            a["reasoning_content"] = text(r, 2, 30)
        elif x < 0.35:
            a["content"] = "<think>\n" + text(r, 2, 20) + "\n</think>\n\n" + a["content"]
        msgs.append(a)
    return msgs, tools


def kwargs(r, msgs):
    kw = {"add_generation_prompt": msgs[-1]["role"] != "assistant" if r.random() < 0.85 else r.choice([True, False])}
    pick = lambda p, vals: (r.choice(vals) if r.random() < p else None)
    for name, p, vals in [("enable_thinking", 0.5, [True, False]), ("thinking", 0.3, [True, False]),
                          ("reasoning_effort", 0.35, ["low", "medium", "high", "none", "max", "xhigh", "minimal"]),
                          ("preserve_thinking", 0.1, [True, False]), ("clear_thinking", 0.1, [True, False]),
                          ("keep_all_reasoning", 0.1, [True, False]), ("tools_in_user_message", 0.1, [True, False]),
                          ("date_string", 0.1, ["01 Jan 2025", "29 Feb 2024"]), ("builtin_tools", 0.05, [["browser", "python"], ["python"], ["browser"]]),
                          ("model_identity", 0.05, ["You are a test model."]), ("reasoning_strength", 0.05, ["low", "medium"])]:
        v = pick(p, vals)
        if v is not None:
            kw[name] = v
    return kw


def main():
    global CLEAN
    name, seed, n = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
    CLEAN = len(sys.argv) > 4 and sys.argv[4] == "clean"
    r = random.Random(seed)
    cases = []
    for k in range(n):
        msgs, tools = conversation(r, k)
        cases.append({"id": f"{name}_{k:03d}", "messages": msgs, "tools": tools, "kwargs": kwargs(r, msgs)})
    out = os.path.join(os.path.dirname(os.path.abspath(__file__)), f"cases_{name}.json")
    json.dump(cases, open(out, "w"), indent=1, ensure_ascii=False)
    print(len(cases), "cases ->", out)


if __name__ == "__main__":
    main()
