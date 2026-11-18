import { TurnStrategy } from "../../views/strategies/turn_strategy.tsx";
import { Llama31App } from "../../views/template_apps/llama31.tsx";
export default function Main({ strategy, log, text = "Live screen." }) {
  return <Llama31App system="Strategy test."><TurnStrategy strategy={strategy} log={log}>
    <aff label="live" description="Choose the live action."
         args_schema={{type:"object", properties:{text:{type:"string"}}, required:["text"]}}
         onChange={({text}) => ({selected:text})}>{text}</aff>
  </TurnStrategy></Llama31App>;
}
