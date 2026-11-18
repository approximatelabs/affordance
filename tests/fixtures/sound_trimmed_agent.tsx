// tests/fixtures/sound_agent.tsx under a sheet that trims its clip to 12 tokens.
import Inner from "./sound_agent.tsx";
import "./sound_trimmed_agent.tss";
export default function Main(props) {
  return Inner(props);
}
