// Runs unchanged with Node's native Web APIs and Affordance's embedded standard library.
async function webCases() {
  const out = [];
  const capture = async fn => {
    try { return {value: await fn()}; }
    catch (error) { return {error: error.name}; }
  };
  const headers = new Headers([["X-B", " one "], ["x-b", "two"], ["X-A", "a"], ["Set-Cookie", "a=1"], ["set-cookie", "b=2"]]);
  const entries = [];
  headers.forEach((value, name, owner) => entries.push([name, value, owner === headers]));
  out.push(entries, headers.get("X-B"), headers.get("missing"), headers.has("x-b"), headers.getSetCookie());
  const copy = new Headers(headers);
  copy.set("X-B", " changed ");
  copy.append("X-B", "third");
  copy.delete("x-a");
  out.push(copy.get("x-b"), headers.get("x-b"), copy.has("x-a"));
  for (const init of [null, [["x"]], {"bad name": "x"}, {x: "a\r\nb"}, {x: "世界"}, {x: "\x01"}, {x: "é"}, {x: "\n value\r\n"}]) {
    out.push(await capture(() => new Headers(init).get("x")));
  }
  const receiver = {name: "receiver"};
  const calls = [];
  new Headers({a: "1"}).forEach(function(value, key) {calls.push([this.name, key, value]);}, receiver);
  out.push(calls);
  for (const status of [200, 200.9, 201, 299, 300, 599, 600, 0, -1, null, "201"]) {
    out.push(await capture(() => {
      const response = new Response(null, {status});
      return [response.status, response.statusText, response.ok, response.type, response.url, response.redirected];
    }));
  }
  for (const init of [null, 1, "x", true, {status: 204}, {statusText: "bad\r\nreason"}]) {
    out.push(await capture(() => new Response("body", init).status));
  }
  for (const body of [null, "", "héllo", "\ufeffhello"]) {
    const response = new Response(body);
    const clone = response.clone();
    out.push([response.headers.get("content-type"), response.bodyUsed, await response.text(), response.bodyUsed,
      await clone.text(), await capture(() => response.text()), await capture(() => response.clone().status)]);
  }
  const json = new Response('{"ok":true}', {status: 201, statusText: "Created", headers: {"Content-Type": "application/json"}});
  out.push([json.headers.get("content-type"), json.ok, json.status, json.statusText, await json.json(), json.bodyUsed]);
  const invalid = new Response("not json");
  out.push(await capture(() => invalid.json()), invalid.bodyUsed);
  const built = Response.json({a: [1, true]}, {status: 202, headers: {"X-Test": "yes"}});
  out.push([built.status, built.headers.get("content-type"), built.headers.get("x-test"), await built.json()]);
  out.push(await capture(() => Response.json(undefined)));
  const immutableMetadata = new Response("ok");
  try { immutableMetadata.status = 500; } catch (error) {}
  out.push(immutableMetadata.status);
  return out;
}

export { webCases };
