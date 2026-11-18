// The supported Web API surface is written in ordinary JavaScript. The host supplies I/O;
// this code is also exercised unchanged in Node as a differential oracle.

function headerName(value) {
  const name = String(value);
  if (!/^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/.test(name)) throw new TypeError("Invalid header name");
  return name.toLowerCase();
}

function headerValue(value) {
  const text = String(value).replace(/^[\t\n\r ]+|[\t\n\r ]+$/g, "");
  for (const character of text) {
    if (character.codePointAt(0) > 255 || character === "\u0000" || character === "\r" || character === "\n") {
      throw new TypeError("Invalid header value");
    }
  }
  return text;
}

class Headers {
  #list;
  constructor(init) {
    this.#list = [];
    if (init instanceof Headers) {
      this.#list = init.#list.map(pair => [...pair]);
    } else if (Array.isArray(init)) {
      for (const pair of init) {
        if (!Array.isArray(pair) || pair.length !== 2) throw new TypeError("A header pair must contain two values");
        this.#list.push([headerName(pair[0]), headerValue(pair[1])]);
      }
    } else if (init !== undefined) {
      if (init === null || (typeof init !== "object" && typeof init !== "function")) throw new TypeError("Invalid Headers initializer");
      for (const name of Object.keys(init)) this.#list.push([headerName(name), headerValue(init[name])]);
    }
  }
  append(name, value) { this.#list.push([headerName(name), headerValue(value)]); }
  delete(name) {
    const key = headerName(name);
    this.#list = this.#list.filter(pair => pair[0] !== key);
  }
  get(name) {
    const key = headerName(name);
    const values = this.#list.filter(pair => pair[0] === key).map(pair => pair[1]);
    return values.length ? values.join(", ") : null;
  }
  has(name) { return this.get(name) !== null; }
  set(name, value) {
    const key = headerName(name), text = headerValue(value);
    let found = false;
    this.#list = this.#list.filter(pair => {
      if (pair[0] !== key) return true;
      if (found) return false;
      found = true;
      pair[1] = text;
      return true;
    });
    if (!found) this.#list.push([key, text]);
  }
  getSetCookie() { return this.#list.filter(pair => pair[0] === "set-cookie").map(pair => pair[1]); }
  forEach(callback, thisArg) {
    if (typeof callback !== "function") throw new TypeError("Headers.forEach requires a callback");
    let index = 0;
    while (true) {
      const names = this.#list.map(pair => pair[0]).sort().filter((name, i, all) => i === 0 || name !== all[i - 1]);
      if (index >= names.length) return;
      const name = names[index++];
      // Fetch iteration keeps separate Set-Cookie entries.
      if (name === "set-cookie") {
        for (const value of this.getSetCookie()) callback.call(thisArg, value, name, this);
      } else callback.call(thisArg, this.get(name), name, this);
    }
  }
}

class ImmutableHeaders extends Headers {
  append() { throw new TypeError("Headers are immutable"); }
  set() { throw new TypeError("Headers are immutable"); }
  delete() { throw new TypeError("Headers are immutable"); }
}

