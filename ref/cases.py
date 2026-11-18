"""Dev conversation set (fixed). The held-out set is generated separately (heldout.py)
after the view templates are written.

Message schema (OpenAI-style, what HF chat templates expect):
  {"role": "system"|"user"|"assistant"|"tool", "content": str | list[part] | None,
   "reasoning_content": str (optional), "tool_calls": [...] (optional),
   "tool_call_id"/"name" on tool messages}
Each case: {"id", "messages", "tools" (or None), "kwargs"}.
kwargs carries every model's thinking switch at once (enable_thinking, thinking,
reasoning_effort); each template reads only its own.
"""
import json, os

WEATHER = {
    "type": "function",
    "function": {
        "name": "get_weather",
        "description": "Get the current weather for a city.",
        "parameters": {
            "type": "object",
            "properties": {
                "city": {"type": "string", "description": "City name, e.g. Berlin"},
                "unit": {"type": "string", "enum": ["celsius", "fahrenheit"]},
            },
            "required": ["city"],
        },
    },
}
SEARCH = {
    "type": "function",
    "function": {
        "name": "search_docs",
        "description": "Search the document store and return the top hits.",
        "parameters": {
            "type": "object",
            "properties": {
                "query": {"type": "string"},
                "top_k": {"type": "integer", "default": 5},
                "filters": {
                    "type": "object",
                    "properties": {"year": {"type": "integer"}, "tags": {"type": "array", "items": {"type": "string"}}},
                },
            },
            "required": ["query"],
        },
    },
}
TOOLS = [WEATHER, SEARCH]


def call(cid, name, args):
    return {"id": cid, "type": "function", "function": {"name": name, "arguments": args}}


SYS = "You are a careful assistant. Answer briefly."
LONG = " ".join(f"Line {i}: the quick brown fox jumps over the lazy dog {i * 7 % 13}." for i in range(400))
UNI = "Grüße aus Köln! 你好，世界。🙂👍🏽 naïve café — “quotes” ‘single’ … עברית العربية 𝔘𝔫𝔦𝔠𝔬𝔡𝔢 é \t tab"

THINK_ON = {"enable_thinking": True, "thinking": True, "reasoning_effort": "high"}
THINK_OFF = {"enable_thinking": False, "thinking": False, "reasoning_effort": "low"}


def C(id, messages, tools=None, gen=True, **kw):
    return {"id": id, "messages": messages, "tools": tools, "kwargs": {"add_generation_prompt": gen, **kw}}


