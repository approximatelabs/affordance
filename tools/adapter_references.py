"""Independent wrapper check: official cached templates, semantic app fixtures.

No Rust render/decode is used to construct the expected prompt. The fixtures use
one hard-bounded action with text that needs no trimming at either edge.
"""
import json
from pathlib import Path
from transformers import AutoTokenizer

ROOT = Path(__file__).resolve().parent.parent
MODELS = {
    "qwen3": "Qwen__Qwen3-4B-Instruct-2507",
    "llama31": "meta-llama__Llama-3.1-8B-Instruct",
}
out = []
for format, model in MODELS.items():
    tok = AutoTokenizer.from_pretrained(ROOT / "ref/models" / model, local_files_only=True)
    for text, active, question in [("Notes: ready", True, ""), ("Notes: café 🙂", True, "Update."), ("Empty.", False, "")]:
        tools = [{"type": "function", "function": {
            "name": "change__notes", "description": "Replace notes",
            "parameters": {"type": "object", "properties": {"text": {}}, "required": ["text"]},
        }}] if active else []
        content = (question + "\n" if question else "") + text
        messages = [{"role": "system", "content": "App test."}, {"role": "user", "content": content}]
        kwargs = {"add_generation_prompt": True}
        if format == "llama31":
            kwargs.update(tools_in_user_message=False, date_string="26 Jul 2024")
        ids = tok.apply_chat_template(messages, tools=tools, tokenize=True, **kwargs)
        if hasattr(ids, "keys"):
            ids = ids["input_ids"]
        out.append({"model": model, "state": {"format": format, "text": text, "active": active, "question": question}, "ids": list(ids)})
path = ROOT / "results/a2/adapters_reference.json"
path.write_text(json.dumps(out, ensure_ascii=False, indent=2) + "\n")
print(f"{len(out)} independent references written to {path}")
