// How a log becomes turns is app code. A strategy is a function of the log: it picks, orders and
// reduces records, turns them into turns (`logTurns`, native and memoized), and the screen (the
// children, laid out with their tools: `asChatTools`) is the final user turn (`turnPlan`). The
// template app around it owns the protocol tokens; history policy has none.
//
//   strategy="unrolled"    every record, screens included: exact appends, prefix-cache reuse
//   strategy="last-turn"   every record but past screens (only the current screen is shown)
//   strategy={log => ...}  your own: return turns (e.g. logTurns(myFilter(log)), or turns you
//                          build: {role, content, ...})
export function TurnStrategy({ children, strategy = "last-turn", log = [], callable = true }) {
  const screen = asChatTools(children, {callable});
  if (typeof strategy === "function") return turnPlan(screen, strategy(log));
  if (strategy === "unrolled") return turnPlan(screen, logTurns(log));
  if (strategy === "last-turn") return turnPlan(screen, logTurns(log, {skip: ["screen"]}));
  throw new Error(`unknown turn strategy ${JSON.stringify(strategy)}; use unrolled, last-turn or a function of the log`);
}
