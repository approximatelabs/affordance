// One template app for any model whose chat format is registered (views/index.json). It is
// given the format (a view from messages and tools to the model's prompt text) and what an
// agent needs to read a reply: the tokens that end one, the reply syntax, and whether the
// prompt ends inside open thinking.
//
// Two rules:
// - Text goes through the format itself. Content that is more than text (an app's screen with
//   its actions, an image) is held back: the format is rendered with a marker in its place, and
//   the content is put where the marker came out.
// - A past reply is written the way the model's publisher writes one: the format is given what
//   the reply said (its text, its thinking, its calls) and writes it. A publisher's template may
//   write a past reply differently from what the model generated (drop its thinking, change its
//   last token, move the tool list): that is the publisher's decision about what the model
//   reads, and the default here. Where the format writes every past reply as exactly the text
//   the model generated, the reply's own tokens are kept, so the prompt extends the last one and
//   a server reuses its work. `keep_replies` keeps the tokens always (the prompt is then the
//   prompt the model saw, its reply, and what the format writes after a reply), for a
//   deployment that prefers reuse to the publisher's rewriting.
import { contentText } from "../lib/log.tsx";
import { publisherTurns } from "./input.tsx";

const OPEN = String.fromCharCode(0xE000);
const CLOSE = String.fromCharCode(0xE001);

// A turn's content as a list of parts (text, images, audio, video), as a tool result that
// returns a picture is.
const isParts = c => Array.isArray(c) && c.length > 0 && c.every(p => isPlainObject(p) && typeof p.type === "string");
// The kind of media a part is, or null: by the names chat APIs and templates use for each.
const KINDS = {image: "image", image_url: "image", audio: "audio", input_audio: "audio", audio_url: "audio", video: "video", video_url: "video"};
const kindOf = p => KINDS[p.type] ?? null;
// A media part's source (a data URL), if it has one.
function sourceOf(p, kind) {
  const url = p[kind + "_url"];
  if (typeof url === "string") return url;
  if (url?.url) return url.url;
  if (typeof p[kind] === "string") return p[kind];
  // OpenAI's audio part: base64 data and its format.
  if (p.input_audio?.data) return `data:audio/${p.input_audio.format ?? "wav"};base64,${p.input_audio.data}`;
  return null;
}
const count = (text, token) => token ? text.split(token).length - 1 : 0;

// The format's text for `turns`, cut where held content goes: strings, and {held: content}.
// With it, each kind of media's sources in the order the format writes their placeholders.
function rendered(Format, turns, tools, options, generate) {
  const held = [];
  const sources = {image: [], audio: [], video: []};
  const tokens = options.media ?? {};
  const messages = turns.map(m => {
    if (m.content === null || m.content === undefined || typeof m.content === "string") return m;
    if (isParts(m.content)) {
      // A kind the format has no placeholder for is given as text ("[image]" where one was).
      if (m.content.some(p => kindOf(p) && !tokens[kindOf(p)])) return {...m, content: contentText(m.content)};
      return {...m, content: m.content.map(p => {
        const kind = kindOf(p);
        if (!kind) return p;
        sources[kind].push(sourceOf(p, kind));
        return {type: kind};
      })};
    }
    held.push(m.content);
    return {...m, content: OPEN + (held.length - 1) + CLOSE};
  });
  // A format that reads content as a list of parts is given each content as one text part.
  const given = options.content_parts ? messages.map(m => typeof m.content === "string" ? {...m, content: [{type: "text", text: m.content}]} : m) : messages;
  const {content_parts, media, media_metrics, ...kwargs} = options;
  const text = Format({...kwargs, messages: given, tools: tools.length ? tools : null, add_generation_prompt: generate});
  const out = [];
  let at = 0;
  for (;;) {
    const open = text.indexOf(OPEN, at);
    if (open < 0) break;
    const close = text.indexOf(CLOSE, open);
    out.push(text.slice(at, open), {held: held[Number(text.slice(open + 1, close))]});
    at = close + 1;
  }
  out.push(text.slice(at));
  return {parts: out, sources, tokens, text, metrics: options.media_metrics ?? {}};
}

