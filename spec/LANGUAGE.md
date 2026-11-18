# The view language

Views are TypeScript/JSX files (`*.tsx`) implementing the JavaScript subset specified here.
Intentional differences are in §7.3; unsupported features are in §7.4. Everything here is implemented in `src/` (`tsx.rs`,
`parser.rs`, `interp.rs`, `jsvalue.rs`) and exercised by `views/`, including the
conformance views that reproduce 740 models' official chat templates token for token.

## 1. What a view is

A view is a program that turns **state** (a JSON object) into a **list of token ids** for one
model's tokenizer, plus **spans** (labelled token ranges, affordances, token-budget boxes).
It never builds the prompt as one string. Special tokens are emitted as ids; text is emitted in
*runs* that are encoded with the model's tokenizer (section 6).

```
state (JSON) ──► view program ──► [ids] + [spans]
                     ▲
              bound to a tokenizer at load time
```

For chat templates the state is the object the official template gets: `messages`, `tools`,
the kwargs (`add_generation_prompt`, `enable_thinking`, ...) and the tokenizer's special-token
strings (`bos_token`, `eos_token`, ...).

## 2. Files

The bound `tok` namespace is immutable from creation.

Global `fetch`, `Headers` and buffered `Response` are supplied by the embedded Web API
standard library. Fetch runs in effects/handlers through an explicitly bound HTTP executor;
it grants no network access by itself. See `docs/HTTP_AND_MCP.md` for the supported Fetch
subset, named capabilities and standard-library behavior.

```tsx
"use strict_tokens";                    // directives (6.3, 6.4)
import { pythonTypeName } from "../common/checks.tsx"; // explicit file binding
import "./chat.tss";                    // a token style sheet (4.1)

const NAME = expr;                      // evaluated once per render, before the root
function name(a, b = 1, ...rest) { statements }
const Turn = ({ role, children }) => p`${tok.im_start}${role}\n${children}${tok.im_end}\n`;
export default function Main({ messages, tools }) { return <>...</>; }
```

The root is a function named by the caller (e.g. `Qwen3`); it receives the whole state as its
props object. Declarations are private to their file; imports bind explicit exports (§2.1).
TypeScript syntax is parsed and erased: annotations of any shape
(object, union, tuple, function, conditional and template-literal types), optional parameters
(`x?: T`), `type` and `interface` declarations (exported or not), type parameters on functions
and arrows (`<T,>(x: T) => ..` in TSX), type arguments at calls (`useState<string[]>([])`),
`as` / `satisfies` casts and the non-null `x!`. Comments: `//`, `/* */`, and `{/* ... */}` in JSX.

The Rust `View::render` API accepts arbitrary app state. `View::render_chat_opts` adds the
Hugging Face chat wrapper's tool-schema validation and iterable-to-list conversion before
rendering a chat view. Validation is outside the core language; an app may use a state field
named `tools` for its own data. A JSON integer larger than 2^53 is a number (as in JavaScript)
that keeps its exact decimal digits when printed; arbitrary-precision arithmetic is not
implemented.

### 2.1 File modules

```tsx
import MainView from "./main_view.tsx";
import { Tool, lookup as find } from "./tools.tsx";
import DefaultView, { helper } from "./views.tsx";
import "./setup.tsx";                  // load/evaluate; introduces no names
import type { Input } from "./types.tsx"; // erased; no runtime dependency
export function Widget(props) { return <MainView {...props}/>; }
export { find as search };
export default function Main(props) { return <Widget {...props}/>; }
```

Functions, constants and classes are private unless exported. Supported exports are
`export function`, `export const`/`let`, `export class`, named default declarations and
`export {local as public}` (including `as default`). A default export does not also create
a named export. An entry selects a function declared in its root file, falling back to
that file's default export when the requested declaration is absent.

Relative paths resolve from the importing file; absolute paths are also accepted. A path that
starts with `@affordance/` names a file of the library (`@affordance/lib/log.tsx`), as an import map
names a package: the deployment's `library` directory, or the copy bundled with Affordance. Source
files are `.tsx`, styles are `.tss`. Canonical file identity deduplicates a dependency in a
program, including diamond imports. Constants initialize dependency first, in source order;
a read of an uninitialized module constant throws a ReferenceError. Module constants are
frozen as described in §7; modules do not provide mutable persistent state. Startup
initialization still happens once per render. Declared functions keep compiled-declaration
identity across renders for equality, Map/Set keys and effect dependencies, while retained
functions execute against their defining initialization's lexical snapshot. Each runtime
source load creates fresh declarations (§2.2).

Named/default imports introduce only their listed local bindings. Export aliases preserve
the identity of the referenced value. Functions, parameter defaults, closures, class methods
and tool type-schema lookup resolve names in their defining file, including during effect
replay. Imported JSX components and mounts resolve the same function; a mounted child's entry
uses its declared function name. Render inputs, builtins and platform globals remain available
unless shadowed by lexical bindings. Style sheets retain the global cascade described below.

Duplicate declarations/import bindings/exports, missing exports and exports of undefined
bindings are load errors. Cyclic imports, namespace imports (`import * as ...`), re-exports
with `from`, dynamic imports and package resolution are unsupported and fail explicitly.
Type-only imports are erased; cross-file type aliases are not available to `schemaOf`.
The built-in module `affordance:web` exports `fetch`, `Headers`, `Response`, `createFetch`,
`AbortController` and `AbortSignal`; its implementation's private names are inaccessible to apps.
All but `createFetch` are also shadowable platform globals (abort signals: `spec/HARNESS.md` §3). `createFetch` selects an explicitly bound
HTTP role; see [HTTP_AND_MCP.md](../docs/HTTP_AND_MCP.md).

### 2.2 Runtime source modules

`loadModule(source, {imports?})` from `affordance:compiler` returns a frozen export namespace
using the same parser and executor as file modules. `imports` maps exact import specifiers to
objects containing named/default exports; installed `affordance:` modules resolve automatically.
Loading performs no filesystem/network lookup. Each load creates a distinct lexical instance;
functions, defaults, closures and type schemas retain it across later calls and awaits.

```tsx
import {loadModule} from 'affordance:compiler';
// In a handler, then retain exports in state:
const next = loadModule(source, {imports: {'./helpers.tsx': {format}}});
// In the parent component:
// <next.default key={revision}/>
```

Acquire source in recoverable app inputs and keep exports in state rather than loading on every
render. A lazy state initializer may load initial source. Source parse/initialization errors
are catchable before selecting a revision. The app owns revision history and controls; a new
`key` resets child-local state, and undo selects old code with fresh local state after removal.
Source is charged to execution fuel. Global fuel exhaustion aborts the render, including its
parent; source modules share the instance's capabilities and computation allowance.

Initialization is synchronous and pure for both file and source modules: no hooks, async calls,
await, Promise reactions, host effects, interactive JSX, scroll viewports or mounts. Put those
inside component/function bodies. Passive text/token constants and class definitions are valid.
Initializers cannot see the loader caller's locals; documented app render inputs and platform
globals remain available. Imported values are deep-frozen, including captured closure cells;
module `let` is read-only just like file-module constants.

Runtime modules use the bound tokenizer, parent styles and document policy. Style-file imports
are unsupported; use inline styles or the parent's existing stylesheet. `strict_tokens` applies
normally; `safe_text` must already be selected by the parent. Unknown directives/options and
missing imports/exports fail explicitly. `js` remains a no-op. `<Mount>` needs a file-backed
entry; a file-backed wrapper can load source from its props to run a separate instance.
See the [live-component example](../views/demo/live_components.tsx).

## 3. JSX and prompt text

Components are functions that return a value; elements are values. Children follow React's
rules: strings as text, numbers as JavaScript prints them, arrays spliced (each element is one
item for a dropping box, 4.1), `true`/`false`/`null`/`undefined` render nothing, a plain object
is an error.

Component tags resolve function values: `<Panel/>` may use a local, prop or closure, and
`<ui.Panel/>` reads an object member. Intrinsic spellings (`Mount`, `tok.*` and the lowercase
elements below) remain reserved. A missing or non-callable component is an error; a string
value does not name an intrinsic element.

