// A support desk: customer messages arrive one at a time, and the agent sends each to the team that
// handles it. Each team is a tool. The app names no model: with a chat model's template the tools
// are function definitions it calls in JSON; with d1's (D1App) they are options it picks by code.
import { useLog } from "../../lib/log.tsx";
import { useLoop } from "../../lib/loop.tsx";
import { Tool } from "../../lib/tool.tsx";
import { ModelContext } from "../../lib/model.tsx";

const QUESTION = "Which team should handle this customer message? Send it there by calling that team's tool.";

export default function Desk({ tickets }) {
  const log = useLog();
  useLoop(log);
  const [i, setI] = useState(0, "i");
  const [routed, setRouted] = useState([], "routed");
  function send(team) {
    const next = [...routed, team];
    setRouted(next);
    if (next.length >= tickets.length) harness.stop({ routed: next });
    setI(i + 1);
    return `Sent to ${team}. Next message.`;
  }
  /** Orders: placing, changing, cancelling or tracking an order */
  function orders({}: {}) { return send("orders"); }
  /** Shipping: setting up or changing a shipping address */
  function shipping({}: {}) { return send("shipping"); }
  /** Delivery: delivery options, times and costs */
  function delivery({}: {}) { return send("delivery"); }
  /** Invoices: finding or checking an invoice */
  function invoices({}: {}) { return send("invoices"); }
  /** Payments: payment methods and problems paying */
  function payments({}: {}) { return send("payments"); }
  /** Refunds: asking for a refund or checking on one */
  function refunds({}: {}) { return send("refunds"); }
  /** Accounts: signing up, editing, switching or deleting an account */
  function accounts({}: {}) { return send("accounts"); }
  /** Feedback: complaints and reviews */
  function feedback({}: {}) { return send("feedback"); }
  /** Contact: the customer wants to reach customer service or a person */
  function contact({}: {}) { return send("contact"); }
  return <ModelContext strategy="last-turn" system={QUESTION} log={log.records}>
    {`Customer message ${i + 1} of ${tickets.length}:\n${tickets[Math.min(i, tickets.length - 1)]}`}
    {[orders, shipping, delivery, invoices, payments, refunds, accounts, feedback, contact].map(fn => <Tool fn={fn}/>)}
  </ModelContext>;
}
