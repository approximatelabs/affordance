// Types for Affordance apps: the globals and elements an app.tsx can use, for editors and `tsc`.
// `affordance init` writes this file beside the app. It describes; it does not run. The
// contracts are spec/LANGUAGE.md and spec/HARNESS.md in the Affordance repository.

// ---- Elements -------------------------------------------------------------------------------

/** Anything a component may return or an element may hold. */
type AffordanceNode = string | number | boolean | null | undefined | AffordanceFragment | AffordanceNode[];
/** A rendered fragment: text and exact token ids, already laid out. */
interface AffordanceFragment {
  includes(text: string): boolean;
}

declare namespace JSX {
  type Element = AffordanceFragment;
  interface ElementChildrenAttribute {
    children: {};
  }
  /** Handlers are attributes named `on` + a capital letter: `onGo`, `onRun`, `onScroll`. */
  type Handlers = { [K in `on${Capitalize<string>}`]?: (args: any) => any };
  interface Common extends Handlers {
    key?: string | number;
    label?: string;
    id?: string;
    className?: string;
    style?: TokenStyle;
    children?: AffordanceNode;
    [meta: string]: any;
  }
  /** Token-aware style: sizes are tokens of the bound tokenizer. */
  interface TokenStyle {
    maxTokens?: number;
    tokSize?: number;
    overflow?: "clip-end" | "clip-start" | "clip-middle" | "elide" | "scroll" | "drop-start" | "drop-end" | "drop-children";
    more?: string;
    pad?: string;
    display?: "none";
    [property: string]: any;
  }
  interface IntrinsicElements {
    /** A token-budget box: its content is encoded on its own and clipped or dropped to fit. */
    box: Common & { max_tokens?: number; tok_size?: number; overflow?: TokenStyle["overflow"]; more?: string; pad?: string };
    /** A labelled span: its token range is reported; extra attributes travel as meta. */
    span: Common;
    /** One turn of a conversation, and the history that holds them (what a sheet's `history` and `turn` rules style). */
    turn: Common;
    history: Common;
    /** An affordance: an action attached to the tokens of its children. With `tool_name`, a tool the model may call. */
    aff: Common & { name?: string; event?: string; tool_name?: string; description?: string; args_schema?: object };
    /**
     * A picture. With `metrics` (how the model's processor turns a picture into tokens; a
     * template app passes its model's) it is sized by layout: it takes the tokens the style
     * allows (`max-tokens`, `min-tokens`, `object-fit`), the file is sent at that size, and
     * `alt` stands in when it cannot fit. With `tok_size` it is that many placeholder tokens
     * and the file is sent as it is.
     */
    img: Common & { tok_size?: number; t?: string; src?: string; alt?: string; max_tokens?: number | string; min_tokens?: number | string; object_fit?: "scale-down" | "contain" | "none";
      metrics?: { kind: "grid"; cell: number; min: number; max: number } | { kind: "fixed"; tokens: number } | { kind: "shape"; most: number } };
    /**
     * A sound clip; `src` is a `data:audio/...` URL. With `metrics` (a template app passes its
     * model's) it is sized by layout: a clip over `max-tokens` is trimmed (a PCM WAV), from the
     * end, or from the start with `overflow: clip-start`, and `alt` stands in when nothing
     * fits. With `tok_size` it is that many placeholder tokens and the file as it is.
     */
    audio: Common & { tok_size?: number; t?: string; src?: string; alt?: string; max_tokens?: number | string;
      metrics?: { kind: "rate"; ms: number; extra?: number; most?: number } };
    /**
     * A video file in the model's own format; `src` is a `data:video/...` URL (MP4 or MOV). It
     * is laid out as the model's processor writes it (`metrics`, or the tokenizer's rule) and
     * sent as it is; over `max-tokens`, or with no rule, `alt` stands in. With `tok_size` it
     * is that many placeholder tokens.
     */
    video: Common & { tok_size?: number; t?: string; src?: string; alt?: string; max_tokens?: number | string; metrics?: object };
    /** Grouping only. */
    text: Common;
    /** One special token chosen at render time. */
    sp: { t: string };
    /** One raw token id. */
    tok: { id: number };
  }
}

/** A child instance: component `entry` of the file `app`, with its own state and journal. */
declare function Mount(props: {
  key: string;
  app?: string;
  entry?: string;
  model?: string;
  /** The parent's machine the child is bound to, by role (`machine="home"`), or several. */
  machine?: string;
  machines?: string[];
  onStop?: (value: any) => any;
  /** The child: one component element; its props are the child's props. */
  children?: JSX.Element;
  [prop: string]: any;
}): JSX.Element;

