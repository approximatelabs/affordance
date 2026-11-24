import { contentText } from "../lib/log.tsx";
// The adapter creates a system turn and a JSON tool table every render. Keep those derived
// values stable so transcript scans can reuse their unchanged prefix by identity.
export function prepareTemplateInput(children, system, generated) {
  const input = templateInput(children, system, generated);
  const head = memoScan(system === null ? [] : [system], null,
    (s, content) => [null, {role: "system", content}], null).outs;
  // asChatTools produces JSON schemas: their serialized data is the dependency, without handlers.
  const tools = memoScan([JSON.stringify(input.tools)], null,
    (s, json) => [null, JSON.parse(json)], null).outs[0];
  return {turns: system === null ? input.turns : [...head, ...input.turns.slice(1)], tools};
}

// A text-only template reads a message's `content` as a string. Content parts (a tool result that
// carries an image) become their text, with `[image]` where an image was: never an empty result
// or the parts printed as JSON. Generated turns are re-emitted by ids and pass through.
const hasParts = m => !m.generated && Array.isArray(m.content);
export function textTurns(turns) {
  return turns.some(hasParts) ? turns.map(m => hasParts(m) ? {...m, content: contentText(m.content)} : m) : turns;
}

// A generated turn as the message a format writes a past reply from: what the reply said (its
// text, its thinking, its calls).
export function said(m) {
  if (!m.generated) return m;
  const {generated, ids, reply, provider_state, ...rest} = m;
  const message = {...rest, content: reply?.content ?? ""};
  if (reply?.reasoning) message.reasoning_content = reply.reasoning;
  return message;
}

// The turns a template app gives its format, so that a past reply is written the way the
// model's publisher writes one. A publisher's template may write a past reply differently from
// what the model generated (drop its thinking, change its last token): that is the publisher's
// decision about what the model reads, and the default. Where the format writes every past
// reply as exactly the text the model generated, the replies stay as generated turns (their own
// tokens are kept, so the prompt extends the last one and a server reuses its work).
// `keep_replies` keeps them always. `write(messages, generate)` is the format's text for a
// conversation. A publisher may write a reply as generated while it is the last turn and rewrite
// it once a newer user message follows (Qwen 3.6, DeepSeek V4.1 drop a past reply's thinking
// then): each reply is checked when it arrives, and again, against the conversation up to that
// message, when the next user message arrives. Earlier turns do not change, so a check is kept.
export function publisherTurns(turns, keep_replies, write) {
  if (keep_replies || !turns.some(m => m.generated)) return turns;
  const written = turns.map(said);
  // Reply `i` is written as generated in the conversation's first `end` turns: as the last turn,
  // or (with a newer user message) in the prompt that message asks for, generation prompt and all.
  const asGenerated = (i, end) => {
    try {
      return write(written.slice(0, end), end > i + 1).startsWith(write(written.slice(0, i), true) + textContent(turns[i].content));
    } catch (e) {
      return false;
    }
  };
  // State: whether every reply so far is written as generated, and the replies since the last
  // user message (checked again when the next one arrives).
  const scan = memoScan(turns, null, (s, m, i) => {
    const {ok, pending} = s ?? {ok: true, pending: []};
    if (!ok) return [{ok, pending: []}, null];
    if (m.generated) return [{ok: asGenerated(i, i + 1), pending: [...pending, i]}, null];
    if (m.role === "user" && pending.length) return [{ok: pending.every(j => asGenerated(j, i + 1)), pending: []}, null];
    return [{ok, pending}, null];
  }, null);
  return scan.state?.ok === false ? written : turns;
}
