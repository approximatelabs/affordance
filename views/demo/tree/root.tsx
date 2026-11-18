// The instance tree's hello world (tests/runtime.rs). The root has no model: it owns the
// greetings, mounts one greeter per name with one of its machines, and hears them through a
// callback. Props flow down; callbacks flow up; nothing else is shared. `onStop` is called by the
// harness when a child stops, whatever stopped it.
import { Greeter } from "./greeter.tsx";

export default function Root({ names }) {
  const [greetings, setGreetings] = useState([], "greetings");
  const [stopped, setStopped] = useState([], "stopped");
  return <>
    {names.map((name, i) =>
      <Mount key={name} model="default" machine={harness.machines[i]}>
        <Greeter name={name} heard={greetings}
                 onGreet={g => { setGreetings([...greetings, g]); }}
                 onStop={how => { setStopped([...stopped, `${name}: ${how}`]); }}/>
      </Mount>)}
  </>;
}