/** A prompt fragment: literal text (split on special tokens) and values, in order. */
declare function p(strings: TemplateStringsArray, ...values: any[]): AffordanceFragment;
/** The bound tokenizer's special tokens, as values: `tok.im_start`, `<tok.im_start/>`. */
declare const tok: { [name: string]: any };

// ---- Hooks ----------------------------------------------------------------------------------

/** A field of the instance's state. With a name it is instance-wide; without, local to the component. */
declare function useState<T>(init: T | (() => T), name?: string): [T, (value: T | ((current: T) => T)) => void];
/** Storage that does not render: `[read, write]`. `read()` works in handlers only. */
declare function useRef<T>(init: T, name?: string): [() => T, (value: T | ((current: T) => T)) => void];
/** After a render whose `deps` changed: run `setup` (it may start async work and return a cleanup). */
declare function useEffect(setup: () => void | (() => void), deps?: readonly any[]): void;
/** Run `listener` when the event `name` arrives (a message, a frontend's prompt, a request). */
declare function useEvent(name: string, listener: (payload: any) => any): void;

// ---- The model ------------------------------------------------------------------------------

/** A kind of model call failure that may pass, which a retry policy can try again. */
type RetryableFailure = "unreachable" | "interrupted" | "busy" | "server";
/** What happens after a model call fails. Fields left out are the binding's. */
interface RetryPolicy {
  /** How long, from the first failure, the call is tried again: seconds, or "forever". */
  for_s?: number | "forever";
  /** `[first, longest]` wait before the next attempt, in seconds; it doubles from one to the other. */
  wait_s?: [number, number];
  /** The kinds of failure that are tried again. */
  on?: RetryableFailure[];
  /** Further attempts after the server refuses the request (a 4xx, a malformed reply). */
  refused?: number;
}

interface ModelOptions {
  max_new?: number;
  /** This call's retry policy over its binding's; `false`: the first failure is the app's. */
  retry?: RetryPolicy | false;
  sampling?: object;
  tag?: string;
  priority?: boolean;
  /** Aborting it stops the call; the output is what was generated, with `aborted: true`. */
  signal?: AbortSignal;
  /** Where a hosted reply's pieces are written as they are generated (`res.stream().pipe(..)`). */
  pipe?: object;
}
interface ModelCall {
  name: string;
  args: any;
  id: string;
  problem?: string;
}
interface ModelOutput {
  text: string;
  /** The model's reasoning; null when a hosted reply carries none. */
  reasoning: string | null;
  calls: ModelCall[];
  problems: string[];
  /** The reply ended at its token limit, or was aborted. */
  cut_off: boolean;
  /** The call was stopped by its signal; `text` is what it generated first. */
  aborted?: boolean;
  /** The reply's exact token ids (null for a hosted reply, which has none). */
  ids: number[] | null;
  usage: { prompt_tokens: number; cached_tokens: number; exact: boolean; generated_tokens: number };
  /** The screen the model saw, for the log's `screen` record. */
  screen?: any;
  /** A hosted API's own form of the reply (signed thinking, call ids), opaque to the app. */
  provider_state?: string | null;
  /** What the server measured: `{cached_tokens, ms, first_token_ms, wall_ms}`. */
  server?: any;
  [more: string]: any;
}
/** Send the page's own view to the instance's model, or an ad-hoc view (`model(<Summary/>)`). */
declare function model(options?: ModelOptions): Promise<ModelOutput>;
declare function model(view: JSX.Element | null | undefined, options?: ModelOptions): Promise<ModelOutput>;
/** Run a page output's calls on the handlers of the view the model saw. */
declare function dispatch(output: ModelOutput): Promise<Array<{ name: string; id: string; result: any; error?: boolean }>>;

// ---- The harness ----------------------------------------------------------------------------

interface Machine {
  /** A shell executor takes `{cmd, timeout_s?, env?, stdin?}` and returns `{output, exit, seconds, timed_out, truncated}`. */
  call(args: any, options?: { signal?: AbortSignal }): Promise<any>;
}
declare const harness: {
  /** This instance's id. */
  readonly agent: string;
  /** The model binding this instance runs on; null for one that has none. */
  readonly model: { name: string; template: string | null; template_options: object } | null;
  /** Page calls made so far. */
  readonly turn: number;
  /** The instance's clock, in milliseconds (also `Date.now()`). */
  readonly clock_ms: number;
  /** The instance's environment (plain and secret values). */
  readonly env: { [name: string]: string };
  /** Tokens of the last page prompt. */
  readonly prompt_tokens: number;
  /** The machine roles bound to this instance. */
  readonly machines: string[];
  machine: Machine & { get(role: string): Machine };
  store: {
    /** `key` may be left out: the store then names the entry itself. */
    put(args: { key?: string; text: string }): Promise<any>;
    get(args: { key: string }): Promise<any>;
    search(args: object): Promise<any>;
    list(args?: object): Promise<any>;
  };
  /** An event (`message` by default) to another instance; resolves when it has journaled it. */
  send(to: string | string[], payload: any, options?: { event?: string }): Promise<any>;
  /** Park this task until `ms` pass or one of `events` arrives. */
  wait(args: { ms?: number; events?: string[] }): Promise<{ woke: "timeout" } | { woke: "event"; event: string; payload: any }>;
  /** A read-only copy of other instances' state (needs the deployment's `inspect` grant). */
  inspect(targets: string | string[], options?: object): Promise<any>;
  now(): number;
  random(): number;
  /** Stop this instance with a value. */
  stop(value?: any): void;
  [more: string]: any;
};

