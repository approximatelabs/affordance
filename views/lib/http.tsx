// A named capability with the same signature as global fetch. The platform helper shares
// normalization and Response construction; the host still checks the caller's bound role.
import { createFetch } from "affordance:web";
export { createFetch };

// Parse a finite SSE response as data. Live streams require a streaming transport; this
// helper deliberately delivers nothing until the HTTP effect has completed and journaled.
export function httpEvents(body) {
  const events = [];
  let data = [];
  let event = "message";
  let id = "";
  const text = body.charCodeAt(0) === 65279 ? body.slice(1) : body;
  for (const line of text.replace(/\r\n?/g, "\n").split("\n")) {
    if (line === "") {
      if (data.length) events.push({event, data: data.join("\n"), id});
      data = [];
      event = "message";
    } else if (!line.startsWith(":")) {
      const colon = line.indexOf(":");
      const field = colon < 0 ? line : line.slice(0, colon);
      let value = colon < 0 ? "" : line.slice(colon + 1);
      if (value.startsWith(" ")) value = value.slice(1);
      if (field === "data") data.push(value);
      if (field === "event") event = value || "message";
      if (field === "id" && !value.includes("\u0000")) id = value;
    }
  }
  return events;
}
