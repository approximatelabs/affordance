// A chat model with a memory longer than its context, behind an OpenAI-compatible endpoint
// (`affordance serve`). Any OpenAI client can talk to it; each conversation is kept here, in the
// app's journal, so the client may send only its last message.
//
// What the model sees for a reply: the system prompt with summaries of older exchanges (a tree:
// every LEAF exchanges become a leaf summary, every three leaves merge into one), the last KEEP
// exchanges verbatim, and the new message. Nothing is lost: the journal holds every exchange, and a
// restart restores the same memory.
import { useOpenAIServer } from "@affordance/lib/openai_server.tsx";
import { ModelContext } from "@affordance/lib/model.tsx";

const KEEP = 6;
const LEAF = 4;

function exchangesText(exchanges, lo, hi) {
  let s = "";
  for (let i = lo; i < hi; i++) s += `[exchange ${i}]\nUSER: ${exchanges[i].user}\nASSISTANT: ${exchanges[i].assistant}\n\n`;
  return s;
}

function memoryText(nodes) {
  if (nodes.length === 0) return "";
  return "\n\nWhat you remember from earlier in this conversation, oldest first:\n\n" +
    nodes.map(n => `[exchanges ${n.lo} to ${n.hi - 1}]\n${n.text}`).join("\n\n");
}

// Fold the oldest unsummarized exchanges into the tree while more than KEEP + LEAF are verbatim.
async function fold(c, summarize) {
  let {exchanges, start, nodes} = c;
  while (exchanges.length - start >= KEEP + LEAF) {
    const leaf = await summarize(exchangesText(exchanges, start, start + LEAF));
    nodes = [...nodes, {lo: start, hi: start + LEAF, level: 0, text: leaf.trim()}];
    start += LEAF;
    const leaves = nodes.filter(n => n.level === 0);
    if (leaves.length >= 3) {
      const merged = await summarize(leaves.map(n => `[exchanges ${n.lo} to ${n.hi - 1}]\n${n.text}`).join("\n\n"));
      nodes = [...nodes.filter(n => n.level !== 0), {lo: leaves[0].lo, hi: leaves[leaves.length - 1].hi, level: 1, text: merged.trim()}];
    }
  }
  return {exchanges, start, nodes};
}

export default function MemoryServer({ system = "You are a helpful assistant with a long memory.", name = "affordance-memory" }) {
  const [read, write] = useRef({}, "conversations");
  const [folding, setFolding] = useRef({}, "folding");
  const summarize = async (text) => (await model(
    <ModelContext system="Summarize this part of a conversation in a few sentences. Keep names, facts, numbers and decisions; drop pleasantries." strategy="unrolled" callable={false}
      log={[{kind: "input", content: text}]}>{null}</ModelContext>, {max_new: 400})).text;
  useOpenAIServer({ name, reply: async ({ conversation, message, pipe }) => {
    const c = read()[conversation] || {exchanges: [], start: 0, nodes: []};
    const log = [];
    for (const e of c.exchanges.slice(c.start)) log.push({kind: "input", content: e.user}, {kind: "model", content: e.assistant});
    log.push({kind: "input", content: message});
    const out = await model(<ModelContext system={system + memoryText(c.nodes)} strategy="unrolled" callable={false} log={log}>{null}</ModelContext>,
      // A streamed request: the reply reaches the client as it is generated.
      pipe ? {pipe} : {});
    const answer = out.text.trim();
    // Two requests on one conversation may be answered at once. The exchange is added to the
    // conversation as it is now (read and written with no wait between), and one fold runs at a
    // time: a fold keeps whatever was added while it summarized.
    const empty = {exchanges: [], start: 0, nodes: []};
    write(all => ({...all, [conversation]: {...(all[conversation] || empty),
      exchanges: [...(all[conversation] || empty).exchanges, {user: message, assistant: answer}]}}));
    if (!folding()[conversation]) {
      setFolding(f => ({...f, [conversation]: true}));
      try {
        const folded = await fold(read()[conversation], summarize);
        write(all => ({...all, [conversation]: {...folded, exchanges: all[conversation].exchanges}}));
      } finally {
        setFolding(f => ({...f, [conversation]: false}));
      }
    }
    return pipe ? {text: answer, piped: true} : answer;
  }});
  return <box>{"serving"}</box>;
}
