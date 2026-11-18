"""Agent-loop cases (ref/cases_agent_loops.json): the conversations an agent harness renders every
turn. Tools are always declared; the model calls tools with object arguments, sometimes with
reasoning or prose, one call or several; results come back; the loop goes on or ends in an answer
and a new user turn. Each case is rendered with and without thinking (where a family has a switch).

  uv run python ref/make_agent_loops.py
"""
import json, os

TOOLS = [
    {"type": "function", "function": {"name": "shell", "description": "Run a shell command and return its output.",
        "parameters": {"type": "object", "properties": {"command": {"type": "string"}, "timeout_s": {"type": "integer"}}, "required": ["command"]}}},
    {"type": "function", "function": {"name": "set_notes", "description": "Replace your notes.",
        "parameters": {"type": "object", "properties": {"text": {"type": "string"}}, "required": ["text"]}}},
]

def call(i, name, args):
    return {"id": f"call{i:05d}", "type": "function", "function": {"name": name, "arguments": args}}

def asst(calls=None, content="", reasoning=None):
    m = {"role": "assistant", "content": content}
    if calls: m["tool_calls"] = calls
    if reasoning is not None: m["reasoning_content"] = reasoning
    return m

def tool(i, name, content):
    return {"role": "tool", "tool_call_id": f"call{i:05d}", "name": name, "content": content}

user = {"role": "user", "content": "Keep the build green. Start by checking the tests."}
ls = call(1, "shell", {"command": "ls -la /srv && cat README.md"})
out1 = "total 8\ndrwxr-xr-x 2 root root 4096 Oct 17 12:00 .\n-rw-r--r-- 1 root root  120 Oct 17 12:00 README.md\n# srv\nRun `make test`."
test = call(2, "shell", {"command": "make test", "timeout_s": 120})
out2 = '{"passed": 41, "failed": 1, "failures": ["test_upload_retry"]}'
notes = call(3, "set_notes", {"text": "test_upload_retry is flaky; quarantined."})

convs = {
    "one_call": [user, asst([ls]), tool(1, "shell", out1)],
    "one_call_reasoning": [user, asst([ls], reasoning="I should look around first."), tool(1, "shell", out1)],
    "prose_and_call": [user, asst([ls], content="Let me look at the repository."), tool(1, "shell", out1)],
    "two_rounds": [user, asst([ls], reasoning="Look first."), tool(1, "shell", out1),
                   asst([test], reasoning="Now the tests."), tool(2, "shell", out2)],
    "parallel_calls": [user, asst([ls, test]), tool(1, "shell", out1), tool(2, "shell", out2)],
    "answer_then_new_task": [user, asst([test]), tool(2, "shell", out2),
                             asst(content="One test fails: test_upload_retry.", reasoning="Report it."),
                             {"role": "user", "content": "Quarantine it and note why."}],
    "three_rounds_with_system": [{"role": "system", "content": "You are a careful build engineer."}, user,
                                 asst([ls]), tool(1, "shell", out1), asst([test]), tool(2, "shell", out2),
                                 asst([notes], reasoning="Write it down."), tool(3, "set_notes", "notes saved")],
    "message_between_turns": [user, asst([test]), tool(2, "shell", out2),
                              {"role": "user", "content": "[message from agent-0002] I am fixing test_upload_retry."}],
}

cases = []
for name, messages in convs.items():
    for think in (None, True, False):
        kw = {"add_generation_prompt": True}
        if think is not None: kw["enable_thinking"] = think
        suffix = "" if think is None else ("_think" if think else "_nothink")
        cases.append({"id": name + suffix, "messages": messages, "tools": TOOLS, "kwargs": kw})

here = os.path.dirname(os.path.abspath(__file__))
json.dump(cases, open(os.path.join(here, "cases_agent_loops.json"), "w"), indent=1, ensure_ascii=False)
print(f"{len(cases)} cases")