Elements have structural paths. An explicit string/number `key` identifies a sibling; duplicate
sibling keys are errors. Otherwise identity uses the JSX tag spelling and its evaluated
sibling position among that tag. Arrays splice in order; null/false/undefined create no
element. Hooks use that path and hook order. `id` and `label` select/describe presentation and
do not reset hooks. Fresh function values at the same path preserve state; this is different
from React's function-object identity. Removing a component discards its local state and
cleans up its effects; named `useState(init, "name")` fields remain instance-wide.

| syntax | meaning |
|---|---|
| `` p`text ${x} more` `` | a **prompt fragment**: literal text (split on special tokens at load time) and values in order |
| `` `text ${x}` `` | a plain string (JavaScript template literal) |
| `{expr}` in JSX | a child (rules above) |
| `<Name a={expr} b="str" c>children</Name>` | call a function component with a props object (`children` is a fragment) |
| `<tok.im_start/>`, `${tok.im_start}` | one special token (6.4) |
| `<sp t={expr}/>`, `<tok id={n}/>` | a special token chosen at render time; a raw token id |
| `<span label="x">..</span>` | labelled span (token range is reported); extra props travel as meta |
| `<aff label="x" onGo={..}>..</aff>` | an **affordance**: an action attached to the tokens of its children (section 5) |
| `<box max_tokens={n} ...>..</box>` | token-budget box (section 4) |
| `<img src={dataUrl} t="<\|image_pad\|>" metrics={..} alt=".."/>` | a picture sized by layout (4.1): the tokens its model's processor gives it within `max-tokens`; the request carries the file at that size |
| `<img tok_size={n} t="<\|image_pad\|>" src={dataUrl}/>` | `n` placeholder tokens for an image, the file as it is |
| `<audio src={dataUrl} t=".." metrics={..} alt=".."/>` | a sound clip sized by layout (4.1): trimmed to `max-tokens` |
| `<audio tok_size={n} t=".." src={dataUrl}/>`, `<video tok_size={n} t=".." src={dataUrl}/>` | `n` placeholder tokens for an audio clip and a video clip, the file as it is |
| `<text>..</text>`, `<>..</>` | grouping only |

Any other lowercase tag is an error: prompt text such as `<think>` or `</tool_call>` is written
inside `` p`...` `` or a string, never as JSX. JSX text follows React's whitespace rule (a
whitespace run containing a newline is dropped), so prompt layout is written in `` p`...` ``.

## 4. Token-aware styles

Style properties are measured in tokens of the bound tokenizer.

| property | element | meaning |
|---|---|---|
| `max_tokens={n}` | `box` | content may use at most `n` tokens; overflow is clipped |
| `tok_size={n}` | `box` | clip to `n` tokens; an explicit `pad` fills shorter content to `n` |
| `tok_size={n}` | `img` | exactly `n` placeholder tokens |
| `overflow="clip-end"` (default), `"clip-start"`, `"scroll"`, `"drop-start"`, `"drop-end"` | `box` | clipping, token scrolling, or whole-item dropping (4.1) |
| `more="…"` | `box` | marker text placed at the cut, counted inside the budget |
| `pad="<\|pad\|>"` | `box` | padding token for `tok_size` |

A box is a **hard tokenization boundary**: its content is encoded on its own, so clipping is
exact and never splits a merged token. The box reports `content_tokens`, `hidden_tokens` and
`overflow` in its span, so a view (or the agent) knows exactly what was hidden.

### 4.1 Token style sheets (`.tss`)

A view can import a style sheet (`import "./chat.tss";`). Rules are CSS-like with token units:

```css
ToolOutput            { max-tokens: 32tok; more: "\n[... truncated]"; }
ToolOutput[full]      { max-tokens: 4096tok; }      /* more specific: wins */
Reasoning[old]        { display: none; }            /* renders nothing, costs nothing */
#scratch              { tok-size: 16tok; }
```

- Selectors apply to both component wrappers and native elements (`box`, `span`, etc.):
  `Component` (the JSX tag spelling, including member tags), `*`, `#label` (the `id` or `label` prop), `[prop]` (truthy), `[prop="v"]`,
  and compounds of these (`Turn[role="tool"]`), and descendant combinators: `A B` matches a B
  inside an element matching A (`Context[policy="trunc"] history`), so an app styles what a
  template it renders writes, by its own props.
- Specificity: inline `style={{...}}` > `#id` > `[prop]` > element/component name;
  later rules win ties. Native presentation attributes such as `max_tokens` are defaults
  below sheet rules. No token property inherits. Inline property names accept camelCase or
  kebab-case; direct native presentation attributes retain snake_case.
- Properties: `max-tokens`, `min-tokens` and `object-fit` (pictures), `tok-size`, `overflow: clip-end | clip-start | clip-middle | scroll | drop-start | drop-end | drop-children`,
  `drop-step`, `more`, `pad`, `display: none | block | contents`. A token size or budget creates a token box (4)
  around the element/component result. A native `box` always provides one box, never an
  extra box for the same style. `display: none` emits no tokens, presentation spans or model-callable
  actions and adds no encoding boundary, while its components, hooks, effects, event listeners
  and child instances stay mounted. Conditional removal, not CSS visibility, unmounts them.
  Invalid property names/values fail explicitly; `null`/`undefined` inline values are omitted.
  Nested boxes apply the child's limit first, then the parent's overflow policy; they do not
  divide the budget equally among siblings.
- `overflow: drop-start | drop-end` drops **whole items** instead of cutting tokens: the box
  drops items from one side until the rest plus the `more` marker fits, keeps at least one
  item (clipping it only if it alone is too big), and reports `dropped_items`. This is the
  history budget an agent needs: older messages disappear whole, structure stays valid.
  Items are the elements of an array: the list a styled component returns
  (`messages.map(m => <Turn .../>)` with `overflow: drop-start` in a `.tss` rule or inline
  `style`), or an array child of a `<box overflow="drop-start">` (`{xs.map(x => ...)}`).
- `max-tokens` is a token count (`1500tok`, or a number) or a share of the prompt's **window**
  (`80win`: 80% of it). The window is the model's context (the binding's `context`) less the
  reply's room (`max_new`; an ad-hoc `model(<View/>, {max_new})` uses its own). With no window
  (the binding states no `context`) a share caps nothing, as a page with no viewport is not
  clipped; set `context` on the binding. A prompt that does not fit the context with its reply is
  never sent: the call fails with a `ContextError` naming the largest boxes.
