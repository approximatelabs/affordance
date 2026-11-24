# Why Affordance is shaped like a browser

This document explains where the line between the engine and an app is, and why. It is for
anyone extending Affordance, person or model, who wants to predict the answer to "where does this
feature go?" before asking. The contracts are in [spec/](../spec/CONTRACTS.md); this is the
reasoning behind them, shown through decisions that were actually made.

## The claim

A web browser renders the internet's state into pixels for a person, and lets the page react to
what the person does. Affordance renders an app's state into tokens for a model, and lets the app
react to what the model does. The analogy is not decoration. The web platform has already
answered, under pressure from millions of applications, most of the questions an agent harness
faces: how a page waits, fetches, cancels, talks to other pages, survives a reload, and is watched
by a developer. When a need appears here, the first question is what the web's answer was.

| The web | Affordance |
|---|---|
| A page: markup, state, event handlers | An app: TSX, `useState`, handlers on `<aff>` and `useEvent` |
| Layout: CSS fits content to a viewport | Layout: token style sheets fit content to a context window |
| A link or button the user can see and use | An affordance: an action attached to tokens that survived layout |
| The event loop | One instance: ordered inputs, tasks, microtasks |
| `fetch` | `model()` (a fetch of a view), `harness.machine.call`, `fetch` |
| `AbortController` | `AbortController` |
| `postMessage` between windows | `harness.send` between instances; `Control::post` from outside |
| An iframe | `<Mount>`: a child instance with its own state and journal |
| A Service Worker answering `fetch` events | `affordance serve`: the event `request`, answered on the machine `client` |
| DevTools: console, network panel, the protocol | `console`, the observer, `affordance host` |
| Reload, session restore | The journal: replay rebuilds the instance exactly |

## Three owners

Every feature belongs to one of three owners. Most disagreements about a design are really
disagreements about which.

1. **The app** (TSX, including the libraries in `views/`). What the model sees, which tools exist,
   when the model is called, how history is kept and folded, what needs the user's permission. A
   website's product decisions.
2. **The engine** (the language, layout, the instance's event loop, effects, the journal). What
   every app needs and no app could build for itself: exact tokens, deterministic replay,
   cancellation that actually stops work. The browser.
3. **The deployment and its host program** (`affordance.yaml`, or a program embedding the
   runtime). Which model server, which machines, which credentials, what an agent is allowed to
   reach. The user's computer and network.

A browser does not know what a shopping cart is. The engine does not know what a coding agent,
a conversation, a permission prompt or a file is.

## The test for a new need

1. **Name the web's primitive for it.** If a web application would solve this with standard APIs,
   say which.
2. **Compose first.** Most needs are an arrangement of primitives that already exist. Write that
   arrangement. If it works, the feature is library TSX or deployment configuration, and the
   engine does not change.
3. **If something is truly missing, add the smallest general primitive**: nothing in it names a
   product, a vendor, a file or a process; it is useful beyond the case that prompted it; a replay
   reproduces it; it behaves under cancellation, close and restore.
4. **Check the ordinary cases**: a chat bot, an autonomous tool-using agent, a fleet of agents, a
   restored journal. If one needs a workaround, the primitive is wrong.

## Decisions, and why they went the way they did

**Stopping a generation.** The need: the user presses Escape; stop now, and keep what was
written. The tempting design is a harness call, `interrupt()`. The web's answer is
`fetch(url, {signal})` with an `AbortController`, so `model()`, machine calls and `fetch` take a
signal. Two details follow from taking the parallel seriously. An aborted `model()` *resolves*
with the partial output instead of throwing, because the app is the reader of a stream and a
stream's reader keeps the chunks it read. And whether a signal is aborted is kept by the
instance, not in the controller object, because what an app keeps in state is frozen and must be
reproducible from the journal.

**Time and randomness.** A handler that read the wall clock would behave differently on replay.
So `Date.now()` is the instance's clock, which advances only with journaled inputs, and
`Math.random()` is refused with a message naming `harness.random()`. The rule behind
both: everything a handler observes arrives as a journaled input.

**`console.log`.** An app needs to say things to its developer. If those lines were state, they
would change renders; if they were journaled, debugging would change the journal. A browser's
console is neither: it is the DevTools view of a page. Here, console lines go to the instance's
trace, which observers see, and nowhere else.

**The loop is not the engine's.** The runtime never calls the model on its own. An autonomous
agent (`useLoop`), a conversation (`useConversation`) and a chat driven by a frontend
(`useFrontend`) are each a few dozen lines of library TSX over the same primitives, and an app
can copy and change them. A browser has no opinion about when a page fetches.

**Fitting the context window.** Dropping old history and eliding old tool results is layout, as
`overflow` is in CSS: the engine does it, exactly, in tokens. Deciding to summarize, and what a
summary says, is the app's: it listens for the layout event and makes an ordinary model call.

**Serving HTTP.** The need: put an app behind an HTTP endpoint. The web's answer is a Service
Worker's `fetch` event and `respondWith`. So a request is an event to the app, and the response
is a call on a machine named `client`. Routing, JSON, and an OpenAI-compatible API are library
TSX. The one thing that was missing was a way for a program outside to post an event into a
running runtime; that became `Control::post`, which knows nothing about HTTP.

**A frontend.** A terminal UI needs to drive an agent and watch it. The web's answer is the
DevTools protocol: a second channel that sees the page without being the page. `affordance host`
is that channel. A permission prompt was not added to it. Instead the frontend is a machine,
`ui`, and an app that wants permission calls it and waits. Which actions need permission is
therefore the app's decision, where a product decision belongs.

**What the frontend is told.** A frontend wants to know when a tool starts, and to redraw a
conversation after a restart. Both were first requested as protocol events. Both turned out to
be compositions: the chat library tells the frontend the model's calls before it runs them, and
answers the event `history` with its log. The protocol did not change.

**File tools.** A coding agent wants `read`, `write` and `edit`. They are `views/tools/files.tsx`:
library functions over a machine's shell. Nothing about them needed the engine, so the engine has
no notion of a file.

**A wait that watches a process.** A client wanted `wait` to end when a process on a guest
machine exited. It was built into the engine, then removed: watching a guest is a property of
that client's machines, not of waiting. The client composes it from a machine call and
`harness.wait`.

## How to tell you are on the wrong side of the line

- The engine change mentions a product noun: file, repository, permission, conversation, tool
  name, vendor.
- The feature could be written in an app today, only less conveniently. Then it is a library.
- A replay would not reproduce it, or would need the outside world to cooperate.
- It makes one of the ordinary cases special.

And the opposite error, rarer but real: an app is asked to do what only the engine can do
honestly, such as counting tokens by estimate, cancelling work by ignoring its result, or
remembering by re-reading the outside world.
