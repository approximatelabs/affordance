// The smallest served app: GET /hello answers JSON, POST /echo answers its body, /count streams.
import { useRequests } from "@affordance/lib/http_server.tsx";

export default function Echo() {
  const [served, setServed] = useState(0, "served");
  useRequests(async (req, res) => {
    setServed(n => n + 1);
    if (req.method === "GET" && req.path === "/hello") return await res.json({hello: "world", served: served + 1});
    if (req.method === "POST" && req.path === "/echo") return await res.text(req.body);
    if (req.path === "/count") {
      const out = await res.stream(200, {"content-type": "text/plain"});
      for (const n of [1, 2, 3]) await out.write(`${n}\n`);
      return await out.end();
    }
    return await res.json({error: "not found"}, 404);
  });
  return <box>{`served ${served} requests`}</box>;
}
