"""Fresh app boundary audit against official templates, with no Rust-derived gold.

Run once to generate references, then --check after building the affordance render command.
Native bodies are hard boundaries; string trimming and merges may intentionally
differ. Check their control-token sequence and span bounds independently.
"""
import json
import random
import subprocess
import sys
import tempfile
from pathlib import Path
from transformers import AutoTokenizer

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "results/a2"
REFERENCE = OUT / "adapters_boundary_reference.json"
MODELS = {
    "qwen3": "Qwen__Qwen3-4B-Instruct-2507",
    "llama31": "meta-llama__Llama-3.1-8B-Instruct",
}
SEED = 202610061402


def generate():
    rng = random.Random(SEED)
    words = ["alpha", "BETA", "42", "café", "中文", "🙂", "e\u0301", "<ordinary>", "[row]", "a/b", "'quoted'"]
    states = []
    for i in range(100):
        text = " ".join(rng.choice(words) for _ in range(rng.randint(1, 14)))
        if i >= 50:
            text = rng.choice(["\n", "\n\n", " ", "\t", "\u2003"]) + text + rng.choice(["\n", "\n\n", " ", "\t", "\u00a0"])
        states.append({"text": text, "active": rng.choice([True, False]), "question": rng.choice(["", "Update."])})
    cases = []
    for format, model in MODELS.items():
        tok = AutoTokenizer.from_pretrained(ROOT / "ref/models" / model, local_files_only=True)
        for i, state in enumerate(states):
            tools = [{"type": "function", "function": {
                "name": "change__notes", "description": "Replace notes",
                "parameters": {"type": "object", "properties": {"text": {}}, "required": ["text"]},
            }}] if state["active"] else []
            content = (state["question"] + "\n" if state["question"] else "") + state["text"]
            messages = [{"role": "system", "content": "App test."}, {"role": "user", "content": content}]
            kwargs = {"add_generation_prompt": True}
            if format == "llama31":
                kwargs.update(tools_in_user_message=False, date_string="26 Jul 2024")
            ids = tok.apply_chat_template(messages, tools=tools, tokenize=True, **kwargs)
            if hasattr(ids, "keys"):
                ids = ids["input_ids"]
            cases.append({"model": model, "index": i, "group": "clean_edges" if i < 50 else "whitespace_edges",
                          "state": {"format": format, **state}, "ids": list(ids)})
    REFERENCE.write_text(json.dumps({"seed": SEED, "cases": cases}, ensure_ascii=False) + "\n")
    print(f"Generated {len(cases)} fresh cases from cached official templates")


def check():
    reference = json.loads(REFERENCE.read_text())
    rows = []
    differences = []
    for format, model in MODELS.items():
        cases = [c for c in reference["cases"] if c["model"] == model]
        tok = AutoTokenizer.from_pretrained(ROOT / "ref/models" / model, local_files_only=True)
        specials = set(tok.all_special_ids)
        fixture = "adapter_reference_llama31.tsx" if format == "llama31" else "adapter_reference.tsx"
        with tempfile.TemporaryDirectory(prefix="afr_boundary_") as tmp:
            path = Path(tmp) / "states.json"
            path.write_text(json.dumps([c["state"] for c in cases], ensure_ascii=False))
            proc = subprocess.run([str(ROOT / "target/release/affordance"), "render", "--model", model,
                                   "--view", str(ROOT / "tests/fixtures" / fixture), "--entry", "AdapterReference",
                                   "--state", str(path), "--batch", "--cache", "--json"], capture_output=True, text=True, check=True)
        results = json.loads(proc.stdout)
        assert len(results) == len(cases)
        for group in ["clean_edges", "whitespace_edges"]:
            exact = control = valid_spans = count = 0
            for case, result in zip(cases, results):
                if case["group"] != group:
                    continue
                count += 1
                ids = result["ids"]
                match = ids == case["ids"]
                exact += match
                control += [i for i in ids if i in specials] == [i for i in case["ids"] if i in specials]
                valid_spans += all(0 <= s["start"] <= s["end"] <= len(ids) for s in result["spans"])
                actions = [s for s in result["spans"] if s["kind"] == "affordance" and "handler" in s["meta"]]
                assert len(actions) == int(case["state"]["active"])
                if actions:
                    s = actions[0]
                    assert ids[s["start"]:s["end"]] == tok.backend_tokenizer.encode(case["state"]["text"], add_special_tokens=False).ids
                if not match:
                    first = next((i for i, (a, b) in enumerate(zip(ids, case["ids"])) if a != b), min(len(ids), len(case["ids"])))
                    differences.append({"model": model, "index": case["index"], "state": case["state"], "first_diff": first,
                                        "native_text": tok.decode(ids, skip_special_tokens=False, clean_up_tokenization_spaces=False),
                                        "official_text": tok.decode(case["ids"], skip_special_tokens=False, clean_up_tokenization_spaces=False)})
            rows.append({"model": model, "group": group, "count": count, "exact": exact,
                         "same_control_tokens": control, "valid_spans": valid_spans})
    result = {"seed": reference["seed"], "results": rows, "differences": differences}
    (OUT / "adapters_boundary_check.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    for row in rows:
        print(row)
    assert all(r["same_control_tokens"] == r["valid_spans"] == r["count"] for r in rows)


if __name__ == "__main__":
    check() if "--check" in sys.argv else generate()
