"""Independently regenerate cheap-tier references from cached official model files.

No downloads or reference edits. Exports used by the HF reference helper go to a
temporary directory. Run with the project's .venv Python.
"""
import copy
import json
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "ref"))
import render_ref as hf
import render_ref_pyenc as ds
import render_ref_mistral_common as mc
from transformers import AutoTokenizer


def render(name, cases):
    directory = ROOT / "ref" / "models" / name
    if name in ds.MODELS:
        source, kind = ds.MODELS[name]
        encoder = ds.load(str(directory / source))
        tok = AutoTokenizer.from_pretrained(str(directory), local_files_only=True)
        result = {}
        for c in cases:
            try:
                messages, args = ds.adapt(c, kind)
                text = encoder.encode_messages(messages, **args)
                result[c["id"]] = {"ids": list(tok(text, add_special_tokens=False)["input_ids"]), "text": text}
            except Exception as e:
                result[c["id"]] = {"error": f"{type(e).__name__}: {str(e)[:300]}"}
        return result
    if name in mc.MODELS:
        path = str(directory / "tekken.json")
        tokenizers = {m: mc.MistralTokenizer.from_file(path, mode=m) for m in (mc.ValidationMode.test, mc.ValidationMode.finetuning)}
        result = {}
        for c in cases:
            try:
                messages = copy.deepcopy(c["messages"])
                mode = mc.ValidationMode.finetuning if messages and messages[-1]["role"] == "assistant" else mc.ValidationMode.test
                request = mc.ChatCompletionRequest.from_openai(messages, tools=copy.deepcopy(c["tools"]))
                output = tokenizers[mode].encode_chat_completion(request)
                result[c["id"]] = {"ids": list(output.tokens), "text": output.text}
            except Exception as e:
                result[c["id"]] = {"error": f"{type(e).__name__}: {str(e)[:300]}"}
        return result
    return hf.render_model(str(directory), cases)[0]


def main():
    # A new set is generated only after the views and renderer have been written.
    # Store its official outputs separately; never derive them from the Rust renderer.
    if len(sys.argv) == 2:
        name = sys.argv[1]
        cases = json.loads((ROOT / "ref" / f"cases_{name}.json").read_text())
        out = ROOT / "ref" / "out" / name
        out.mkdir(parents=True, exist_ok=True)
        models = json.loads((ROOT / "views" / "index.json").read_text())
        with tempfile.TemporaryDirectory(prefix="affordance-refs-") as tmp:
            hf.HERE = tmp
            for model in models:
                if model.startswith("_"):
                    continue
                result = render(model, cases)
                (out / f"{model}.json").write_text(json.dumps(result, ensure_ascii=False))
                print(f"{model}: {len(result)} official references", flush=True)
        return 0
    count = errors = 0
    with tempfile.TemporaryDirectory(prefix="affordance-refs-") as tmp:
        hf.HERE = tmp
        for refdir in sorted((ROOT / "ref" / "new_out").iterdir()):
            cases = json.loads((ROOT / "ref" / f"cases_{refdir.name}.json").read_text())
            for path in sorted(refdir.glob("*.json")):
                expected = json.loads(path.read_text())
                regenerated = render(path.stem, cases)
                for c in cases:
                    key = c["id"]
                    # Includes full token ids, text, and error messages.
                    if regenerated[key] != expected[key]:
                        errors += 1
                        print(f"DIFFERENT {path.stem} {key}: expected {expected[key].get('error', 'ids/text')!r}, regenerated {regenerated[key].get('error', 'ids/text')!r}", flush=True)
                count += len(cases)
            print(f"{refdir.name}: verified {len(cases)} cases per model", flush=True)
    print(f"TOTAL {count} model-case pairs, {errors} changed references")
    return bool(errors)


if __name__ == "__main__":
    sys.exit(main())
