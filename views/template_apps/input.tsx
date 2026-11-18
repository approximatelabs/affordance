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
// conversation; each reply is checked once (a conversation's earlier turns do not change).
export function publisherTurns(turns, keep_replies, write) {
  if (keep_replies || !turns.some(m => m.generated)) return turns;
  const written = turns.map(said);
  const asGenerated = i => {
    try {
      return write(written.slice(0, i + 1), false).startsWith(write(written.slice(0, i), true) + textContent(turns[i].content));
    } catch (e) {
      return false;
    }
  };
  const unchanged = memoScan(turns, true, (ok, m, i) => [ok && (!m.generated || asGenerated(i)), null], null).state;
  return unchanged ? turns : written;
}
