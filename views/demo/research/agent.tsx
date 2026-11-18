import { useLog } from "@affordance/lib/log.tsx";
import "./agent.tss";
// A deep-research agent with a context policy: the BrowseComp-Plus ladder (rungs R0 to R4) on the
// instance runtime. It answers a question by searching a corpus with `bcp_search` and
// `bcp_get_document`, programs on its machine, through one `bash` tool; it submits with
// `echo COMPLETE_TASK_AND_SUBMIT_FINAL_OUTPUT` after writing ~/answer.txt, and reports the answer
// to its parent (`onDone`). The window is the model binding's `context` (less `max_new`); the
// policies are layout (agent.tss) or react to it (the layout event, when a prompt passes `trig`
// of the window):
//   "none"     R0: nothing (the run ends when the context is full).
//   "trunc"    R1: the oldest whole turns drop, in steps, past 85% of the window (agent.tss).
//   "compact"  R2: one model call writes a handoff summary (model(<Handoff/>)); it replaces the turns.
//   "tree"     R4: each fold writes a leaf summary; three leaves merge into one; `recall` reads a
//              past turn in full.
//   "fold"     R5: context folding (FoldAgent): `branch` opens a side task, `return` folds its turns
//              away to one summary; over `trig` in a branch, the agent is told to return; in the
//              main line, the oldest turns drop as in "trunc".
//   lazy       R3 (any policy): the search commands are native tools, loaded on request.
// History is the app's log (useLog); state holds the cut point, the summaries and the loaded tools.
// `template` picks the model family: "messages" (a hosted chat API), "qwen36", "glm53".
import { TurnStrategy } from "@affordance/strategies/turn_strategy.tsx";
import { useLoop } from "@affordance/lib/loop.tsx";
import { MessagesApp } from "@affordance/template_apps/messages.tsx";
import { Qwen36App } from "@affordance/template_apps/qwen36.tsx";
import { GLM53App } from "@affordance/template_apps/glm53.tsx";
import { BRANCH_OVERFLOW, FOLD, HANDOFF, branches, compact, openBranch, project, recall, returnBranch, summaryTree, turnStarts } from "@affordance/strategies/context.tsx";

const SUBMIT_CMD = "echo COMPLETE_TASK_AND_SUBMIT_FINAL_OUTPUT";
const BASH_DESC = "Execute a bash command in a persistent shell. The working directory and environment variables are preserved between calls.";
const SYSTEM = "You are a helpful assistant that can interact with a computer.\n\nYour response must include a THOUGHT section before your action where you\nexplain your reasoning. After the THOUGHT, you must call a tool\nwith EXACTLY ONE action (multiple bash commands chained with `&&` or `||`\ncount as a single action).\n\nWhen you are finished, submit by running the standalone command\n`echo COMPLETE_TASK_AND_SUBMIT_FINAL_OUTPUT` (with nothing else in that command).\nAfter it runs you cannot continue.\n";
const INSTRUCTION = "You are a deep research agent. Answer the question below by searching a fixed corpus of documents with the two shell commands described here. Reason step by step and use the commands as many times as you need.\n\nSearch commands (run them inside `bash`):\n- `bcp_search \"your query\"` returns the 10 best matching documents, each with its docid, a relevance score and a snippet.\n- `bcp_get_document DOCID` prints the document text.\n\nQuestion: {question}\n\nWhen you have your final answer, write it to the file ~/answer.txt (for example with cat > ~/answer.txt), in exactly this format, and then submit:\nExplanation: {your explanation, citing evidence documents by docid in square brackets, for example [20]}\nExact Answer: {your succinct, final answer}\nConfidence: {your confidence score between 0% and 100%}";
const LAZY = "\n\nThe search commands are also tools whose schemas are not loaded yet: call `load_tool` with `bcp_search` or `bcp_get_document` to use them directly.";
const TOOL_DOCS = {
  bcp_search: "Search the corpus. Returns the 10 best matching documents, each with its docid, a relevance score and a snippet.",
  bcp_get_document: "Print a document's text, given its docid.",
};
const STATE = "$HOME/.shell_state";