// ---- Web platform subset --------------------------------------------------------------------

declare class AbortSignal {
  readonly aborted: boolean;
  readonly reason: any;
  throwIfAborted(): void;
  static abort(reason?: any): AbortSignal;
}
declare class AbortController {
  readonly signal: AbortSignal;
  abort(reason?: any): void;
}
declare class Headers {
  constructor(init?: Record<string, string> | [string, string][] | Headers);
  get(name: string): string | null;
  set(name: string, value: string): void;
  has(name: string): boolean;
  append(name: string, value: string): void;
  delete(name: string): void;
  forEach(callback: (value: string, name: string) => void): void;
}
declare class Response {
  readonly ok: boolean;
  readonly status: number;
  readonly statusText: string;
  readonly url: string;
  readonly headers: Headers;
  text(): Promise<string>;
  json(): Promise<any>;
}
/** Fetch over the machine role `http` the deployment binds; it grants no network access by itself. */
declare function fetch(
  input: string,
  init?: { method?: string; headers?: Record<string, string> | Headers; body?: string; redirect?: "follow" | "error" | "manual"; signal?: AbortSignal },
): Promise<Response>;

/** Lines for whoever watches the app (its trace); never state, never journaled. */
declare const console: {
  log(...values: any[]): void;
  info(...values: any[]): void;
  warn(...values: any[]): void;
  error(...values: any[]): void;
  debug(...values: any[]): void;
};
interface DateConstructor {
  /** The instance's clock in milliseconds: the journal's in a handler, the render's in a render. */
  now(): number;
}

// ---- Builtins -------------------------------------------------------------------------------

declare function countTokens(text: string): number;
/** JSON text with explicit formatting (`{indent, separators, ascii, sortKeys}`). */
declare function formatJSON(value: any, options?: { indent?: number | string; separators?: [string, string]; ascii?: boolean; sortKeys?: boolean }): string;
/** The text Python prints for a value (`True`, `None`, `{'a': 1}`), as official templates write it. */
declare function formatPythonValue(value: any): string;
/** A deep copy of plain data. */
declare function structuredClone<T>(value: T): T;
/** `memoScan(items, init, step, deps)`: a fold over a growing list that keeps its work between renders. */
declare function memoScan(items: any[], init: any, step: (state: any, item: any, index: number) => any, deps?: any): { state: any; [more: string]: any };
/** For strategy and template-app authors (`views/strategies/turn_strategy.tsx`): a log as turns, a turn plan, a screen's tools, a template's input, a hosted API's messages, and a value's text. */
declare function logTurns(log: any[], skip?: string[]): any;
declare function turnPlan(app: any, turns: any[]): any;
declare function asChatTools(children: any, options?: object): any;
declare function templateInput(children: any, system?: any, generatedOk?: boolean, alternate?: any): any;
declare function messagesJson(input: any): any;
declare function textContent(value: any): string;
declare function tokens(ids: number[]): AffordanceFragment;
declare function formatNow(format: string): string;
declare function isFragment(value: any): value is AffordanceFragment;
declare function isPlainObject(value: any): boolean;
declare function nameOf(fn: Function): string;
declare function docOf(fn: Function): string;
declare function schemaOf(fn: Function): object;

// ---- The bundled library (`@affordance/...`) ------------------------------------------------
// The binary resolves these imports; their source is views/ in the Affordance repository.

