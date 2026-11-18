// A first app: state becomes token IDs and a visible action can change that state.
export default function Hello({name = "world"}) {
  return <box>
    <span label="greeting">{`Hello, ${name}!`}</span>
    <aff name="greet" onGreet={({name}) => ({name})}>{" [greet]"}</aff>
  </box>;
}
