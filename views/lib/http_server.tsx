// Serving HTTP from an app (`affordance serve`), as a Service Worker answers `fetch` events: each
// request arrives as the event `request` ({id, method, path, query, headers, body}); the answer goes
// to the machine `client`.
//
//   useRequests(async (req, res) => {
//     if (req.method === "GET" && req.path === "/hello") return res.json({hello: "world"});
//     return res.json({error: "not found"}, 404);
//   });
//
// A handler may answer later (after a model call, say) and may stream: `const out = await
// res.stream(200, {"content-type": "text/event-stream"}); await out.write("data: ...\n\n");
// await out.end();`. A handler that throws before answering answers 500 with the error.
//
// A model's reply can be piped into a streamed response as it is generated:
//
//   const out = await res.stream(200, {"content-type": "text/event-stream"});
//   const reply = await model(<View/>, {pipe: out.pipe({text: `data: {"delta": "${PIECE}"}\n\n`})});
//
// A frame is the chunk written for each piece, with PIECE where the piece goes (escaped as the
// inside of a JSON string). Kinds: `text`, `reasoning`, and `tool_call` (which also takes
// PIECE_INDEX, PIECE_ID and PIECE_NAME; its PIECE is more of the call's arguments). A kind with no
// frame is not written. The platform writes the chunks; the app's code does not run for a piece,
// and `reply` is the same whole reply as without a pipe. Pieces come from a hosted binding's
// stream; a native binding's reply is not piped yet.

/** Where a piece goes in a frame (see above). */
export const PIECE = "";
export const PIECE_INDEX = "";
export const PIECE_ID = "";
export const PIECE_NAME = "";

/** The response helpers for request `id` on the machine `client`. */
export function responder(client, id) {
  let answered = false;
  let streaming = false;
  const send = async (args) => { answered = true; return await client.call({respond: id, ...args}); };
  return {
    isAnswered: () => answered,
    isStreaming: () => streaming,
    json: (body, status = 200, headers = {}) =>
      send({status, headers: {"content-type": "application/json", ...headers}, body: JSON.stringify(body)}),
    text: (body, status = 200, headers = {}) =>
      send({status, headers: {"content-type": "text/plain; charset=utf-8", ...headers}, body}),
    stream: async (status = 200, headers = {}) => {
      await send({status, headers, stream: true});
      streaming = true;
      return {
        write: async (chunk) => await client.call({respond: id, chunk}),
        // The `pipe` option of a model call: its pieces, framed, are written to this response.
        pipe: (frames) => ({machine: "client", call: {respond: id}, ...frames}),
        end: async () => { streaming = false; return await client.call({respond: id, end: true}); },
      };
    },
  };
}

/** Handle every request with `handler(req, res)` (see above). */
export function useRequests(handler) {
  const client = harness.machine.get("client");
  useEvent("request", async (req) => {
    const res = responder(client, req.id);
    try {
      await handler(req, res);
    } catch (e) {
      if (!res.isAnswered()) {
        await res.json({error: {message: String(e)}}, 500);
      } else if (res.isStreaming()) {
        // A response already streaming cannot become an error; it is ended, so the client is not
        // left waiting for the server's timeout.
        try { await client.call({respond: req.id, end: true}); } catch (gone) {}
      }
    }
  });
}
