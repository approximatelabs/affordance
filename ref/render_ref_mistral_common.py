"""Reference outputs for Mistral models that ship only tekken.json (no HF chat template):
Mistral-Small-3.2-24B-Instruct-2506 and Magistral-Small-2509. The official encoder is
mistral_common, which builds token ids directly (control tokens + per-piece encoding).

Adapter (our choice, documented in the report):
  - messages/tools converted with ChatCompletionRequest.from_openai.
  - ValidationMode.test (generation) when the last message is user/tool; ValidationMode.finetuning
    when the last message is an assistant turn (mistral_common appends EOS there).
    add_generation_prompt is not an input of mistral_common.
  - kwargs (enable_thinking, reasoning_effort, ...) are not inputs of mistral_common and are ignored.

The HF tokenizer used by the Rust renderer is transformers' tekken converter
(convert_tekken_tokenizer); it is exported to ref/tokenizers/<model>.json.
"""
import copy, json, os, sys, warnings
warnings.filterwarnings("ignore")
from mistral_common.tokens.tokenizers.mistral import MistralTokenizer
from mistral_common.protocol.instruct.request import ChatCompletionRequest
from mistral_common.protocol.instruct.validator import ValidationMode
from transformers.integrations.mistral.tokenizer import convert_tekken_tokenizer

HERE = os.path.dirname(os.path.abspath(__file__))
MODELS = ["mistralai__Mistral-Small-3.2-24B-Instruct-2506", "mistralai__Magistral-Small-2509"]


def main():
    set_name = sys.argv[1] if len(sys.argv) > 1 else "dev"
    cases = json.load(open(os.path.join(HERE, f"cases_{set_name}.json")))
    outd = os.path.join(HERE, "out", set_name)
    os.makedirs(outd, exist_ok=True)
    for name in MODELS:
        tk = os.path.join(HERE, "models", name, "tekken.json")
        toks = {m: MistralTokenizer.from_file(tk, mode=m) for m in (ValidationMode.test, ValidationMode.finetuning)}
        hf = convert_tekken_tokenizer(tk)
        tdir = os.path.join(HERE, "tokenizers")
        open(os.path.join(tdir, name + ".json"), "w").write(hf.backend_tokenizer.to_str())
        json.dump(hf.special_tokens_map, open(os.path.join(tdir, name + ".special.json"), "w"), ensure_ascii=False)
        res = {}
        for c in cases:
            try:
                msgs = copy.deepcopy(c["messages"])
                mode = ValidationMode.finetuning if msgs and msgs[-1]["role"] == "assistant" else ValidationMode.test
                req = ChatCompletionRequest.from_openai(msgs, tools=copy.deepcopy(c["tools"]))
                enc = toks[mode].encode_chat_completion(req)
                res[c["id"]] = {"ids": list(enc.tokens), "text": enc.text}
            except Exception as e:
                res[c["id"]] = {"error": f"{type(e).__name__}: {str(e)[:300]}"}
        json.dump(res, open(os.path.join(outd, name + ".json"), "w"), ensure_ascii=False)
        print(f"OK {name}: {len(res)} cases, {sum('error' in v for v in res.values())} encoder errors")


if __name__ == "__main__":
    main()