function isSubmit(command) {
  const c = (command || "").trim().replaceAll("\"", "").replaceAll("'", "").replaceAll("/bin/echo", "echo");
  return c == SUBMIT_CMD;
}

function shellQuote(x) { return "'" + String(x).replaceAll("'", "'\\''") + "'"; }

// A persistent shell over one-shot machine calls: the working directory and exported variables
// carry over in files under $HOME.
function persistent(command) {
  return `mkdir -p ${STATE}; cd "$(cat ${STATE}/cwd 2>/dev/null || echo $HOME)" 2>/dev/null; . ${STATE}/env 2>/dev/null\n${command}\n` +
    `_ec=$?; pwd > ${STATE}/cwd; export -p > ${STATE}/env; exit $_ec`;
}

// What a command printed, whole: layout shows its head and tail (agent.tss).
function observation(r) {
  const out = r.output.trimEnd();
  return (out || "(no output)") + "\n\n(exit_code=" + String(r.exit) + ")";
}

function Context({ template, policy, log, children }) {
  if (template == "qwen36") return <Qwen36App system={SYSTEM} enable_thinking={true}><TurnStrategy strategy="unrolled" log={log}>{children}</TurnStrategy></Qwen36App>;
  if (template == "glm53") return <GLM53App system={SYSTEM}><TurnStrategy strategy="unrolled" log={log}>{children}</TurnStrategy></GLM53App>;
  return <MessagesApp system={SYSTEM}><TurnStrategy strategy="unrolled" log={log.map(r => ({...r, ids: null}))}>{children}</TurnStrategy></MessagesApp>;
}

// A one-shot handoff summary, rendered for the same model family. The turns it quotes are laid
// out in the call's window (agent.tss `#handoff`).
function Handoff({ template, text }) {
  const body = <box id="handoff">{text}</box>;
  if (template == "qwen36") return <Qwen36App system={HANDOFF}><TurnStrategy strategy="unrolled" log={[]}>{body}</TurnStrategy></Qwen36App>;
  if (template == "glm53") return <GLM53App system={HANDOFF}><TurnStrategy strategy="unrolled" log={[]}>{body}</TurnStrategy></GLM53App>;
  return <MessagesApp system={HANDOFF}><TurnStrategy strategy="unrolled" log={[]}>{body}</TurnStrategy></MessagesApp>;
}

