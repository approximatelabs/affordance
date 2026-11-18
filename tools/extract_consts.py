"""Copy long constant prompt texts verbatim from shipped templates into view-language
const declarations (template text, not expected outputs)."""
import json, re, sys

ROOT = __file__.rsplit("/tools/", 1)[0]


def gptoss():
    t = open(f"{ROOT}/ref/models/openai__gpt-oss-20b/chat_template.jinja").read()
    m = re.search(r"macro render_builtin_tools.*?endmacro", t, re.S).group(0)
    browser = re.search(r"if browser_tool %\}(.*?)\{%- endif", m, re.S).group(1)
    py = re.search(r"if python_tool %\}(.*?)\{%- endif", m, re.S).group(1)

    def cat(block):
        parts = re.findall(r'\{\{- "((?:[^"\\]|\\.)*)" \}\}', block)
        return "".join(json.loads('"' + p + '"') for p in parts)

    out = "// Builtin tool descriptions, copied verbatim from the gpt-oss template.\n\"use strict_tokens\";\n"
    out += "const BROWSER_TOOL = %s;\n" % json.dumps(cat(browser), ensure_ascii=False)
    out += "const PYTHON_TOOL = %s;\n" % json.dumps(cat(py), ensure_ascii=False)
    open(f"{ROOT}/views/third_party/openai/gpt_oss.tsx", "w").write(out)


def mimo():
    import re as _re
    t = json.load(open(f"{ROOT}/ref/models/XiaomiMiMo__MiMo-V2.5-Pro/tokenizer_config.json"))["chat_template"]
    pro = _re.search(r'\{\{- "<\|im_start\|>system\\n(You are MiMo, a warm.*?)" \}\}', t, _re.S).group(1)
    t2 = json.load(open(f"{ROOT}/ref/models/XiaomiMiMo__MiMo-V2-Flash/tokenizer_config.json"))["chat_template"]
    fmt = _re.search(r"\{\{- '(\\n\\nFor each function call, output.*?)' \}\}", t2, _re.S).group(1)
    out = "// MiMo prompt constants, copied verbatim from the templates.\n\"use strict_tokens\";\n"
    out += "const MIMO_PRO_SYSTEM = %s;\n" % json.dumps(json.loads('"' + pro + '"'), ensure_ascii=False)
    # Under strict_tokens a special token is a `tok.*` value, never text.
    names = {"<tool_call>": "tok.tool_call", "</tool_call>": "tok.tool_call_end", "<think>": "tok.think", "</think>": "tok.think_end"}
    parts = _re.split(r"(</?tool_call>|</?think>)", fmt.replace("\\n", "\n"))
    expr = " + ".join(names[p] if p in names else json.dumps(p, ensure_ascii=False) for p in parts if p)
    out += "const MIMO_TOOL_FORMAT = (%s);\n" % expr
    open(f"{ROOT}/views/third_party/xiaomi/mimo.tsx", "w").write(out)


def glm():
    t = json.load(open(f"{ROOT}/ref/models/zai-org__glm-4-9b-chat-hf/tokenizer_config.json"))["chat_template"]
    head = re.search(r"<\|system\|>\n(.*?)\{% set tools", t, re.S).group(1)
    fn_tail = re.search(r"tojson\(indent=4\) \}\}\n(.*?)\{% elif tool\['type'\] == 'python' %\}", t, re.S).group(1)
    py = re.search(r"\{% elif tool\['type'\] == 'python' %\}\n(.*?)\{% elif", t, re.S).group(1)
    br = re.search(r"\{% elif tool\['type'\] == 'simple_browser' %\}\n(.*?)\{% elif", t, re.S).group(1)
    cv = re.search(r"\{% elif tool\['type'\] == 'cogview' %\}\n(.*?)\{% endif %\}", t, re.S).group(1)
    out = "// glm-4-9b-chat prompt texts, copied verbatim from its template (trim_blocks applied by hand:\n// the first newline after each block tag is dropped).\n\"use strict_tokens\";\n"
    for name, val in [("GLM4_SYS", head), ("GLM4_FN_TAIL", fn_tail), ("GLM4_PYTHON", py), ("GLM4_BROWSER", br), ("GLM4_COGVIEW", cv)]:
        out += "const %s = %s;\n" % (name, json.dumps(val, ensure_ascii=False))
    open(f"{ROOT}/views/third_party/zai-org/glm4.tsx", "w").write(out)


if __name__ == "__main__":
    {"gptoss": gptoss, "mimo": mimo, "glm": glm}[sys.argv[1]]()