declare module "@affordance/lib/log.tsx" {
  export function useLog(name?: string): {
    records: any[];
    append(...records: any[]): void;
    call(view?: JSX.Element, options?: ModelOptions): Promise<ModelOutput>;
    dispatch(output: ModelOutput): Promise<any[]>;
    step(view?: JSX.Element, options?: ModelOptions): Promise<{ out: ModelOutput; results: any[] }>;
    close(output: ModelOutput, why?: string): void;
    isOpen(): boolean;
    held(): any[];
    release(upto: number, keep?: string[]): Promise<number>;
    read(from: number, to: number): Promise<any[]>;
    get(id: string): Promise<any>;
    reset(records: any[]): void;
  };
  /** A record's content as text. */
  /** Content (a string, text and image parts, or nothing) as plain text, with `[image]` for an image. */
  export function contentText(content: any): string;
  /** Whether the model owes an answer: the last record is an input or a tool result. */
  export function unanswered(records: any[]): boolean;
  export function recordText(record: any): string;
  /** The log records of one model reply (its `model` record). */
  export function turnRecords(out: ModelOutput): any[];
  /** A tool result (`{id, name, result, error?}`) as an `action` record. */
  export function actionRecord(result: any): any;
  /** The calls of an output that was not dispatched, each answered "not run (why)". */
  export function notRun(out: ModelOutput, why: string): any[];
}
declare module "@affordance/lib/loop.tsx" {
  /**
   * Ask the model, run its calls, repeat. A reply with no tool call ends the instance with the
   * reply as its answer (`{answer: text}`); `idle(out)` returning true asks again instead (change
   * the page first: a nudge, a wait).
   */
  export function useLoop(log: any, options?: { before?: () => any; idle?: (out: ModelOutput) => any; name?: string }): void;
  export function useConversation(log: any, options?: { name?: string }): void;
}
declare module "@affordance/lib/model_layer.tsx" {
  /** Serve another harness's model calls: the event `generate`, answered on the machine `ui`. */
  export function useModelLayer(): void;
  /** Chat messages (`{role, content, tool_calls?}`) as log records. */
  export function chatRecords(messages: any[]): any[];
}
declare module "@affordance/lib/frontend.tsx" {
  export function useFrontend(
    log: any,
    options?: { name?: string; before?: () => any; call?: () => ModelOptions },
  ): { signal(): AbortSignal | undefined };
  export function ask(question: any, fallback?: any): Promise<any>;
}
declare module "@affordance/lib/model.tsx" {
  export function ModelContext(props: {
    system?: string;
    log?: any[];
    strategy?: string;
    callable?: boolean;
    children?: AffordanceNode;
  }): JSX.Element;
  /** The template apps by registry key (`views/template_apps/index.json`). */
  export const templateApps: { [key: string]: Function };
}
declare module "@affordance/lib/http_server.tsx" {
  /** A request served by `affordance serve`. */
  export interface ServedRequest {
    id: string;
    method: string;
    path: string;
    query: string;
    headers: { [name: string]: string };
    body: string;
  }
  export interface ResponseStream {
    write(chunk: string): Promise<any>;
    /** The `pipe` option of a model call: its pieces, framed, are written to this response. */
    pipe(frames: { text?: string; reasoning?: string; tool_call?: string }): object;
    end(): Promise<any>;
  }
  export interface Responder {
    isAnswered(): boolean;
    isStreaming(): boolean;
    json(body: any, status?: number, headers?: object): Promise<any>;
    text(body: string, status?: number, headers?: object): Promise<any>;
    stream(status?: number, headers?: object): Promise<ResponseStream>;
  }
  /** Where a piece goes in a pipe's frame. */
  export const PIECE: string;
  export const PIECE_INDEX: string;
  export const PIECE_ID: string;
  export const PIECE_NAME: string;
  export function responder(client: any, id: string): Responder;
  /** Handle every request with `handler(req, res)`. */
  export function useRequests(handler: (req: ServedRequest, res: Responder) => any): void;
}
declare module "@affordance/lib/openai_server.tsx" {
  /** The text of a chat message's content. */
  export function messageText(content: any): string;
  /** An OpenAI-compatible endpoint (`/v1/models`, `/v1/chat/completions`); the app supplies the reply. */
  export function useOpenAIServer(options: {
    name?: string;
    reply: (ask: { conversation: string; message: string; messages: any[]; request: any; pipe?: object }) => any;
  }): void;
}
declare module "@affordance/lib/tool.tsx" {
  export function Tool(props: { fn: Function; name?: string; description?: string; schema?: object; children?: AffordanceNode; [meta: string]: any }): JSX.Element;
}
declare module "@affordance/tools/machine.tsx" {
  export function machineTools(machine: Machine, options?: { signal?: () => AbortSignal | undefined }): {
    shell(args: { command: string; timeout_s?: number }): Promise<any>;
  };
  export function wait(args: { seconds?: number; events?: string[] }): Promise<any>;
}
declare module "@affordance/tools/files.tsx" {
  export function fileTools(
    machine: Machine,
    options?: { signal?: () => AbortSignal | undefined; permit?: (tool: string, path: string) => any },
  ): {
    read(args: { path: string; offset?: number; limit?: number }): Promise<string>;
    write(args: { path: string; content: string }): Promise<string>;
    edit(args: { path: string; old_string: string; new_string: string; replace_all?: boolean }): Promise<string>;
  };
}
declare module "@affordance/*";
