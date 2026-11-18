// Capability-injected tools: the app selects its machine and owns truncation/output policy.
//   const {shell} = machineTools(harness.machine.get("home"));
//   <Tool fn={shell}/>
// `signal`: a function returning the AbortSignal a command stops on (the current turn's), if any.
export function machineTools(machine, options) {
  const signal = options && options.signal;
  /** Run a shell command on the bound machine. */
  async function shell({command, timeout_s = 60}: {command: string; timeout_s?: number}) {
    const {output, ...meta} = await machine.call({cmd: command, timeout_s}, {signal: signal ? signal() : undefined});
    return {result: output, ...meta};
  }
  return {shell};
}

/** Wait for a duration, or until one of the named events arrives. */
export async function wait({seconds, events = []}: {seconds?: number; events?: string[]}) {
  if (seconds !== undefined && (!Number.isFinite(seconds) || seconds < 0)) throw new RangeError("seconds must be finite and nonnegative");
  if (seconds === undefined && !events.length) throw new Error("wait requires seconds or events");
  return await harness.wait(seconds === undefined ? {events} : {ms: seconds * 1000, events});
}
