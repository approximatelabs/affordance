import { TurnStrategy } from "../../views/strategies/turn_strategy.tsx";
import { Qwen3BaseApp } from "../../views/template_apps/qwen3_base.tsx";
export default function Main({ strategy, log, text = "Live screen." }) {
  return <Qwen3BaseApp system="Strategy test."><TurnStrategy strategy={strategy} log={log}>
    <aff label="live" description="Choose the live action."
         args_schema={{type:"object", properties:{text:{type:"string"}}, required:["text"]}}
         onChange={({text}) => ({selected:text})}>{text}</aff>
  </TurnStrategy></Qwen3BaseApp>;
}
