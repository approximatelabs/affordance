// Permanent memory for agents, after OptMem (github.com/VictorTaelin/OptMem), as an instance tree.
//
// The memory is this root's state: an append-only log of one-line memories and a binary tree of
// one-line summaries over it (block [lo, hi) summarizes its two halves). Sessions are its children,
// one at a time; each wakes with the memory's cover (recent memories verbatim, older ones collapsed
// into tree nodes, `views/strategies/context.tsx` `cover`, the same tiling as OptMem's) and can
// recall, zoom and note. Differences from OptMem, which the harness makes possible:
//   - Compression happens here, as a model call (a model() call) when a note
//     makes a block complete. A session never stops its work to "nap".
//   - Who may write memory is structural, not a prompt rule: a session writes through the `onNote`
//     callback this root passes it. A helper it mounts gets no `onNote`, so it cannot write.
//   - The memory outlives sessions because the root does: a session's stop (`onStop`) mounts the
//     next, and a crash resumes the whole tree from its journals.
import { TurnStrategy } from "@affordance/strategies/turn_strategy.tsx";
import { cover, pendingBlocks } from "@affordance/strategies/context.tsx";
import { MessagesApp } from "@affordance/template_apps/messages.tsx";
import { GLM53App } from "@affordance/template_apps/glm53.tsx";
import { Session } from "./session.tsx";

// One line in place of a block of memories.
function Compress({ template, lo, hi, body, chars }) {
  const text = `Summarize memories #${lo}-${hi - 1} in one line of at most ${chars} bytes.\nKeep what will still matter later and leave out the rest. Add nothing that is not in them.\n\n${body}\n\nReply with the line only.`;
  if (template == "glm53") return <GLM53App system="You maintain a memory."><TurnStrategy strategy="unrolled" log={[]}>{text}</TurnStrategy></GLM53App>;
  return <MessagesApp system="You maintain a memory."><TurnStrategy strategy="unrolled" log={[]}>{text}</TurnStrategy></MessagesApp>;
}

// What `wake` prints: the cover of the log, one line per block, finest near now.
export function wakeText(log, tree, lines) {
  if (log.length == 0) return "(no memories yet)";
  return cover(log.length, lines).map(([lo, hi]) =>
    hi - lo == 1 ? `#${lo} ${log[lo]}` : `#${lo}-${hi - 1} ${tree[`${lo}-${hi}`] ?? "(summary pending)"}`).join("\n");
}

export default function Memory({ sessions, template = "messages", wake_lines = 96, chars = 280 }) {
  const [log, setLog] = useState([], "log");
  const [tree, setTree] = useState({}, "tree");
  const [current, setCurrent] = useState(0, "current");
  // A note is appended at once (a functional update: notes from sessions running side by side all
  // land). Compression is its own step: while a block of notes is complete and unsummarized,
  // summarize the smallest one from its two halves, one at a time.
  const note = (text) => { setLog(l => [...l, String(text).slice(0, chars)]); };
  const [busy, setBusy] = useState(false, "compressing");
  const pending = pendingBlocks(log.length, tree);
  useEffect(() => {
    if (busy || !pending.length) return;
    const [lo, hi] = pending[0];
    const mid = (lo + hi) / 2;
    const half = (a, b) => b - a == 1 ? `  #${a} ${log[a]}` : `  #${a}-${b - 1} ${tree[`${a}-${b}`]}`;
    const body = half(lo, mid) + "\n" + half(mid, hi);
    setBusy(true);
    const compress = async () => {
      try {
        const r = await model(<Compress template={template} lo={lo} hi={hi} chars={chars} body={body}/>);
        setTree(t => ({...t, [`${lo}-${hi}`]: r.text.trim().split("\n")[0].slice(0, chars)}));
      } catch (e) {
        // A summary that fails for good ends the run with the reason. Left to the effect below,
        // it would be asked for again at once, without end.
        harness.stop({reason: "compress_error", error: e.message});
      } finally {
        setBusy(false);
      }
    };
    compress();
  }, [busy, log.length, Object.keys(tree).length]);
  if (current >= sessions.length) return "";
  return <Mount key={`session-${current}`} model="default">
    <Session task={sessions[current]} template={template} wake={wakeText(log, tree, wake_lines)} memories={log} tree={tree}
             onNote={note} onStop={() => { setCurrent(c => c + 1); }}/>
  </Mount>;
}
