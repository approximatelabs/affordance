# Reading model replies

Contract authority: [public contracts](../spec/CONTRACTS.md),
[language](../spec/LANGUAGE.md) and [harness](../spec/HARNESS.md).

Rendering turns state into a model's tokens; reply parsing turns the model's tokens back into
thinking, content and tool calls. Every chat view in `views/index.json` has a row in
`views/reply_protocols.json` that names its reply syntax, the tokens that end a reply, and whether
the generation prompt ends inside an open thinking block. The parsers live in `src/reply.rs`
(`Syntax`, `parse_reply_typed`, `Problem`) and `src/reply_formats.rs`.

| Syntax | Families (entry views) | Tool calls | Thinking |
|---|---|---|---|
| `hermes` | Qwen2.5, QwQ, MiMo-7B, Qwen3 (+ Thinking/Instruct-2507), Kolibri | `<tool_call>\n{"name", "arguments"}\n</tool_call>` | `<think>` |
| `xml` | Qwen3.5/3.6/3.8, Qwen3-Coder, MiMo V2/V2.5/V2.5-Pro/V2.6 | `<tool_call><function=x><parameter=k>v</parameter></function></tool_call>` (MiMo V2.6 also `<function=x>{json}</function>`) | `<think>` |
| `glm` | GLM-4.5 … GLM-5.3 | `<tool_call>x<arg_key>k</arg_key><arg_value>v</arg_value></tool_call>` | `<think>` |
| `deepseek_v3` | DeepSeek V2.5, V3, V3-0324, R1, R1-0528 | `<｜tool▁call▁begin｜>function<｜tool▁sep｜>x` + a ```` ```json ```` block | `<think>` |
| `deepseek_v31` | DeepSeek V3.1 | `<｜tool▁call▁begin｜>x<｜tool▁sep｜>{json}<｜tool▁call▁end｜>` | `<think>` |
| `dsml` | DeepSeek V3.2, V4, V4.1 | `<｜DSML｜invoke name="x"><｜DSML｜parameter name="k" string="true">v</…>` | `<think>` |
| `llama31` | Llama 3.1, 3.2 | `{"name": "x", "parameters": {…}}` (one call) | none |
| `llama4` | Llama 4 | `[x(k="v"), y(k=1)]` | none |
| `mistral_list` | Mistral 7B v0.3, Nemo | `[TOOL_CALLS] [{"name", "arguments"}, …]` | none |
| `mistral_args` | Ministral 3, Mistral Small 3.2 / 4, Medium 3.5, Magistral | `[TOOL_CALLS]x[CALL_ID]id[ARGS]{json}` | `[THINK]…[/THINK]` |
| `harmony` | gpt-oss | `<|start|>assistant to=functions.x<|channel|>commentary json<|message|>{json}` | `analysis` channel |
| `atem` | Muse Glimmer | `<|start|>assistant to=x<|message|><atem:function_calls>…` | `to=self` messages |
| none | DeepSeek LLM, Llama 2, Llama 3, Qwen2 (no tools); GLM-4 classic (`glm-4-9b-chat`, GLM-4-0414, GLM-Z1-0414: the model's call format is not confirmed from the references) | | |

A model bound by name whose format has no way to write a tool call gets the syntax `text`: its
reply is prose, with any `<think>` block split off, and the loop ends the agent with that prose
as its answer.

Every parser reports the same `Problem`s: `no_action`, `multiple_actions`, `prose_before_action`,
`text_after_action`, `unclosed_think`, `bad_syntax`, `unknown_action`, `bad_args`, `cut_off`. A reply
cut off before its thinking closes is thinking (`unclosed_think` + `cut_off`), never content; a
call cut off mid-way is never returned. Syntaxes whose values arrive as text (`xml`, `glm`,
`atem`, `llama4`) are typed by the action's JSON schema (`"3"` → `3`; Python literals such as
`{'a': True}` are read too).

Decode replies with special tokens kept (`decode(ids, false)`): several families write their
markers as special tokens (Mistral `[TOOL_CALLS]`, gpt-oss channels, Glimmer headers, Llama
`<|python_tag|>`, DeepSeek V3.2 `｜DSML｜`).

## A format no parser was written for

A model bound by its registered name (`docs/MODEL_BINDINGS.md`) may write tool calls in a shape
none of the parsers above reads. For those, the reader learns the shape from the format itself
(`syntax: "pattern"`, `src/reply_pattern.rs`).

The model's chat template writes an assistant turn's calls into a prompt, and the model was
trained to write its own calls the same way. So the template app (`views/template_apps/chat.tsx`)
asks the template to write calls whose name, argument keys, argument values and id are strings
that occur nowhere else: one call with one argument, one with two, one with a number, one with
none, and two calls. Where each piece lands is known, and what lies between the pieces is the
format's own text. That gives a pattern: the text before the calls, one call (literal text with a
hole for the name, and the arguments either as one JSON object or as key and value pairs in the
format's markup), the text between calls, and the text after them. A reply is matched against
it, ignoring differences in white space; pair values are text and are typed by the tool's schema.

`tools/protocol_survey.rs` tries this on every registered model whose calls no parser reads and
keeps it only where a real call, written by the template, is read back as the same tool and
arguments. `tests/any_model.rs` checks each of those models through the app.

## Adding a family

1. Write the view (`views/<family>/…`) and add it to `views/index.json` (conformance as usual).
2. Add its row to `views/reply_protocols.json`: `syntax` (an existing one, or `null` with a
   `reason`), `stop` (the token strings that end a reply), `in_think` (does the default generation
   prompt end inside an open thinking block?).
3. If the format is new, add a `Syntax` variant (`src/reply.rs`, name in `Syntax::NAMES` /
   `Syntax::parse`) and its parser in `src/reply_formats.rs` returning `Parts`.
4. Run `cargo test --release --test suite reply_coverage::`. It fails for a view without a row, and it
   round-trips the parser through the view: it renders a conversation without and with an
   assistant turn holding tool calls (one call, two calls, string arguments, with reasoning) and
   parses what the template wrote for the model's turn. Where history is rendered differently from
   a fresh generation, the calls are checked in their history form. A rendering that cannot be
   read back for a template-specific reason goes in the test's `KNOWN_LOSSY` list with that reason.
5. To run the family as an agent, its template app (`views/template_apps/`) declares the same
   protocol in a zero-token `<span label="protocol" syntax=… stop={[…]} in_think={…}/>` (checked
   against the table by `reply_coverage`), passes the family's special tokens (`bos_token`, …), and
   calls `templateInput(children, system, true, {alternate})`.
   - **Generated turns.** The view renders a `generated` assistant turn (`m.ids`, with the turn's
     calls as `tool_calls` metadata) as exactly what its generation prompt was at that point, then
     `tokens(m.ids)`, then the turn end only if the ids lack it. It updates its own state as after
     an assistant turn (DeepSeek's flags after tool output, gpt-oss's last tool). Every chat view
     does this. `m.ids` is a token run that reads like a list of ids (HARNESS_GUIDE section 5).
   - **Laid-out content.** A screen is a fragment: string operations on content (`.includes`,
     `.split`, iterating it as parts) must let fragments through as text.
   - **`alternate`.** For a format that requires alternating roles (Llama 2, Mistral v0.3), or
     places tools by the last user turn (Mistral Small 3.2): a user turn right after another user
     turn or a tool result joins it.
   - **`system` null.** For a format that moves the system prompt every turn (Mistral v0.3 puts it
     in the final user turn): no system turn.

   `tests/template_sweep.rs` runs every chat view this way, through a real agent, for three turns
   (a tool loop, or a text loop for a family without tool calls). It checks that each reply parses,
   that each prompt is an exact append of the last plus the reply, and that the turn boundary after
   a reply is the template's own (no doubled or missing stop). It reports the configuration each
   family needs.