// A media element: the format's placeholder token, carrying the image or the clip to the
// model request.
// It is sized by layout: it takes the tokens the model's processor gives it at the size the
// style sheet allows (`img { max-tokens: .. }`), by the model's size rule for that kind of
// file (`metrics`, from the registry row, or the tokenizer's own). With no rule, or when it
// cannot fit, it is not laid out at a guess: its alt text stands in and its file is not sent.
// `last`: it is the newest of its kind in the prompt, so a sheet can give the newest more room
// than the ones before it (`img { max-tokens: 64tok; } img[last] { max-tokens: 1024tok; }`).
function Media({ kind, src, t, metrics, last }) {
  if (kind === "audio") return <audio src={src} t={t} metrics={metrics} last={last}/>;
  if (kind === "video") return <video src={src} t={t} metrics={metrics} last={last}/>;
  return <img src={src} t={t} metrics={metrics} last={last}/>;
}

// One piece of the format's text with each of `kind`'s placeholders as the media itself.
// `next`: the index of the first one's source (a holder, counted on).
function withMedia(piece, kind, token, sources, next, metrics) {
  if (typeof piece !== "string") return Array.isArray(piece) ? piece.map(x => withMedia(x, kind, token, sources, next, metrics)) : piece;
  if (piece.indexOf(token) < 0) return piece;
  return piece.split(token).map((text, i) => {
    if (i === 0) return text;
    const at = next.n++;
    const src = sources[at];
    // `index`: the file's place in the prompt, for a rule that numbers its pictures (MiniCPM-V).
    const placed = isPlainObject(metrics) ? {...metrics, index: at} : metrics;
    // `after`: text the processor adds behind the token that closes a file (Ling-VL's newline
    // after its end marker), where the format wrote that token.
    const after = src && isPlainObject(metrics) && metrics.after;
    const rest = after && text.startsWith(after[0]) ? after[0] + after[1] + text.slice(after[0].length) : text;
    return [src ? <Media kind={kind} src={src} t={token} metrics={placed} last={at === sources.length - 1}/> : token, rest];
  });
}

// The pieces from `from` on as content: held content in place, and each media placeholder the
// format wrote as the image or clip itself.
function emit({parts, sources, tokens, metrics}, from = 0) {
  const before = parts.slice(0, from).filter(x => typeof x === "string").join("");
  let pieces = parts.slice(from).map(x => typeof x === "string" ? x : {held: x.held});
  for (const kind of ["image", "audio", "video"]) {
    const token = tokens[kind];
    if (!token) continue;
    const next = {n: count(before, token)};
    pieces = pieces.map(x => isPlainObject(x) && "held" in x ? x : withMedia(x, kind, token, sources[kind], next, metrics[kind]));
  }
  return pieces.map(x => isPlainObject(x) && "held" in x ? x.held : x);
}

// The format's rendering. Formats differ on what an assistant turn must have: one with calls
// and no text is given as "" and, if the format refuses that, as null; one with no thinking is
// given none and, if the format refuses that, an empty one.
// An assistant turn without an empty reasoning field: a format with no thinking at all refuses
// even an empty one (Mistral's V11 tokenizer).
function unthought(m) {
  if (m.role !== "assistant" || m.reasoning_content !== "") return m;
  const {reasoning_content, ...rest} = m;
  return rest;
}

function written(Format, turns, tools, options, generate) {
  const bare = m => m.role === "assistant" && m.content === "" && m.tool_calls ? {...m, content: null} : m;
  const thought = m => m.role === "assistant" && m.reasoning_content === undefined ? {...m, reasoning_content: ""} : m;
  let refused;
  for (const given of [turns, turns.map(bare), turns.map(thought), turns.map(bare).map(thought), turns.map(unthought), turns.map(bare).map(unthought)]) {
    try {
      return rendered(Format, given, tools, options, generate);
    } catch (e) {
      refused = refused ?? e;
    }
  }
  throw refused;
}

