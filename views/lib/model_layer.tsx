// Affordance as another harness's model layer. The harness keeps its own loop, tools and
// conversation; for each model call it sends what a chat API takes (a system prompt, the messages
// so far, the tools it offers) and gets the reply back, parsed. Affordance renders the conversation
// through the bound model's template into exact tokens, calls the model, and reads the reply.
//
//   useModelLayer();   // in an app run with `affordance host`
//
// A call is the event `generate`: {id, system?, messages, tools?, max_new?, sampling?}.
//   messages: {role: "user", content} | {role: "assistant", content, reasoning?, tool_calls?:
//             [{id, name, arguments}]} | {role: "tool", tool_call_id, name, content, error?}
//             (content: a string, or parts {type: "text", text} / {type: "image_url", image_url})
//   tools:    {name, description, parameters (a JSON schema)}; the caller runs them, not this app
// The event `abort` ({id}) stops a call: its answer is what was generated, with `aborted`.
// The answer is a call on the machine `ui`: {generated: {id, output: {text, reasoning, calls:
// [{id, name, arguments}], cut_off, aborted, problems, usage}}} or {generated: {id, error, name}}.
// The reply's text streams as it is generated (the host's `delta` events).
import { ModelContext } from "./model.tsx";

// Chat messages as log records: what the model saw and did, in the shape the templates read.
export function chatRecords(messages) {
  return messages.map(m => {
    if (m.role === "assistant") {
      return {kind: "model", ids: null, reasoning: m.reasoning || "", content: m.content || "",
              calls: (m.tool_calls || []).map(c => ({type: "function", id: c.id, function: {name: c.name, arguments: c.arguments}}))};
    }
    if (m.role === "tool") {
      const record = {kind: "action", name: m.name, tool_call_id: m.tool_call_id, result: m.content};
      return m.error ? {...record, error: true} : record;
    }
    return {kind: "input", content: m.content};
  });
}

export function useModelLayer() {
  // The calls in flight, by id: each one's controller. A ref: it never renders.
  const [calls, setCalls] = useRef({}, "model_layer~calls");
  const answer = generated => harness.machine.get("ui").call({generated});
  useEvent("generate", async request => {
    const controller = new AbortController();
    setCalls(c => ({...c, [request.id]: controller}));
    let generated;
    try {
      // The tools are declared for the model to call; their handlers are the caller's.
      const tools = (request.tools || []).map(t =>
        <aff key={t.name} tool_name={t.name} description={t.description || ""}
             args_schema={t.parameters || {type: "object", properties: {}}} onRun={() => null}>{""}</aff>);
      const options = {signal: controller.signal};
      if (request.max_new !== undefined) options.max_new = request.max_new;
      if (request.sampling !== undefined) options.sampling = request.sampling;
      const out = await model(
        <ModelContext system={request.system} strategy="unrolled" log={chatRecords(request.messages || [])}>{tools}</ModelContext>,
        options);
      generated = {id: request.id, output: {
        text: out.text, reasoning: out.reasoning,
        calls: out.calls.map(c => ({id: c.id, name: c.name, arguments: c.args})),
        cut_off: out.cut_off, aborted: !!out.aborted, problems: out.problems, usage: out.usage}};
    } catch (e) {
      generated = {id: request.id, error: e.message, name: e.name};
    }
    setCalls(c => { const {[request.id]: done, ...rest} = c; return rest; });
    await answer(generated);
  });
  useEvent("abort", ({ id }) => {
    const controller = calls()[id];
    if (controller) controller.abort();
  });
}