class Response {
  #body;
  #used;
  #headers;
  #status;
  #statusText;
  constructor(body = null, init = {}) {
    const options = init == null ? {} : init;
    if (typeof options !== "object" && typeof options !== "function") throw new TypeError("Invalid Response initializer");
    const number = options.status === undefined ? 200 : Number(options.status);
    const status = Number.isFinite(number) ? ((Math.trunc(number) % 65536) + 65536) % 65536 : 0;
    if (status < 200 || status > 599) throw new RangeError("Response status must be between 200 and 599");
    const statusText = options.statusText === undefined ? "" : String(options.statusText);
    if (!/^[\t\x20-\x7e\x80-\xff]*$/.test(statusText)) throw new TypeError("Invalid response statusText");
    if (body !== null && [204, 205, 304].includes(status)) throw new TypeError("This status cannot have a body");
    this.#body = body === null ? null : String(body);
    this.#used = false;
    this.#headers = new Headers(options.headers);
    if (body !== null && !this.#headers.has("content-type")) this.#headers.set("content-type", "text/plain;charset=UTF-8");
    this.#status = status;
    this.#statusText = statusText;
  }
  get status() { return this.#status; }
  get statusText() { return this.#statusText; }
  get ok() { return this.#status >= 200 && this.#status <= 299; }
  get headers() { return this.#headers; }
  get url() { return ""; }
  get redirected() { return false; }
  get type() { return "default"; }
  get bodyUsed() { return this.#used; }
  get body() {
    if (this.#body === null) return null;
    throw new TypeError("ReadableStream response bodies are not implemented; use text() or json()");
  }
  async text() {
    if (this.#used) throw new TypeError("Body has already been consumed");
    if (this.#body === null) return "";
    this.#used = true;
    return this.#body.charCodeAt(0) === 65279 ? this.#body.slice(1) : this.#body;
  }
  async json() { return JSON.parse(await this.text()); }
  clone() {
    if (this.#used) throw new TypeError("Body has already been consumed");
    return new Response(this.#body, {status: this.#status, statusText: this.#statusText, headers: this.#headers});
  }
  static json(data, init = {}) {
    const body = JSON.stringify(data);
    if (body === undefined) throw new TypeError("Value is not JSON serializable");
    const headers = new Headers(init == null ? undefined : init.headers);
    if (!headers.has("content-type")) headers.set("content-type", "application/json");
    return new Response(body, {...init, headers});
  }
}

class NetworkResponse extends Response {
  #raw;
  #networkHeaders;
  #nullBody;
  constructor(raw, nullBody) {
    const pairs = [];
    for (const name of Object.keys(raw.headers)) {
      for (const value of raw.headers[name]) pairs.push([name, value]);
    }
    super(nullBody ? null : raw.body, {status: raw.status, statusText: raw.statusText || "", headers: pairs});
    this.#raw = raw;
    this.#nullBody = nullBody;
    this.#networkHeaders = new ImmutableHeaders(pairs);
  }
  get headers() { return this.#networkHeaders; }
  get url() { return this.#raw.url || ""; }
  get type() { return "basic"; }
  clone() {
    if (this.bodyUsed) throw new TypeError("Body has already been consumed");
    return new NetworkResponse(this.#raw, this.#nullBody);
  }
}

// `AbortController` and `AbortSignal`. A signal is named by an id its instance keeps, with whether it
// was aborted and why: a controller kept in state is frozen, as all state is, and still aborts.
// Calls take one as `{signal}`: `model(view, {signal})`, `machine.call(args, {signal})`, `fetch`.
function abortError() {
  const error = new Error("This operation was aborted");
  error.name = "AbortError";
  return error;
}

class AbortSignal {
  constructor(id) {
    if (!Number.isInteger(id) || id < 0) throw new TypeError("Illegal constructor");
    this.__signal = id;
  }
  get aborted() { return harness.signal.state(this.__signal) !== null; }
  get reason() {
    const state = harness.signal.state(this.__signal);
    if (state === null) return undefined;
    return "reason" in state ? state.reason : abortError();
  }
  throwIfAborted() {
    if (this.aborted) throw this.reason;
  }
  static abort(reason) {
    const signal = new AbortSignal(harness.signal.create());
    harness.signal.abort(signal.__signal, reason);
    return signal;
  }
}

class AbortController {
  constructor() { this.signal = new AbortSignal(harness.signal.create()); }
  abort(reason) { harness.signal.abort(this.signal.__signal, reason); }
}

async function fetchBound(role, input, init = {}) {
  if (input === undefined) throw new TypeError("fetch requires an input URL");
  if (init === null) init = {};
  if (typeof init !== "object" && typeof init !== "function") throw new TypeError("Invalid fetch options");
  for (const key of Object.keys(init)) {
    if (!["method", "headers", "body", "redirect", "signal"].includes(key)) throw new TypeError("Unsupported fetch option: " + key);
  }
  const url = String(input).split("#")[0];
  let method = init.method === undefined ? "GET" : String(init.method);
  if (!/^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/.test(method)) throw new TypeError("Invalid HTTP method");
  const upper = method.toUpperCase();
  if (["CONNECT", "TRACE", "TRACK"].includes(upper)) throw new TypeError("Forbidden HTTP method");
  if (["DELETE", "GET", "HEAD", "OPTIONS", "POST", "PUT"].includes(upper)) method = upper;
  const body = init.body == null ? null : String(init.body);
  if (body !== null && ["GET", "HEAD"].includes(method)) throw new TypeError("GET and HEAD cannot have a body");
  const redirect = init.redirect === undefined ? "follow" : String(init.redirect);
  if (!["follow", "error", "manual"].includes(redirect)) throw new TypeError("Invalid redirect mode");
  const headers = new Headers(init.headers);
  if (body !== null && !headers.has("content-type")) headers.set("content-type", "text/plain;charset=UTF-8");
  const record = {};
  headers.forEach((value, name) => { record[name] = value; });
  const request = {path: url, method, headers: record};
  if (body !== null) request.body = body;
  let raw;
  try { raw = await harness.machine.get(role).call(request, {signal: init.signal}); }
  catch (error) {
    if (["AbortError", "TimeoutError"].includes(error.name)) throw error;
    throw new TypeError(error.message);
  }
  if (redirect !== "manual" && [301, 302, 303, 307, 308].includes(raw.status)) {
    throw new TypeError("Redirect blocked by HTTP binding policy");
  }
  return new NetworkResponse(raw, method === "HEAD" || [204, 205, 304].includes(raw.status));
}

async function fetch(input, init) {
  return await fetchBound("http", input, init);
}

function createFetch(role) {
  if (typeof role !== "string" || !role.length) throw new TypeError("createFetch requires a named HTTP binding");
  return async (input, init) => await fetchBound(role, input, init);
}
export { fetch, Headers, Response, createFetch, AbortController, AbortSignal };