// The prompt with every reply's own tokens: the prompt the model saw, its reply, and what the
// format writes after a reply.
function keeping(Format, turns, tools, options, stop) {
  let last = -1;
  turns.forEach((m, i) => { if (m.generated) last = i; });
  const held = turns.map(heldReply);
  let all;
  try {
    all = rendered(Format, held, tools, options, true);
  } catch (e) {
    all = rendered(Format, held.map(unthought), tools, options, true);
  }
  if (last < 0) return emit(all);
  const reply = turns[last];
  // What the format writes after this reply: everything after its marker.
  const here = all.parts.findIndex(x => typeof x !== "string" && x.held === reply.content);
  if (here < 0) throw new Error("this model's format did not write the assistant turn's content, so the reply cannot be placed in the next prompt");
  // The rest of what the format writes for this turn (its calls again, the turn's end) is
  // already in the reply's tokens, which end with the token that stopped it. A reply that was
  // cut off has no such token: the format's own turn end is kept.
  const text = typeof all.parts[here + 1] === "string" ? all.parts[here + 1] : "";
  const ends = stop.map(s => [text.indexOf(s), s]).filter(([at]) => at >= 0).sort((a, b) => a[0] - b[0])[0];
  if (ends) all.parts[here + 1] = text.slice(ends[0] + (reply.ids?.at(-1) === tok[ends[1]]?.id ? ends[1].length : 0));
  return [keeping(Format, turns.slice(0, last), tools, options, stop), tokens(reply.ids), emit(all, here + 1)];
}

// A generated turn as a message whose content is held back (its tokens go where the format
// writes it). Whatever else the format writes for the turn is discarded, so it is given an
// empty thinking (some formats refuse an assistant turn without one) and its calls without
// their data (a later tool result may be written from their names and ids).
function heldReply(m) {
  if (!m.generated) return m;
  const {generated, ids, reply, ...rest} = m;
  if (rest.reasoning_content === undefined) rest.reasoning_content = "";
  if (rest.tool_calls) rest.tool_calls = rest.tool_calls.map(c => ({...c, function: {...c.function, arguments: {}}}));
  return rest;
}

// The prompt for `turns`, ending where the model writes next.
export function chatPrompt(Format, turns, tools, options, stop, keep_replies = false) {
  const given = publisherTurns(turns, keep_replies, (messages, generate) => written(Format, messages, tools, options, generate).text);
  return given.some(m => m.generated) ? keeping(Format, given, tools, options, stop) : emit(written(Format, given, tools, options, true));
}

// How the format writes a tool call, for the reply reader (src/reply_pattern.rs). The format is
// asked to write calls whose every piece is a string that occurs nowhere else, so where each
// lands is known and what lies between is the format's own writing: one call with one argument,
// with two, with a number, with none, and two calls; and a turn with thinking.
const piece = (kind, i) => String.fromCharCode(kind) + i;
const NAME = 0xE010;
const KEY = 0xE020;
const VALUE = 0xE030;
const ID = 0xE040;

// What the format writes for an assistant turn `turn` after the prompt, up to the turn's end.
function probed(Format, given_options, tools, turn, stop) {
  const {content_parts, media, media_metrics, ...options} = given_options;
  const user = {role: "user", content: "x"};
  const given = tools.length ? tools : null;
  let prompt, full;
  try { prompt = Format({...options, messages: [user], tools: given, add_generation_prompt: true}); } catch (e) { return null; }
  // A turn with calls has no text. Formats differ on whether that is null or "", and one that
  // prints the wrong one as text ("None") is given the other: the shorter writing is kept.
  for (const content of turn.content === undefined ? [null, ""] : [turn.content]) {
    try {
      const text = Format({...options, messages: [user, {role: "assistant", ...turn, content}], tools: given, add_generation_prompt: false});
      if (full === undefined || text.length < full.length) full = text;
    } catch (e) {}
  }
  if (full === undefined) return null;
  let at = 0;
  while (at < prompt.length && at < full.length && prompt[at] === full[at]) at++;
  let body = full.slice(at);
  const ends = stop.map(s => body.indexOf(s)).filter(i => i >= 0).sort((a, b) => a - b)[0];
  return ends === undefined ? body : body.slice(0, ends);
}

