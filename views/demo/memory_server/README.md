# A memory longer than the context, for any OpenAI client

A chat model behind an OpenAI-compatible endpoint (`affordance serve`) that keeps each
conversation itself, in its journal. A client may send its whole history or only its newest
message; either way the model sees the same thing: a system prompt with summaries of older
exchanges (every four exchanges become a leaf summary, every three leaves merge into one), the last
six exchanges verbatim, and the new message. A restart with `--restore` keeps every conversation.

```sh
affordance serve views/demo/memory_server --set model.url=http://127.0.0.1:8080/v1
curl -s localhost:8000/v1/chat/completions -H 'content-type: application/json' \
  -d '{"model": "m", "messages": [{"role": "user", "content": "My name is Ada."}]}'
```

The header `x-conversation-id` (or the request's `user` field) names a conversation; without one,
every request is the conversation `default`. Streaming (`"stream": true`) returns the reply as one
server-sent event, then `[DONE]`.

The app is [app.tsx](app.tsx), about sixty lines: [views/lib/openai_server.tsx](../../lib/openai_server.tsx)
turns requests into `reply({conversation, message})` calls; the app keeps the exchanges, folds the
old ones with the model, and answers. `tests/serve.rs` runs it against a stand-in model server.