export default function ResearchAgent({ question, template = "messages", policy = "trunc", lazy = false, trig = 0.85, max_steps = 40, onDone }) {
  const [fold, setFold] = useState(FOLD, "fold");
  const [loaded, setLoaded] = useState([], "loaded");
  const [window, setWindow] = useState(null, "window");
  const log = useLog();
  const t = log.records;
  const task = INSTRUCTION.replaceAll("{question}", question) + (lazy ? LAZY : "");
  const run = async (command) => {
    const r = await harness.machine.call({cmd: persistent(command), timeout_s: 60});
    const content = observation(r);
    if (isSubmit(command) && r.exit == 0) {
      const a = await harness.machine.call({cmd: "cat ~/answer.txt 2>/dev/null"});
      if (onDone) onDone({policy, answer: a.output, turns: harness.turn});
      harness.stop({reason: "submit"});
    }
    return content + (window ? "\n[context: ~" + String(harness.prompt_tokens) + "/" + String(window) + " tokens]" : "");
  };
  const left = max_steps - harness.turn;
  const notice = left <= 3 && left >= 1 ? `[SYSTEM NOTICE] Only ${left} turns remain. Write ~/answer.txt now and submit.\n` : "";
  const app = <>
    {notice ? <box label="notice">{notice}</box> : ""}
    <aff tool_name="bash" description={BASH_DESC}
         args_schema={{type: "object", properties: {command: {type: "string", description: "The bash command to execute."}}, required: ["command"]}}
         onBash={async ({ command }) => await run(command)}>{"[bash]\n"}</aff>
    {lazy ? <aff tool_name="load_tool" description="Load a tool by name so it becomes available as a function."
         args_schema={{type: "object", properties: {name: {type: "string", enum: ["bcp_search", "bcp_get_document"]}}, required: ["name"]}}
         onLoad={({ name }) => { setLoaded(l => l.includes(name) ? l : [...l, name]); return `Loaded \`${name}\`: ${TOOL_DOCS[name]}`; }}>{"[load_tool]\n"}</aff> : ""}
    {lazy && loaded.includes("bcp_search") ? <aff tool_name="bcp_search" description={TOOL_DOCS.bcp_search}
         args_schema={{type: "object", properties: {query: {type: "string"}}, required: ["query"]}}
         onSearch={async ({ query }) => await run("bcp_search " + shellQuote(query))}>{"[bcp_search]\n"}</aff> : ""}
    {lazy && loaded.includes("bcp_get_document") ? <aff tool_name="bcp_get_document" description={TOOL_DOCS.bcp_get_document}
         args_schema={{type: "object", properties: {docid: {type: "string"}}, required: ["docid"]}}
         onGet={async ({ docid }) => await run("bcp_get_document " + shellQuote(docid))}>{"[bcp_get_document]\n"}</aff> : ""}
    {policy == "fold" ? <aff tool_name="branch" description="Start a side branch for a self-contained sub task (one search thread, one document); its steps leave your context when you return."
         args_schema={{type: "object", properties: {description: {type: "string"}}, required: ["description"]}}
         onBranch={({ description }) => branches(t).open != null ? "error: already in a branch; return first" : openBranch(description)}>{"[branch]\n"}</aff> : ""}
    {policy == "fold" ? <aff tool_name="return" description="Finish the current branch and return what it found or did."
         args_schema={{type: "object", properties: {summary: {type: "string"}}, required: ["summary"]}}
         onReturn={({ summary }) => { if (branches(t).open == null) return "error: not in a branch"; if (fold.notice) setFold({...fold, notice: ""}); return returnBranch(summary); }}>{"[return]\n"}</aff> : ""}
    {policy == "fold" && branches(t).open != null ? <box label="branch">{`[IN BRANCH: ${branches(t).open.desc}]${fold.notice ? "\n" + fold.notice : ""}\n`}</box> : ""}
    {policy == "tree" ? <aff tool_name="recall" description="Read an earlier turn of this session in full, by its turn number."
         args_schema={{type: "object", properties: {turn: {type: "integer"}}, required: ["turn"]}}
         onRecall={({ turn }) => recall(t, turn)}>{"[recall]\n"}</aff> : ""}
  </>;
  // The context policies that are not layout's react to it: after a prompt is laid out past
  // `trig` of the window, "compact" and "tree" summarize, and "fold" in a branch is told to return.
  const [folding, setFolding] = useState(false, "folding");
  useEvent("layout", async ({ prompt_tokens, window: w }) => {
    if (w !== window) setWindow(w);
    if (w == null || folding || prompt_tokens <= trig * w || turnStarts(t).length - fold.start < 2) return;
    if (policy == "fold") { if (branches(t).open != null && !fold.notice) setFold({...fold, notice: BRANCH_OVERFLOW}); return; }
    if (policy != "compact" && policy != "tree") return;
    const summarize = async (text) => (await model(<Handoff template={template} text={text}/>)).text;
    setFolding(true);
    try { setFold(policy == "compact" ? await compact(t, fold, summarize) : await summaryTree(t, fold, summarize)); }
    finally { setFolding(false); }
  });
  useLoop(log);
  return <Context template={template} policy={policy} log={project(t, fold, [{kind: "input", content: task}])}>{app}</Context>;
}

export { ResearchAgent };
