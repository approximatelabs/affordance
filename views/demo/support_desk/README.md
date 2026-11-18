# A support desk, for a chat model or a decision model

Customer messages arrive one at a time, and the agent sends each to the team that handles it. Each
team is a tool (`<Tool fn={orders}/>`). The app names no model: `ModelContext` renders it in the
format of the binding's template.

- With a chat model's template (`qwen3`), the tools are function definitions and the model calls one
  in JSON.
- With `d1` (LiquidAI's d1-3B, a decision model), the screen is d1's state and the tools are its
  options, in d1's own prompt format. The model answers with one option code, which calls that tool.

For d1, serve the model with vLLM and restrict its next token to the option codes, so a greedy
token is d1's choice:

```yaml
model:
  kind: completions
  url: http://127.0.0.1:8000
  name: d1
  tokenizer: path/to/d1-3B/tokenizer.json
  template: d1
  sampling: {temperature: 0}
  provider_options: {allowed_token_ids: [...]}   # the ids of "A".."I", " A".." I" and <|im_end|>
agents:
  - id: desk
    max_new: 2
    props: {tickets: ["I want my money back for order 123", "..."]}
```
