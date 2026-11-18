import { TurnStrategy } from "../../views/strategies/turn_strategy.tsx";
import { Qwen36App } from "../../views/template_apps/qwen36.tsx";
export default function Main({ strategy, log, text = "Live screen." }) {
  // The references (tools/middle_references.py) are made with the template's `preserve_thinking` on.
  return <Qwen36App system="Strategy test." preserve_thinking={true}><TurnStrategy strategy={strategy} log={log}>
    <aff label="live" description="Choose the live action."
         args_schema={{type:"object", properties:{text:{type:"string"}}, required:["text"]}}
         onChange={({text}) => ({selected:text})}>{text}</aff>
  </TurnStrategy></Qwen36App>;
}