export function callProbe(Format, options, tools, stop, id_name) {
  const call = (i, args) => ({id: piece(ID, i - 1), type: "function", function: {name: piece(NAME, i), arguments: args}});
  const calls = list => probed(Format, options, tools, {tool_calls: list}, stop);
  const arg = i => ({[piece(KEY, i)]: piece(VALUE, i)});
  const thought = probed(Format, options, tools, {reasoning_content: piece(VALUE, 8), content: piece(VALUE, 9)}, stop);
  const [at8, at9] = thought ? [thought.indexOf(piece(VALUE, 8)), thought.indexOf(piece(VALUE, 9))] : [-1, -1];
  return {
    one: calls([call(1, arg(1))]),
    pair: calls([call(1, {...arg(1), ...arg(2)})]),
    number: calls([call(1, {[piece(KEY, 1)]: 7})]),
    none: calls([call(1, {})]),
    two: calls([call(1, arg(1)), call(2, arg(2))]),
    think: at8 >= 0 && at9 > at8 ? [thought.slice(0, at8), thought.slice(at8 + 2, at9)] : null,
    id_name: id_name ?? null,
  };
}

// `text`: the format is a function from messages to prompt text (a converted template). A
// hand-written format takes laid-out content and generated turns itself.
// `alternate`: the format requires user and assistant turns to alternate, so a user turn after
// another joins it. `system_role`: false for a format with no system turn; the system text then
// opens the first user turn. `special`: the tokenizer's named tokens, as its template reads them.
// `tools`: false for a format that refuses tools; the model then only talks. `parts`: the
// format reads content as a list of parts. `id_name`: the text before and after a tool's name in
// a call id, for a format that writes ids and no names. `media`: the token the format writes
// where an image, an audio clip or a video goes.
export function ChatApp({ children, system = "Use the available tools to interact with the app.", format, text = true, stop = [],
                          syntax = null, in_think = false, alternate = false, system_role = true, tools = true, parts = false, id_name = null, media = null, media_metrics = null, special = {}, keep_replies = false, options = {} }) {
  const input = templateInput(children, system_role ? system : null, true, alternate);
  const Format = format;
  let turns = input.turns;
  if (!system_role && system) {
    const first = turns.findIndex(m => m.role === "user");
    if (first >= 0) turns = turns.map((m, i) => i === first ? {...m, content: [system + "\n\n", m.content]} : m);
  }
  const all = {...special, ...options, content_parts: parts, media, media_metrics};
  const prompt = text ? chatPrompt(Format, turns, tools ? input.tools : [], all, stop, keep_replies)
    : <Format {...all} messages={turns} tools={tools ? input.tools : []} add_generation_prompt={true}/>;
  const stops = stop.map(s => tok[s]?.id).filter(id => id !== undefined);
  // A format with no way to write a tool call: its reply is text.
  if (syntax !== "pattern") return [prompt, <span label="protocol" syntax={syntax ?? "text"} stop={stops} in_think={in_think}/>];
  // Measured once for a set of tools.
  const calls = memoScan([JSON.stringify(input.tools)], null,
    (s, json) => [null, callProbe(Format, all, JSON.parse(json), stop, id_name)], null).outs[0];
  return [prompt, <span label="protocol" syntax="pattern" calls={calls} stop={stops} in_think={in_think}/>];
}
