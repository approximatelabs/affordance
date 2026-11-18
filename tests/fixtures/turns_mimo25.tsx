import { TurnStrategy } from "../../views/strategies/turn_strategy.tsx";
import { MiMo25App } from "../../views/template_apps/mimo25.tsx";
export default function Main({ strategy, log, text = "Live screen." }) {
  return <MiMo25App system="Strategy test."><TurnStrategy strategy={strategy} log={log}>
    <aff label="live" description="Choose the live action."
         args_schema={{type:"object", properties:{text:{type:"string"}}, required:["text"]}}
         onChange={({text}) => ({selected:text})}>{text}</aff>
  </TurnStrategy></MiMo25App>;
}
