// The template apps of the deployment's model bindings, by template name: a hand-written one
// (`qwen3`, `messages`, ..: template_apps/index.json), or, for a registered model's name, the one
// template app for any model given that model's format. The runtime makes this module from
// the bindings, so an app loads only the formats it is bound to (a format's file is written
// for its own model's tokenizer and may not load under another's).
import { templates } from "affordance:model";
import { TurnStrategy } from "../strategies/turn_strategy.tsx";

export const templateApps = templates;

// Template selection belongs to the binding. History selection still belongs to the app.
export function ModelContext({ children, system, log = [], strategy = "last-turn", callable = true }) {
  const binding = harness.model;
  if (!binding || !binding.template) throw new Error("ModelContext needs to know the model's format: set `template` on the model in the deployment, or pass --template NAME to affordance render");
  const Template = templates[binding.template];
  if (!Template) throw new Error(`ModelContext: the template ${binding.template} was not loaded with this app: an app is loaded with the templates its deployment's model bindings name (View::load_model, or affordance render --template, for one model outside a run)`);
  // Chat transport replies carry JSON token IDs, not native assistant-turn IDs.
  const records = binding.template === "messages"
    ? memoScan(log, null, (s, r) => [null, {...r, ids: null}], null).outs : log;
  return <Template {...binding.template_options} system={system}>
    <TurnStrategy log={records} strategy={strategy} callable={callable}>{children}</TurnStrategy>
  </Template>;
}