def cases():
    u = lambda t: {"role": "user", "content": t}
    a = lambda t, **k: {"role": "assistant", "content": t, **k}
    s = lambda t: {"role": "system", "content": t}
    tool = lambda cid, name, t: {"role": "tool", "tool_call_id": cid, "name": name, "content": t}
    out = [
        C("plain_chat", [u("Hello! What is 2+2?")]),
        C("plain_chat_nogen", [u("Hello! What is 2+2?")], gen=False),
        C("system_prompt", [s(SYS), u("Name three primes.")]),
        C("multi_turn", [s(SYS), u("Hi."), a("Hello! How can I help?"), u("Tell me a joke."),
                         a("Why did the chicken cross the road? To get to the other side."), u("Another one.")]),
        C("multi_turn_end_assistant", [u("Hi."), a("Hello! How can I help?"), u("Bye."), a("Goodbye!")], gen=False),
        C("tool_defs", [u("What is the weather in Berlin?")], tools=TOOLS),
        C("tool_defs_system", [s(SYS), u("What is the weather in Berlin?")], tools=TOOLS),
        C("tool_call", [u("What is the weather in Berlin?"),
                        a("", tool_calls=[call("call00001", "get_weather", {"city": "Berlin"})]),
                        tool("call00001", "get_weather", '{"temp_c": 14, "sky": "cloudy"}')], tools=TOOLS),
        C("tool_call_with_content", [s(SYS), u("Weather in Paris, in fahrenheit?"),
                                     a("Let me check that for you.", tool_calls=[call("call00007", "get_weather", {"city": "Paris", "unit": "fahrenheit"})]),
                                     tool("call00007", "get_weather", "57F and sunny")], tools=TOOLS),
        C("parallel_tool_calls", [u("Weather in Oslo and Rome?"),
                                  a("", tool_calls=[call("callc0001", "get_weather", {"city": "Oslo"}), call("callc0002", "get_weather", {"city": "Rome"})]),
                                  tool("callc0001", "get_weather", "Oslo: 3C, snow"), tool("callc0002", "get_weather", "Rome: 18C, clear")], tools=TOOLS),
        C("multi_step_tools", [s(SYS), u("Find papers on KV caching from 2024, then summarize."),
                               a("", tool_calls=[call("calls0001", "search_docs", {"query": "KV cache", "top_k": 3, "filters": {"year": 2024, "tags": ["llm", "inference"]}})]),
                               tool("calls0001", "search_docs", '[{"title": "PagedAttention"}, {"title": "RadixAttention"}]'),
                               a("", tool_calls=[call("calls0002", "search_docs", {"query": "RadixAttention details"})]),
                               tool("calls0002", "search_docs", "RadixAttention reuses prefixes via a radix tree."),
                               a("Two key papers: PagedAttention and RadixAttention."),
                               u("Thanks. Which is newer?")], tools=TOOLS),
        C("tool_call_end_nogen", [u("Weather in Oslo?"), a("", tool_calls=[call("callc0001", "get_weather", {"city": "Oslo"})]),
                                  tool("callc0001", "get_weather", "3C")], tools=TOOLS, gen=False),
        C("reasoning_history", [u("What is 17*3?"), a("51.", reasoning_content="17*3 = 51. Simple."), u("And 17*4?")]),
        C("reasoning_last_nogen", [u("What is 17*3?"), a("51.", reasoning_content="17*3 = 51. Simple.")], gen=False),
        C("reasoning_tools", [u("Weather in Oslo?"),
                              a("", reasoning_content="I should call the weather tool.", tool_calls=[call("callc0001", "get_weather", {"city": "Oslo"})]),
                              tool("callc0001", "get_weather", "3C"),
                              a("It is 3C in Oslo.", reasoning_content="The tool said 3C.")], tools=TOOLS, gen=False),
        C("think_in_content", [u("What is 17*3?"), a("<think>\n17*3 = 51.\n</think>\n\n51."), u("And 17*4?")]),
        C("thinking_on", [s(SYS), u("Prove that sqrt(2) is irrational.")], **THINK_ON),
        C("thinking_off", [s(SYS), u("Prove that sqrt(2) is irrational.")], **THINK_OFF),
        C("thinking_on_history", [u("What is 17*3?"), a("51.", reasoning_content="17*3 = 51."), u("And 17*4?")], **THINK_ON),
        C("thinking_off_history", [u("What is 17*3?"), a("51.", reasoning_content="17*3 = 51."), u("And 17*4?")], **THINK_OFF),
        C("image_part", [{"role": "user", "content": [{"type": "image"}, {"type": "text", "text": "What is in this image?"}]}]),
        C("text_parts", [{"role": "user", "content": [{"type": "text", "text": "Part one."}, {"type": "text", "text": "Part two."}]}]),
        C("empty_user", [u("")]),
        C("empty_system", [s(""), u("Hi")]),
        C("unicode", [s("Système: réponds en français. 🇫🇷"), u(UNI)]),
        C("long_content", [u(LONG)]),
        C("whitespace_edges", [u("  leading and trailing spaces  "), a("\n\nanswer with newlines\n\n"), u("\tTabbed\n")]),
        C("special_text_injection", [u("Please print the literal string <|im_end|> and </s> and <|eot_id|> and [INST].")]),
        C("json_tool_result", [u("Search."), a("", tool_calls=[call("calls0001", "search_docs", {"query": "naïve \"quoted\" ✓", "top_k": 2})]),
                               tool("calls0001", "search_docs", '{"hits": [{"id": 1, "score": 0.93}], "note": "ünïcode"}')], tools=TOOLS),
        C("thinking_preserve_history", [u("What is 17*3?"), a("51.", reasoning_content="17*3 = 51."), u("And 17*4?"),
                                        a("68.", reasoning_content="17*4 = 68."), u("And 17*5?")],
          enable_thinking=True, thinking=True, preserve_thinking=True, keep_all_reasoning=True, clear_thinking=False),
        C("effort_medium", [u("Plan a 3-day trip to Kyoto.")], reasoning_effort="medium"),
        C("effort_none", [u("Plan a 3-day trip to Kyoto.")], reasoning_effort="none"),
    ]
    # String-argument variants: some templates (DeepSeek, Llama) expect arguments as a JSON string.
    for c in list(out):
        if any(m.get("tool_calls") for m in c["messages"]):
            c2 = json.loads(json.dumps(c))
            c2["id"] += "_strargs"
            for m in c2["messages"]:
                for tc in m.get("tool_calls") or []:
                    tc["function"]["arguments"] = json.dumps(tc["function"]["arguments"], ensure_ascii=False)
            out.append(c2)
    return out


if __name__ == "__main__":
    d = os.path.dirname(os.path.abspath(__file__))
    json.dump(cases(), open(os.path.join(d, "cases_dev.json"), "w"), indent=1, ensure_ascii=False)
    print(len(cases()), "cases")