- `drop-step: 25%` (of the box's cap) or `drop-step: 400tok` makes a drop **stable**: the amount
  dropped is rounded up to a multiple of the step, and the cut is the first item boundary at or
  past it. Content that grows on the far side (a history) then moves the cut once per step of
  new tokens, not on every new item, so the kept prefix (and a server's prefix cache) survives
  many turns. A step leaves at least `cap - step` tokens of items.
- `overflow: clip-middle` keeps the head and the tail (the head gets the odd token) with the
  `more` marker between them. In `more`, `{hidden}` is the number of hidden tokens and `{id}`
  the box's `id`/`label`: `ToolResult { max-tokens: 1500tok; overflow: clip-middle;
  more: "\n[... {hidden} tokens hidden, read_event {id}]\n" }`.
- **Graded results:** `result:nth-last(n+K / S)` matches a tool result at least K from the end
  among the results of its history (in document order; the current turn's are not counted),
  the boundary moving S results at a time, so a step rewrites S results once every S appended.
  It counts as an attribute in specificity. With `overflow: elide` the matched result shows its
  `more` marker alone: `result:nth-last(n+20 / 10) { max-tokens: 64tok; overflow: elide; more:
  "[output elided: {hidden} tokens; read_event(\"{id}\")]"; }`. Results are boxed only when a
  rule applies to them apart from position; a history drop measures graded results at their
  static length, so grading never moves its cut.
- After a page prompt is laid out for a `model()` call, a `layout` event reports what its boxes
  did, as `ResizeObserver` reports sizes: `useEvent("layout", f => ...)` gets `{prompt_tokens,
  window, context, max_new, boxes: [{label, tokens, content_tokens, hidden_tokens,
  dropped_items}]}`. It is delivered in the same input, only to an app that listens. An app
  schedules a fold or a summary from it; nothing measures during render.
- Item boundaries are marks, not tokenization boundaries: a drop box that fits encodes exactly
  as its content would without it. A cut that would fall inside a token two items share is an
  error.
- Template apps mark what they write: `<turn role=...>` around each protocol turn (a unit a
  drop removes whole: a message, a tool group, a GLM assistant with its observation block, a
  DeepSeek exchange) and `<history>` around the run of history turns, between the system turn
  and the current screen (`<turn current>`). Both are transparent label spans. With a rule they
  are boxes whose items are their children: `history { max-tokens: 60win; overflow:
  drop-start; drop-step: 25%; }` drops whole turns from the start and leaves the system turn
  and the screen as the template wrote them. For a hosted API (the `messages` template) layout
  runs before serialization: the same rule drops whole messages (a call with all its results is
  one unit), measured with the bound tokenizer (approximate; the provider's count differs), and
  the window check counts the JSON that is sent. When what is left no longer starts with a user
  message (an agent's one user message was the oldest), a user message stands where the drop
  was: the rule's `more` text, or a default sentence saying earlier messages were dropped.
- **A picture is sized in tokens (4.1).** A model's processor turns a picture into tokens by a
  rule that is the model's: its *metrics*, which a template app passes to `<img metrics=..>`
  (`views/index.json`, `agent.media_metrics`; an app author does not write them). `grid`
  metrics cut the picture into cells of `cell` by `cell` pixels, one token each, for a picture
  whose area is within `min` to `max` pixels; `fixed` metrics give every picture the same
  count. On such an element `max-tokens` is the most tokens the picture takes (a count or a
  share of the window), `min-tokens` the fewest, and `object-fit` how it meets them:
  `scale-down` (the default) shrinks it to fit and never enlarges it past its natural size,
  `contain` shrinks or enlarges it to the largest size that fits, `none` keeps its natural size.
  The natural size is the picture's own, brought onto the grid and inside the processor's
  bounds. Layout holds one placeholder for each token, so boxes, drops and the window check
  count the picture at what the model reads. The request carries the file resampled to the
  laid-out size (PNG and JPEG are read; the result is a PNG, the same bytes on every machine),
  and a server that expands a placeholder itself is sent one. A picture that cannot fit (the
  cap is under the processor's least, or `object-fit: none` and it is over the cap) renders
  its `alt` text in its place and its file is not sent. `img { max-tokens: 256tok; }`,
  `img { max-tokens: 2win; }`. In a registered model's format the
  newest picture carries `last`, so the newest can have more room than the ones before it:
  `img { max-tokens: 64tok; } img[last] { max-tokens: 1024tok; }`.
- **A sound clip is sized in tokens too.** `rate` metrics give a clip one token for each `ms`
  milliseconds (the last for what is left), plus `extra` around it, and at most `most`. On
  `<audio metrics=..>` a `max-tokens` under the clip's size trims it: the start is kept
  (`overflow: clip-end`, the default) or the end (`overflow: clip-start`), and the request
  carries the trimmed clip. A PCM WAV file is read; another format is an error that names WAV.
  A cap with no room for one token renders `alt`. `audio { max-tokens: 250tok; }`.
- **A video is shown as its frames** by `frames(shots, {seconds_apart, max_frames})`
  (`views/lib/media.tsx`): message content with each frame as a picture and the time it was
  taken before it, `max_frames` of them spread from the first to the last. Each frame is an
  `<img>`, so `img { max-tokens: 64tok; }` sizes it and the video takes at most `max_frames`
  times that. `<video>` is the model's own video format. With `frames` metrics
  (Qwen3-VL's design) layout holds what the processor writes: for every two frames it takes,
  their time (`<1.4 seconds>`), an opening token, one token a cell and a closing token. The
  file goes as it is; over its `max-tokens` it is not shown.
- **No rule, no guess.** A model's size rules are part of its tokenizer (`Encoder.media`: read
  from the processor files beside `tokenizer.json`, or given to the element as `metrics` from
  the registry). An `<img>`, `<audio>` or `<video>` with no `tok_size` is laid out by the rule
  for its kind. With no rule, or a file the rule cannot measure, its `alt` text is laid out in
  its place (`[picture not shown]` when it has none), its span says why (`not_shown`), and its
  file is not sent. `shape` metrics (Gemma 4) give a picture a count that follows its
  proportions; such a picture, like a `fixed` one, is shown or not, never resized.
- `overflow: drop-children` drops oldest whole items and may retain zero items.
  Unlike `drop-start`, it never clips a final oversized item. This keeps complete
  chat exchanges intact even with a zero budget. The marker is clipped if needed
  to fit the budget. Hidden actions disappear; retained spans use final token positions.

### 4.2 Scroll viewports

`overflow: scroll` with a token budget/size creates an executable token viewport on either a
native element or a component wrapper. Its offset starts at zero and is keyed by element
identity, independently of application hook order. Layout slices already measured token IDs;
it never decodes and re-encodes the window. The optional `more` marker is counted inside the
budget and appears at the end when content is omitted. Hidden content has zero extent.

When the content exceeds a nonzero viewport capacity, one `scroll` action accepts
`{by: integer}` in token positions. Its metadata includes `scroll_offset`, `scroll_max` and
`content_tokens`. Dispatch clamps against the captured view's bounds and applies movement to
the live offset; the next layout re-clamps if the content changed. A resize retains the stored
requested position without a render-time write; the next scroll action commits its clamped
position. Fitting content, a zero
budget, or a marker consuming the whole budget publishes no automatic scroll action.

An `onScroll` observer receives `{by, offset}` after the platform write, with `by` equal to the
applied delta. The default action result is the new offset. Synchronous observer writes and
the offset update share one action prefix; async observer work follows normal task semantics.
Ordinary `onX` actions, including app-defined scrolling over a data source, remain available
without a platform viewport. Explicit labels/IDs identify model action targets; otherwise the
structural path does. Ancestor clipping removes partly clipped scroll actions. Historical
snapshots retain viewport output while dropping executable handlers as for other actions.

## 5. Affordances

`<aff label="..." onX={handler} ...>children</aff>` (or `onX` on a `<box>` or `<span>`) marks
the tokens of `children` as the target of an action. Other attributes are copied into the span's
`meta`. The renderer returns, for each affordance, its name, arguments and token range
`[start, end)`. Example (`views/demo/scroll_list.tsx`):

```tsx
const NumberList = ({ numbers, offset, budget, onScroll }) =>
  <box label="numbers" style={{ maxTokens: budget, more: "\n[... more below]" }} onScroll={onScroll}>
    {numbers.slice(offset).map(n => p`${n}\n`)}
  </box>;
```

The tool list is therefore part of the view: an action exists exactly where it is rendered.

An `aff` element is a **hard tokenization boundary** (its content is encoded on its own), so an
action always owns whole tokens. Found in testing: inside one run, `[scroll up][scroll down]`
encodes `][` as one token, so a soft label cannot start cleanly. Plain `<span>` labels stay soft
(they do not change tokenization); a `span` whose boundary falls inside a merged token is reported
with `straddle: true`.

### 5.1 Native apps as chat tools

`asChatTools(children)` lays out a native app and returns `{ tree, tools }`.
`tree` retains native token IDs and visible spans. Emit it as a child inside a
model wrapper; do not convert it to text. Model-callable `on*` actions become
function schemas with names such as `scroll__numbers`, retaining their original
handler references and hook state. Harness callbacks are excluded.

`asChatTools(children, {callable: false})` preserves the same tool schemas and token
content, but removes handler references from the returned tree and adds `callable:false`
metadata. Turn plans retain this restriction; hosted adapters disable provider calls. Use it for summaries,
previews or measurement without exposing model-callable actions. Derive aliases before
removing handlers, so names match an active projection of the same screen. This does not
disable hooks, effects or event listeners in the surrounding document.

Set `tool_name="shell"` for an exact tool name (otherwise `verb__label`, e.g. `run__shell`;
see `docs/HARNESS_GUIDE.md`). Use unique stable labels for stable tool names, and `args_schema` for explicit
JSON types or constraints. Otherwise destructured parameter names have
unconstrained JSON types and defaults make fields optional. A single object
parameter has an unconstrained object schema. Runtime argument validation is
the application's job. Apply budgets before projection; a later crop that
hides a projected action raises an error. Native trees form hard encoding
boundaries. `docs/MIDDLE_LAYER.md` shows an app written once and rendered for each model.

### 5.2 Replies

A model reply is optional thinking, then tool calls in the model's own format, then the end. The
template app's zero-token `protocol` span names the format (`syntax`), so the harness reads each
call as `(name, args)`, checks it against the rendered affordances and their `args_schema`, and runs
the named `on*` handler (`src/reply.rs`; the harness side is `spec/HARNESS.md` §3). A call to an
action that is not in the render, or with arguments that fail the schema, becomes an error result
the model sees. A reply with no call is allowed; whether the model is called again is the app's (`model()`).

The screen's actions stay callable however the template writes the screen, as a page keeps its
listeners whether it is shown, printed or read aloud. A template that writes the screen in place
carries them at their tokens. One that writes it in another form (d1's state text through
`textContent`, a hosted API's JSON) offers them still: the template app element lays out, zero-width
after its output, each action of the screen its `templateInput` read that the output does not carry.

A decision model writes no call: it reads a list of options and answers with one option code. Its
template app (`views/template_apps/d1.tsx`) lists the screen's tools as the options, and its protocol
span has `syntax="choice"` and `codes` (`{code: tool}`). A reply of one code calls that tool with no
arguments; any other reply is a syntax problem. Which code is most likely is the server's to work
out: restrict its next token to the codes (vLLM's `allowed_token_ids` in the binding's
`provider_options`) and sample greedily, which picks the same option as the model's published
readout.

## 6. Text and tokens

### 6.1 Runs
Consecutive text pieces form a **run**; a run ends at a special token, a box boundary, or the
end of the view. Each run is encoded once with the tokenizer (`add_special_tokens=false`) and
the ids are appended. Runs are cached by content, so re-rendering a conversation re-encodes only
new text.

Tokenizers with nonzero BPE dropout are rejected at load: stochastic encoding cannot satisfy
the exact-token contract. Dropout zero or null is accepted.

Why runs and not per-piece encoding: BPE merges and pre-tokenizer splits cross piece boundaries
(`"\n\n"` + `"Hello"`, `"user\n"` + `"\n..."`). Added tokens are hard boundaries in every
Hugging Face tokenizer, so per-run encoding gives exactly the ids of tokenizing the whole string,
provided two tokenizer rules are reproduced (both automatic):

1. **Position:** SentencePiece-style tokenizers with `prepend_scheme: first` add the `▁` prefix
   only to the text at the very start of the prompt. The renderer detects this at load time and
   encodes non-first runs accordingly.
2. **Added-token flags:** tokens with `lstrip`/`rstrip` swallow the whitespace next to them.

Dynamic text (`{expr}`) that contains an added-token string (e.g. `"<|im_start|>" + role`) is
split by the emitter at that token, exactly where the tokenizer's added vocabulary would split it
(leftmost-longest match; `lstrip`/`rstrip` honoured). The ids are unchanged, but each message then
becomes its own cached run. In `safe_text` mode this split does not happen (6.3).

Measured (boundary study, `affordance boundaries`): when each text piece is encoded on its own instead of
per run, 28% of dev renders and 33% of held-out-3 renders change. Piece boundaries that merge:
space|letter 100%, space|newline 100%, newline|newline 99.8%, punctuation|newline 85%,
space|punctuation 63%, punctuation|punctuation 26%; newline|letter and newline|digit never.

### 6.2 What is *not* a boundary
Two text pieces next to each other are one run even if they come from different components or
expressions. If the view wants a boundary without a token, it uses a `box` (or a span-free
`<box>` with no limit).

### 6.3 `"use safe_text"`
By default, text that contains a special-token string (a user typing `<|im_end|>`) is tokenized as
that special token, exactly like every Jinja chat template does (prompt injection by design).
With `"use safe_text";` all dynamic text is encoded with added-token matching off, whether the
tokenizer marks the token special or not (DeepSeek's `<｜User｜>`, GLM's `<tool_call>` and both
models' `<think>` are not special); special tokens can then only come from the view's own literal
text and `tok.*` values. This holds for every run, including runs after the start on a
position-dependent tokenizer (6.1 rule 1), and is the browser's `textContent`: text is never
markup. `mistral_common` (Mistral Small 3.2, Magistral) behaves this way, and those two views need
the directive to match it.

### 6.4 Special tokens are values, not text (`"use strict_tokens"`)
A special token is a value of its own kind, never a string:

- `tok.im_start`, `tok.think_end`, `tok.User` (alias of the token text: `<|x|>`, `<｜x｜>`,
  `[x]`, `<x>` give `x`; a leading `/` gives `x_end`), or `tok["<|im_start|>"]`.
- `<tok.im_start/>` as a JSX element, `${tok.im_start}` inside `` p`...` ``.
- `tok.x + "text"` gives a **token fragment** (tokens and text in order). Fragments render as
  their parts. `.trim()`, `.trimStart()`, `.trimEnd()` and `.join()` keep the tokens. Read-only
  questions (`startsWith`, `endsWith`, `includes`, `indexOf`, `lastIndexOf`) look at the text the
  fragment stands for. A fragment joins only with text: `fragment + [1]` is a TypeError (convert
  the value first with `String(x)` or `JSON.stringify(x)`), so a stray object never prints into a
  prompt. `isFragment(x)` tests for one.
- `tok.x.text` is the token's string and `tok.x.id` its id, for questions about data
  (`content.split(tok.think_end.text)`). Text from `.text` is still text when it is rendered.
- `tok.x` for a token the tokenizer does not have is an error, not empty text.
- `textOrToken("<tool_response>")` is the token if the tokenizer has it, else the same plain
  text. Use it only in a module shared by models that differ here (Qwen2.5 has no
  `<tool_response>` token, Qwen3 does; GLM-Z1 has no `<think>` token, GLM-4.5 does). This is the
  one place where a special-token string may appear in a strict file.

With `"use strict_tokens";` at the top of a file, any special-token string of the loaded
tokenizer written as text (in `` p`...` ``, a string, JSX text, or a template) is a load error
that names the `tok.` value to use. The hand-written reference views use it (the demos too). Data text that
contains a special-token string still becomes that token, as in the official templates, unless
the view also says `"use safe_text"` (6.3).

## 7. Expressions and statements

The semantics are JavaScript's (`src/jsvalue.rs`, `src/jsbuiltins.rs`, `src/jsmath.rs`).
`tests/js_semantics.rs` checks expressions against what JavaScript prints, and
`tests/js_differential.rs` runs a corpus (`tests/js_corpus/*.js`) and seeded random programs
(`tools/jsgen.mjs`) through both the views and Node, and requires the same printout. The few
differences left are listed in 7.3, and what is not supported fails with an error (7.4).

- Literals: numbers (`12`, `1.5`, `.5`, `1e3`, `0x1F`, `0o17`, `0b101`, `1_000`; an integer
  beyond 2^53 is the double JavaScript reads), `NaN`, `Infinity`, strings (`"s"`, `'s'`, template
  literals; escapes `\n \t \r \v \b \f \0 \xHH \uHHHH \u{H..}` and line continuations), `true
  false null undefined`, arrays, objects (`{k: v, "k2": v, 3: v, [expr]: v, short, f(x) {..},
  ...spread}`), regex literals (7.1), functions (`x => e`, `(a, b = a) => { ... }`, `async (a) =>
  e`, `({a, b}) => ..`, `([a, b], {c: d = 1}) => ..`, `function (x) { .. }`).
- Operators, with JavaScript's precedence: `+ - * / % **`, unary `- + ! ~ typeof void delete`,
  `++`/`--` (prefix and postfix), `== != === !==`, `< <= > >= in instanceof` (the built-in
  constructors: `Array`, `Object`, `Error` and its kinds, `RegExp`, `Function`), `& | ^ << >> >>>`,
  `&& || ??` (`??` does not mix with `&&`/`||` without parentheses, as in JavaScript), `c ? a :
  b`, `,`, assignment as an expression (`a = b = 0`, `x += 1`, `o.k ??= v`, all of `+= -= *= /= %=
  **= <<= >>= >>>= &= |= ^= &&= ||= ??=`), `x.k`, `x[i]`, `f(a)(b)`, optional chains (`x?.k`,
  `x?.[i]`, `x?.f()`, `f?.()`: a nullish link makes the whole chain `undefined`), `new Error(..)`,
  spread in arrays, objects, calls (`f(...xs)`) and element attributes (`<C {...props} b="z"/>`:
  a later attribute of the same name wins, as in JSX). An error stringifies as `Name: message`.
- `+` concatenates when either side is a string; `[1, 2] + ""` is `"1,2"`. Numbers are IEEE
  doubles (`7 / 2` is `3.5`, `-7 % 3` is `-1`, `0 * -1` is `-0`, `2 ** 53 + 1` is `2 ** 53`);
  `2.0` prints as `2`, and numbers print as JavaScript's `Number.prototype.toString`
  (`1e+21`, `1e-7`). `==` and `<` convert arrays and objects to their strings first.
- Truthiness is JavaScript's: `[]` and `{}` are truthy; `0`, `""`, `NaN`, `null`, `undefined`
  falsy. `===` compares objects and arrays by identity.
- `a[-1]` is `undefined` (use `a.at(-1)`); `a[1.0]` and `a["1"]` are `a[1]`. Reading a property
  of `null` or `undefined` is a TypeError, as is `for...of` over a non-iterable (an object,
  `undefined`). Object keys keep JavaScript's order: integer-like keys first, ascending, then
  the others in insertion order (objects from the host's JSON keep the host's order).
- Statements: `const`/`let`/`var` (with nested array and object patterns, defaults, holes,
  renames and `...rest`), blocks `{ .. }`, `function` declarations (hoisted to the top of their
  block), `if/else`, `for (const p of xs)`, `for (const k in obj)`, `for (init; cond; update)` (any
  parts, several declarators, `,` updates), `while`, `do..while`, labels on loops and blocks with
  `break label` / `continue label`, `switch` (with `===`), `try/catch/finally` (`catch` binds
  the thrown value itself; a failed operation binds an error (an instance of its class: `name`, `message`, `stack`) with JavaScript's
  error name), `throw`, `return` (a line break after `return` returns `undefined`), `break`,
  `continue`. Statements end at `;`, a line break, or `}` (automatic semicolon insertion), so
  `return x y` is a syntax error rather than two statements. A `const` cannot be reassigned
  (a TypeError, as in JavaScript). Loops are bounded at 10,000,000 iterations.
- Functions are closures: a function shares the variables it uses with the scope that declared
  them, so it sees later assignments, its own assignments are seen outside, and closures made
  together share state (`makeCounter()`); each iteration of `for (let ..)` has its own copy.
  A `let`/`const` read before its declaration ran is a ReferenceError in a block that declares a
  function (elsewhere see 7.3).
- Arrays and objects are references, as in JavaScript: `const b = a;
  b.push(1)` changes `a`; a function that changes its parameter changes the caller's object;
  `for..of` over an array sees what its body pushes. `Object.freeze(o)` (shallow),
  `Object.isFrozen(o)` and `structuredClone(v)` (a deep copy; shared parts stay shared, a cycle
  stays a cycle; a function is a `DataCloneError`) are JavaScript's.
- **`Map` and `Set`** are JavaScript's: `new Map(entries)`, `new Set(values)`, `get set has delete
  clear forEach size`, `keys values entries`, insertion order, SameValueZero keys (objects by
  identity, one `NaN`, `-0` is `0`), iteration (`for..of`, spread, `Array.from`, destructuring,
  `Object.fromEntries`) that sees entries added during it and skips those deleted before it
  reaches them, `Map.groupBy`, `instanceof`. `JSON.stringify` writes one as `{}`, as JavaScript
  does. A Map or Set in state is the runtime's: `set`/`add`/`delete` on it are a `TypeError` that
  says to copy it (`new Map(m)`). `keys()`, `values()` and `entries()` return arrays (JavaScript's
  return iterators: their `.next()` is not there, an error rather than a different value).
- **Errors** are instances of the built-in error classes: `message` and `stack` are own and not
  enumerable (`Object.keys(e)` is `[]`, `JSON.stringify(e)` is `"{}"`), `name` comes from the class,
  `e instanceof Error` holds for every kind (and for the runtime's `AbortError`, `TimeoutError`,
  `InvalidStateError`, ...); a failed operation throws one. A regex's `lastIndex` is JavaScript's:
  a global regex's `exec`/`test` start at it and move it, a literal makes a new RegExp each time
  it is evaluated, and a frozen (kept) global regex cannot `exec`.
- **`this` and classes** are JavaScript's. A `function`, a method (`{m() {..}}`, a class's) and
  an accessor bind `this` when called (`o.m()`'s receiver, else `undefined`); an arrow takes the
  enclosing one. `f.call(t, ..)`, `f.apply(t, [..])`, `f.bind(t, ..)`. `class A extends B {
  constructor(..) { super(..); this.x = .. } m() {} get g() {} set g(v) {} static s() {} static k =
  1; f = 1; #p = 1; #m() {} }`, `new`, `super(..)` and `super.m(..)`, `instanceof`, `extends` of a
  class or of a built-in error (`class NotFound extends Error {}`): fields initialize in
  JavaScript's order (a base class's before its constructor, a derived class's right after
  `super(..)` returns); methods and accessors live on the prototype (not own, not enumerable:
  `Object.keys`, spread and `JSON.stringify` see only the fields); private members are invisible
  to them and checked (`o.#x` on an object of another class is a `TypeError`). Private names
  belong to their lexical class; base and derived `#x` slots are distinct, and a public
  `o["#x"]` property cannot access either. Extracted methods retain the home class used by
  `super`, while their receiver still comes from the call. Static members
  are inherited. A module-level class definition has a stable type brand owned by its program,
  so an instance kept in state stays `instanceof` that definition on later renders. Class values
  and static initializers are evaluated afresh with each render's inputs, as module consts are;
  class objects are not shared between instances or renders. Like every module const the value
  is frozen (`A.count++` on a module-level class is a `TypeError`). A class created inside a
  function has fresh type identity each time it is evaluated. Replaying a task reconstructs
  that same evaluation and preserves its retained reference identities.
- **Native methods keep their defining identity.** Borrowing a method with `call`, `apply` or
  `bind` changes its receiver, not which operation runs. Array methods share their algorithms
  with object arraylikes and read items when visited; callbacks receive their supplied
  `thisArg`. Branded methods such as Map/Set access and RegExp execution reject incompatible
  receivers. Sparse array results remain unsupported and fail explicitly.
- **What the runtime keeps is frozen.** A value written to state (a setter's value, an updater's
  result, `useState`'s initial value), the props, module consts, `harness`, what `memoScan` keeps
  (its items, deps, init, and each step's state and out), a kept turn, and everything a closure
  kept past its run (a render's handlers and effects, an effect's cleanup) captured are frozen
  deeply. Writing to a frozen value is a `TypeError` (as writing to a frozen object is in strict
  mode); copy it to change it (`{...s, k: v}`, `[...xs, x]`, `structuredClone(s)`). A value that
  contains itself cannot be kept in state (a `TypeError`, as `JSON.stringify` refuses it). Cycles
  in a run's own objects (`o.self = o`, an object whose method refers to it) are freed when the
  run ends.

**Methods.** Strings: `trim trimStart trimEnd toLowerCase toUpperCase startsWith endsWith
includes indexOf lastIndexOf replace replaceAll split slice substring substr at charAt charCodeAt
codePointAt padStart padEnd repeat concat localeCompare match matchAll search toString`
(replacement patterns `$$ $& $\` $'` and replacer functions with string patterns too). Arrays:
`map filter find findIndex findLast findLastIndex some every includes indexOf lastIndexOf reduce
reduceRight forEach join concat slice at flat flatMap keys values entries toString push pop
shift unshift splice sort reverse fill toSorted toReversed toSpliced with` (`sort` is V8's
TimSort, so even an inconsistent comparator orders as in Node). Numbers: `toFixed` (JavaScript's
rounding of the exact value) and `toString(radix)`. Objects: `hasOwnProperty`, `toString`, and a
function-valued member is callable (`game.press(b)`).

**Globals.** `Math.*` (all functions and constants; `random` is an error: renders are
deterministic; use `await harness.random()` in a handler), `Object.keys/values/entries/
fromEntries/assign/is`, `JSON.stringify` (with a replacer function or key list, `toJSON`, and
a number or string indent) and `JSON.parse` (with a reviver), `Array.isArray/from/of`,
`Array(a, b, ..)` and `Array(n).fill(x)`, `Number.isInteger/isSafeInteger/isFinite/isNaN/parseInt/
parseFloat` and the `Number` constants, `String.fromCharCode/fromCodePoint`, `String() Number()
Boolean() parseInt parseFloat isNaN isFinite`, `Error`, `TypeError`, `RangeError`,
`SyntaxError`, `ReferenceError`, `URIError`, `EvalError`, `RegExp`. Built-in functions are
values too (`xs.map(Number)`, `xs.filter(Boolean)`, `xs.map(Math.abs)`).

Global intrinsics obey lexical bindings: local variables, parameters and imports named
`Math`, `JSON`, `Array`, `Number`, or `harness` take precedence over the platform's values.
This also applies to global constants such as `undefined`, `NaN` and `Infinity`, and includes
lexical declarations before their initializer runs (the temporal dead zone).
Supported intrinsic constructors and namespace methods are first-class values: aliases and
computed property reads use the same resolution as direct calls. This does not add unsupported
JavaScript APIs or sparse arrays; `Array(n).fill(x)` remains a supported dense construction.

**Affordance builtins.** `useState(init)` (a function initializer is called only when the hook mounts), `isFragment(x)`, `isPlainObject(x)`,
`countTokens(text)` (tokens under the bound tokenizer), `tokens(ids)` (exact ids as a fragment),
`textOrToken(s)` (6.4), `formatNow(fmt)` (the render clock, strftime format), `asChatTools`
(5.1), `turnStrategy`, `templateInput`, `messagesJson`, `asContent`, `textContent(content)` (the
text laid-out content stands for, like the browser's `Node.textContent`: for a template whose model
tokenizes one whole string), and `harness.*` in handlers
(`docs/RUNTIME.md`).

**The clock and the console.** `Date.now()` is the instance's clock in milliseconds: the journal's
in a handler (as `harness.now()`), so a replay reads the same time, and the render's in a render.
There is no `Date` object; `formatNow(fmt)` prints the clock. `console.log`, `info`, `warn`, `error`
and `debug` write a line for whoever watches the app, as a page's console is its DevTools view:
to the instance's trace (`{t: "console", level, text}`, which `affordance host` and an observer
also see) or, under `affordance render`, to stderr. Strings print as they are and other values as
JSON. A console line is never state and never journaled; a render writes its lines when it is
evaluated, not when a cached one is reused. A missing browser or Node global (`setTimeout`,
`process`, `window`, `require`, `localStorage`) is a ReferenceError that names what to use instead.

**Exact template text.** The official chat templates are Python; where they print a value, the
views print the same characters with explicit helpers:

| helper | prints |
|---|---|
| `formatJSON(x, {indent, separators: [item, key], ascii, sortKeys})` | Python `json.dumps` text (defaults `", "` / `": "`; `","` with an indent) |
| `formatPythonValue(x)` | Python `str(x)`: `True`, `None`, `1.0`, `{'a': 1}`; laid-out text (a fragment) is itself, tokens and spans kept |
| `formatPythonRepr(x)` | Python `repr(x)` |
| `trimChars(s, chars)`, `trimStartChars`, `trimEndChars` | strip any of `chars` from the ends |
| `capitalize(s)`, `titleCase(s)` | Python `str.capitalize()` / `str.title()` |

Where a template fails on a value of the wrong type, its view fails at the same point with an
explicit check: `views/common/checks.tsx` has `textOperand`, `mappingEntries`, `templateLength`,
`templateItems` and `templateTruthy` (Python's `str + x`, `.items()`, `|length`, `for` and
truthiness), each throwing `new TypeError(..)` where Python would.

An object that defines `toString` or `valueOf` (itself or in its class) is asked for its text or
its number wherever JavaScript asks: in a template (`` `${o}` ``), `String(o)`, `Number(o)`,
arithmetic, comparison and `==` with a primitive, a computed key (`m[o]`, `{[o]: 1}`, `o in m`,
`delete m[o]`), an array's `join` and text, and the arguments of string, number and `Math`
functions. Where text is wanted `toString` is asked first, elsewhere `valueOf`; if the answer is
an object the other is asked, and if neither gives a primitive it is a `TypeError`. The array
methods that take a position (`xs.slice(o)`, `xs.at(o)`) do not ask.

### 7.1 Regular expressions (`src/jsregex.rs`)

JavaScript regexes on the Rust `regex` crate. `/src/flags` literals (compiled at load time; `/` after an operand is
still division) and `new RegExp(src, flags)` / `RegExp(src, flags)` make a `RegExp` value with
`source flags global ignoreCase multiline dotAll unicode lastIndex`. Methods: `re.test(s)`,
`re.exec(s)`, and with a regex argument `s.match(re)` (all matches with `g`), `s.matchAll(re)` (an
array of match arrays; needs `g`), `s.search(re)`, `s.replace(re, str|fn)`, `s.replaceAll(re, ..)`
(needs `g`), `s.split(re, limit)` (captures spliced in). Replacement strings take `` $$ $& $` $' $n
$nn $<name> ``; a replacer function gets `(match, p1.., offset, string[, groups])`. Unmatched groups
are `undefined`; no match is `null`. Flags `g i m s u`. Semantics follow JS: `.` excludes all line
terminators unless `s`; `\d \w \b` are ASCII; `\s` is the JS whitespace set; `[]`/`[^]`; global
iteration steps one code point past empty matches. Differential (27 patterns x 15 strings, 7
operations each): equal to Node 22 except `m`-mode `^`/`$` next to U+2028 (2 of 405 cases). With `m`,
`^`/`$` treat `\n` and `\r` as line ends, not U+2028/U+2029, and do not match between `\r` and `\n`.

**Not supported** (a load or construction error, never a silent difference): lookahead `(?=` `(?!`,
lookbehind `(?<=` `(?<!`, backreferences `\1` `\k<name>`, flags `y d v`. Other differences:
values are immutable, so `lastIndex` stays 0 and `exec`/`test` search from the start; match arrays
are plain arrays without `index`/`input`/`groups` (use `search` for the index); indices are code
points, as everywhere in the language; `i` folds Unicode case as JS does with `u`.

### 7.2 Async functions and parallel effects

`async function f(..) {..}` (top level, also `export`), `const f = async (..) => ..` and async
arrows in expressions. `await` is an operator only in an async function's own body (not in a
nested plain function, a class field or a module initializer): elsewhere it is a load error, as
in JavaScript. Inside a handler, calling an async function runs its synchronous prefix
immediately and returns a promise. Every `await`, including `await 0`, suspends that
function and queues its continuation. Each instance has one FIFO microtask queue for awaits,
reactions, adoption and combinators. The current input's listeners run their synchronous
prefixes before the queue drains. `await Promise.all(xs.map(x => f(x)))` starts every `f` before
parking; the host may serve their requests concurrently and complete them in any order.
Saved frames retain locals, evaluated operands, references and exception/finally state; a
resumption never reruns the earlier computation. Recovery reconstructs the same execution by
feeding journaled external inputs through this executor.

**Promise completions** use JavaScript values: `p.then(f, g)`,
`p.catch(g)` and `p.finally(f)` (callbacks run as queued jobs), `Promise.all`, `Promise.race`, `Promise.any`, `Promise.allSettled`, `Promise.resolve`
and `Promise.reject`. Combinators accept supported iterables (arrays, strings, Map and Set);
an invalid iterable rejects the returned promise with a TypeError. Static methods keep their
`Promise` receiver: a detached method requires `.bind(Promise)` or `.call(Promise, ...)`.
Reaction registration and settlement determine job order. Already settled inputs enqueue
reactions in input order; the resulting promise may still need further adoption or reaction
jobs. Thus `Promise.race([Promise.all([0]), Promise.resolve(1)])` resolves to `1`.
`race` does not cancel its other inputs. An async function that throws
returns a rejected promise. Each promise retains one fulfillment value or rejection reason;
repeated awaits preserve reference identity. `Promise.resolve(p)` returns `p` when it is already
a promise. `catch (e)` and rejection callbacks receive the original thrown/rejected value,
including primitives and `undefined`. Host failures produce Error objects (`name`, `message`,
`stack`); typed failures keep their name (`AbortError`, `TimeoutError`, `InvalidStateError`,
`TypeError`, `AggregateError`). `Promise.any` rejects with an AggregateError whose `errors` holds
the reasons in input order. Non-callable reaction arguments are ignored. `finally`
runs when its block really ends, not when the block suspends on a pending request. A handler body is the root even when written
`async`; outside handlers (render, plain `act`) async functions run synchronously. An unawaited
async call that is still parked when its handler returns resumes when what it waits on settles.

Pending promises can be retained in state and observed by another handler. Retaining a
promise freezes its exposed completion when it settles, without freezing the suspended
function's private locals. Cancelling a task drops its frames and reactions and rejects its
outstanding host promises with `AbortError`; completed external operations are not undone.
Promise construction with an executor and custom thenables are outside the supported subset.

### 7.3 Differences from JavaScript

Each is deliberate (the language's model) or a known limit. Synchronous differences have corpus
cases marked `// expect: divergent` in `tests/js_corpus/`. Supported async behavior is compared
with Node in `tests/promise_corpus.js` and checked through live execution and journal restore.

- **Strings are Unicode code points**, not UTF-16 code units: `length`, indexes, `slice`,
  `charCodeAt` and the rest count a character outside the Basic Multilingual Plane (an emoji) as
  one; a lone surrogate (`"\uD800"`) cannot be written.
- **The state's keys are also global variables** (a template may read `messages` without
  declaring it); a name declared nowhere is a ReferenceError, as in JavaScript (`typeof x` of one
  is `"undefined"`). A `let`/`const` read before its declaration ran is a ReferenceError only in a
  block that contains a function (where declarations are hoisted); elsewhere the name still
  refers to the enclosing scope.
- **Functions have no `arguments`, `length` or `name`**, and prototypes are not values
  (`Object.getPrototypeOf`, `__proto__` and `X.prototype` are not supported). The members objects
  inherit are JavaScript's for `in` and for reading them as values (`"toString" in {}`,
  `o.hasOwnProperty.call(p, k)`, `"push" in []`).
- **Handlers start from the render's values.** Within one render or one handler run, closures
  share variables as above. A handler kept from a render (and every closure it reaches) starts
  each invocation from the variables it captured as that render left them (frozen). Suspended
  invocations retain their own locals, and durable recovery reproduces them from inputs. Assigning such a
  variable is a `TypeError` naming it (the assignment would not last past the run): keep it in
  state.
- **Numbers from JSON**: an integer beyond 2^53 (in the state or `JSON.parse`) keeps its exact
  digits when printed (Python templates print them so); arithmetic on it uses the double.
- **Math**: `Math.sin`, `Math.cos` and `Math.pow` / `**` with an inexact result can differ from
  V8 in the last bit for some arguments (V8 uses its own implementations; the others match V8's
  fdlibm bit for bit).
- **`localeCompare`** follows the root collation for ASCII text; other text compares by code
  points.
- **Sizes are bounded per step** (fuel counts steps, not bytes): a string longer than V8's
  536,870,888 code points is V8's `RangeError: Invalid string length`, however it is built (`+`,
  templates, `join`, `concat`, `repeat`, `pad*`, `replace`, `JSON.stringify`). A step that makes
  or grows an array by many items at once (`Array(n).fill`, spreads, `concat`, `push(...xs)`,
  `split`, `match` with `g`, `Array.from`) throws `RangeError: Invalid array length` past
  10,000,000 items, where V8 allows about 134 million. Child lists nest at most 400 levels; a
  child list inside itself is a TypeError (React overflows the stack).

### 7.4 Not supported

Each raises an explicit load or runtime error, never a silently different
value: getters and setters in object literals (classes have them), static blocks, computed class
member names, function source reflection, primitive boxing through borrowed Object methods,
generators, `yield`, `arguments`, `eval`, `with`, `import()`; `WeakMap`, `WeakSet`,
`Date` (other than `Date.now()`), `Symbol`, `BigInt` (`10n`), `Proxy`, `Reflect`, `Intl`, `Promise` constructors other than
the static methods (7.2), custom thenables; `new` of a function (only classes and the built-in constructors); sparse arrays (holes `[a, , b]`, `Array(n)` without `.fill`, assigning
past the end, `delete xs[i]`, growing `length`); named properties on arrays; tagged templates
other than ``p`..` ``; the regex features in 7.1; legacy octal literals and escapes.

## 8. Errors
A render either returns ids or an error. `throw new Error(msg)` is an explicit raise
(`raise: msg`); `throw new TypeError(msg)`, a failed operation (a property of `undefined`, a
non-iterable) and an unknown name are type errors. `affordance check` counts "both raise" as a match and
compares the kind and the message of explicit raises. Extra props on `<span>`
travel as the span's meta: a template adapter's zero-token `<span label="protocol" syntax="glm"
stop={[...]} in_think={true}/>` tells a runtime how to read the model's replies
(`docs/RUNTIME.md`).

A render error keeps its message on the first line and appends the view stack, one
`    at Name (file:line)` frame per function or component (innermost first), then
`    while rendering TurnStrategy log[i] (kind "...")` when a template turn from the log is in scope.
`affordance::error_message` strips them; `affordance check` compares messages only.

## 9. Not built yet
Embedding another instance's screen inside a parent, streaming/incremental renders that
reuse a previous render's ids, resizing a video file (a `<video>` is counted as its model lays it out and sent as it is; a video is given a size as its frames), an image the model's template app
formats (today the app writes the model's own placeholder token in `t` and any markers around it),
and `{% generation %}` assistant masks (spans can provide them; not wired to the HF API).

## 10. The program is an event loop: the harness

The view renders; the *program* is a loop. The runtime is the browser's event loop, and the loop
that calls the model is the app's own: an instance is a page that renders its state, runs listeners
when events arrive, and calls its model when its code decides to (`spec/HARNESS.md` §3).
What the language contributes:

- **Hooks.** `useState(init, "name")` (a named field of the instance's state); setters called in a
  task apply once to live state, in order (a function to the current value). `useEffect(setup,
  deps)`: after a render whose `deps` changed (React's rule: an item differs by `Object.is`, or
  the length differs; no deps array runs it after every render), the previous cleanup runs, then
  `setup` (a synchronous function that may start async work and return a cleanup).
  `useRef(init, "name")` returns `[read, write]`: storage kept with the instance's hooks
  (journal, images) that does not render. `read()` works in handlers only; a render that reads a
  ref fails (React's rule for `ref.current`), so a handler's writes to refs alone leave the last
  evaluated document current and the page is not evaluated again for them. `views/lib/log.tsx` keeps its open turn's bookkeeping in one
  (`log.isOpen()`, `log.held()`).
- **Listeners and tools.** `useEvent(name, listener)` or `<aff event="name" onCallback>` (an
  event arrives; a `useEvent` listener is a zero-token span after the root), a `<Mount>`'s `onX`
  (its child called back) and `onStop`; `<aff tool_name ...>` tools run when an output is
  dispatched (`dispatch(output)`).
  All reach the world through harness requests. The journal records external inputs for recovery.
- **A tool is a function.** `<Tool fn={f}/>` (`views/lib/tool.tsx`) lowers to `<aff>`: its name
  is `nameOf(f)` (a declaration's name, or the `const` it is bound to), its description `docOf(f)`
  (the summary of the `/** */` comment before the declaration, up to the first `@tag`), its
  argument schema `schemaOf(f)`, the props parameter's type as JSON Schema
  (`src/schema.rs`: string, number, boolean, arrays, object types with optional members,
  string-literal unions as enums, `T | null`, `type` and `interface` aliases from the function's defining file;
  properties and `required` in declaration order; anything else is an error naming the type).
- **A model call is a fetch of a view** (`model()` for the page's own view, or `model(<App/>)` for
  another): a prompt is written in the language like any view, including its template; the output
  is data, and `dispatch(output)` runs its calls. A log of turns is app state
  (`views/lib/log.tsx`), and `views/lib/loop.tsx` writes the two common loops over it:
  `useLoop(log)` (ask, act, repeat) and `useConversation(log)` (answer whatever is unanswered).

Not built: compute allotments as properties of a view (`style={{ maxCallsPerMinute: 6,
computeShare: 0.2 }}`, enforced by the harness), and decision readouts as a model binding's mode.

## 11. Python idioms
Chat templates are written in Jinja, so a view ported from one, or written by a model that
knows them, reaches for Python idioms (`str()`, `len()`, slices, keyword arguments, Python
truthiness). The language is the one in sections 2-8; its directives are `"use strict_tokens"`
(6.4) and `"use safe_text"` (6.3).

The table below is generated from `src/migrate.rs`, the table the parser, the runtime and
`affordance lint` use. A Python idiom fails with its replacement in the message, at load time for syntax
(`Python slice syntax ... use x.slice(a, b)`) and at run time for names
(`ReferenceError: enumerate is not defined (... use xs.entries())`). `affordance lint [paths]` flags all
of them ahead of time (section 12).

| Python / Jinja | JavaScript |
|---|---|
| slices `x[:160]`, `xs[-5:]`, `t[a:b]`, `s[::-1]` | `x.slice(0, 160)`, `xs.slice(-5)`, `t.slice(a, b)`, `[...s].reverse().join("")` |
| keyword arguments `dict(r, reasoning="", ids=null)`, `f(x, indent=4)` | `{...r, reasoning: "", ids: null}`, `f(x, {indent: 4})` |
| `k not in o`, `x not in xs`, `a and b`, `a or b`, `not a` | `!(k in o)`, `!xs.includes(x)`, `a && b`, `a \|\| b`, `!a` |
| `list + [x]`, `a + b` on lists | `xs.push(x)` (or `[...xs, x]`), `[...a, ...b]` |
| `True`, `False`, `None` | `true`, `false`, `null` |
| empty list / dict is false | `xs.length > 0`, `Object.keys(o).length > 0` (`[]` and `{}` are truthy) |
| `x is defined`, `x is none` | `x !== undefined`, `x === null` |
| `str(..)` | String(x), or a template literal `${x}` (formatPythonValue(x) for Python's text) |
| `len(..)` | x.length (Object.keys(o).length for an object) |
| `int(..)` | parseInt(s) or Math.trunc(x) |
| `float(..)` | Number(x) |
| `bool(..)` | Boolean(x) |
| `repr(..)` | formatPythonRepr(x) |
| `json(..)` | JSON.stringify(x), or formatJSON(x, {indent, separators, ascii, sortKeys}) for Python's json.dumps text |
| `tojson(..)` | JSON.stringify(x), or formatJSON(x) for Python's json.dumps text |
| `parse_json(..)` | JSON.parse(s) |
| `try_parse_json(..)` | JSON.parse(s) inside try { } catch { } |
| `enumerate(..)` | xs.entries() (for (const [i, x] of xs.entries())) |
| `range(..)` | for (let i = 0; i < n; i++), or Array.from({length: n}, (_, i) => i) |
| `items(..)` | Object.entries(o) |
| `keys(..)` | Object.keys(o) |
| `values(..)` | Object.values(o) |
| `dict(..)` | {...o, k: v} |
| `list(..)` | [...x] or Array.from(x) |
| `reversed(..)` | [...xs].reverse() |
| `sorted(..)` | [...xs].sort((a, b) => ...) |
| `sort_by(..)` | [...xs].sort((a, b) => ...) |
| `join(..)` | xs.join(sep) |
| `first(..)` | xs[0] |
| `last(..)` | xs.at(-1) |
| `default(..)` | x ?? d |
| `merge(..)` | {...a, ...b} |
| `omit(..)` | const {k, ...rest} = o |
| `min(..)` | Math.min(...xs) |
| `max(..)` | Math.max(...xs) |
| `abs(..)` | Math.abs(x) |
| `sum(..)` | xs.reduce((a, b) => a + b, 0) |
| `any(..)` | xs.some(x => ...) |
| `all(..)` | xs.every(x => ...) |
| `raise(..)` | throw new Error(msg) |
| `trim(..)` | s.trim() |
| `strftime_now(..)` | formatNow(fmt) |
| `count_tokens(..)` | countTokens(s) |
| `is_fragment(..)` | isFragment(x) |
| `is_string(..)` | typeof x === "string" |
| `is_number(..)` | typeof x === "number" |
| `is_int(..)` | Number.isInteger(x) |
| `is_bool(..)` | typeof x === "boolean" |
| `is_map(..)` | isPlainObject(x) |
| `is_list(..)` | Array.isArray(x) |
| `is_sequence(..)` | Array.isArray(x) |
| `is_none(..)` | x === null |
| `is_defined(..)` | x !== undefined |
| `is_undefined(..)` | x === undefined |
| `.strip(..)` | .trim() (trimChars(s, chars) to strip given characters) |
| `.lstrip(..)` | .trimStart() (trimStartChars(s, chars)) |
| `.rstrip(..)` | .trimEnd() (trimEndChars(s, chars)) |
| `.startswith(..)` | .startsWith(x) |
| `.endswith(..)` | .endsWith(x) |
| `.lower(..)` | .toLowerCase() |
| `.upper(..)` | .toUpperCase() |
| `.title(..)` | titleCase(s) |
| `.splitlines(..)` | .split("\n") (or .split(/\r?\n/)) |
| `.rsplit(..)` | .split(sep) and take from the end |
| `.rfind(..)` | .lastIndexOf(x) |
| `.isdigit(..)` | /^\d+$/.test(s) |
| `.isalpha(..)` | /^[a-zA-Z]+$/.test(s) |
| `.isalnum(..)` | /^[a-zA-Z0-9]+$/.test(s) |
| `.isspace(..)` | /^\s+$/.test(s) |
| `.format(..)` | a template literal `${x}` |
| `.items(..)` | Object.entries(o) |
| `.keys(..)` | Object.keys(o) |
| `.values(..)` | Object.values(o) |
| `.get(..)` | o[k] ?? d |
| `.append(..)` | .push(x) |
| `.extend(..)` | .push(...xs) |
| `.count(..)` | s.split(x).length - 1, or xs.filter(y => y === x).length |
| `.index(..)` | .indexOf(x) |

## 12. Changing the language

The language has consumers outside its own views: agent apps in other repositories import views
from this tree. A removal that migrates every view here but not those leaves their agents idle on a
view that no longer parses. So:

1. **Syntax or builtins are not removed without migrating every consumer in the same change.**
   The consumers are this tree's `views/`, `tests/fixtures/`, the docs' code examples, and the
   out-of-tree directories listed (one per line) in a local, untracked `tools/view_consumers.txt`.
2. **A removal adds its migration** to `src/migrate.rs` (and so to the errors and to `affordance lint`)
   and to the table in section 11. `tests/lint.rs` fails if the two disagree.
3. **`affordance lint` must be clean** (`tests/lint.rs` runs it over all of the above), and the runtime's
   acceptance port (`tests/session_agent.rs`) is run against the real agent views before the tree
   is left in that state: `VIEW=<consumer view> PROPS='{...}' cargo test --release
   --test session_agent`.
4. **Consumers import, they do not copy.** An app that is a variant of a view in this tree imports
   it and overrides props (`SessionAgent({...props})`), so it follows
   the language with its source.
