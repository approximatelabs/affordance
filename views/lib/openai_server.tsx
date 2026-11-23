// An OpenAI-compatible endpoint for an app served with `affordance serve`: `GET /v1/models` and
// `POST /v1/chat/completions` (with or without `stream`). The app supplies the reply:
//
//   useOpenAIServer({name: "my-agent", reply: async ({conversation, message, messages, request}) => "..."});
//
// For a request with `stream`, `pipe` is given too: pass it to the model call that writes the
// reply (`model(view, {pipe})`) and its text and reasoning reach the client as they are generated;
// then return `{text, piped: true}` so the text is not sent a second time. Without it (or for a
// reply that is not one model call) return the text, which is sent whole.
//
// `message` is the text of the request's last user message; `messages` the whole list the client
// sent; `conversation` names the conversation: the `x-conversation-id` header, else the request's
// `user` field, else "default". A client sending its whole history each time can talk to an app
// that keeps its own (its journal), so the app decides what the model sees.
import { useRequests, PIECE } from "./http_server.tsx";

/** The text of a chat message's content: a string, or the text parts of a list. */
export function messageText(content) {
  if (typeof content === "string") return content;
  if (Array.isArray(content)) return content.filter(p => p && p.type === "text").map(p => p.text).join("\n");
  return "";
}

function invalid(res, message, status = 400) {
  return res.json({error: {message, type: "invalid_request_error"}}, status);
}

export function useOpenAIServer({ name = "affordance", reply }) {
  useRequests(async (req, res) => {
    if (req.method === "GET" && req.path === "/v1/models") {
      return await res.json({object: "list", data: [{id: name, object: "model", owned_by: "affordance"}]});
    }
    if (req.path !== "/v1/chat/completions") return await invalid(res, `no route ${req.method} ${req.path}`, 404);
    if (req.method !== "POST") return await invalid(res, "use POST", 405);
    let body;
    try { body = JSON.parse(req.body); } catch (e) { return await invalid(res, "the body is not JSON"); }
    const messages = body.messages;
    if (!Array.isArray(messages) || messages.length === 0) return await invalid(res, "messages must be a nonempty list");
    const last = messages[messages.length - 1];
    if (!last || last.role !== "user") return await invalid(res, "the last message must be the user's");
    const conversation = req.headers["x-conversation-id"] || body.user || "default";
    const id = `chatcmpl-${req.id}`;
    const created = Math.floor(harness.clock_ms / 1000); // the journal's clock, so a replay matches
    const model = body.model || name;
    const ask = {conversation, message: messageText(last.content), messages, request: body};
    if (body.stream) {
      const out = await res.stream(200, {"content-type": "text/event-stream", "cache-control": "no-cache"});
      const chunk = (delta, finish) => `data: ${JSON.stringify({id, object: "chat.completion.chunk", created, model,
        choices: [{index: 0, delta, finish_reason: finish}]})}\n\n`;
      await out.write(chunk({role: "assistant", content: ""}, null));
      // The frames of a piped reply: one chunk a piece, in the same shape. A reply the app does
      // not pipe (its `piped` is false) is sent whole below.
      const pipe = out.pipe({text: chunk({content: PIECE}, null), reasoning: chunk({reasoning_content: PIECE}, null)});
      try {
        const answer = await reply({...ask, pipe});
        if (!(answer && answer.piped)) await out.write(chunk({content: typeof answer === "string" ? answer : answer.text}, null));
        await out.write(chunk({}, "stop"));
      } catch (e) {
        // The response has begun, so the failure travels in it: an error event, then the end.
        await out.write(`data: ${JSON.stringify({error: {message: String(e.message || e), type: "server_error"}})}\n\n`);
      }
      await out.write("data: [DONE]\n\n");
      return await out.end();
    }
    const answer = await reply(ask);
    const text = typeof answer === "string" ? answer : answer.text;
    return await res.json({id, object: "chat.completion", created, model,
      choices: [{index: 0, message: {role: "assistant", content: text}, finish_reason: "stop"}]});
  });
}
