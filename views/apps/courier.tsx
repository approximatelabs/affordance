import { ModelContext } from "../lib/model.tsx";
"use strict_tokens";

// The same state and transitions support several views. No expert labels enter here.
const DIRECTIONS = [
  { name: "north", dx: 0, dy: -1 },
  { name: "east", dx: 1, dy: 0 },
  { name: "south", dx: 0, dy: 1 },
  { name: "west", dx: -1, dy: 0 }
];

export function Courier({ board, agent, parcel, depot, carrying = false, layout = "map", history = [], history_mode = "none", history_n = 4 }) {
  const delivered = carrying && agent.x === depot.x && agent.y === depot.y;
  const moves = DIRECTIONS.filter(d => {
    const x = agent.x + d.dx;
    const y = agent.y + d.dy;
    return !delivered && y >= 0 && y < board.length && x >= 0 && x < board[0].length && board[y][x] !== "#";
  });
  const cell = (x, y) => agent.x === x && agent.y === y ? "@" :
    !carrying && parcel.x === x && parcel.y === y ? "P" :
    depot.x === x && depot.y === y ? "D" : board[y][x];
  const map = board.map((row, y) => p`${Array.from({length: row.length}).map((_, x) => cell(x, y)).join("")}\n`);
  const pickups = history.filter(e => !e.carrying_before && e.carrying_after);
  const pinned = history.length === 0 ? [] : pickups.length === 0 ? [history[0]] :
    history[0] === pickups.at(-1) ? [history[0]] : [history[0], pickups.at(-1)];
  const shown = history_mode === "all" ? history : history_mode === "pinned" ? pinned : history.slice(-history_n);
  return <>
    {p`COURIER\nCollect the parcel, then bring it to the depot. Walls cannot be crossed.\n`}
    {p`@ courier | P parcel | D depot | # wall | . floor\n`}
    {layout === "map" ? <>
      {map}
      {p`Inventory: ${carrying ? "parcel" : "empty"}\n`}
    </> : layout === "coordinates" ? <>
      {p`Position: (${agent.x}, ${agent.y})\nParcel: ${carrying ? "carried" : p`(${parcel.x}, ${parcel.y})`}\nDepot: (${depot.x}, ${depot.y})\n`}
      {p`Board rows, north to south; columns west to east:\n`}{map}
    </> : <>
      {p`Inventory: ${carrying ? "parcel" : "empty"}; courier (${agent.x},${agent.y}); depot (${depot.x},${depot.y})\n`}
      {p`Parcel origin (${parcel.x},${parcel.y}).\n`}
      {board.map((row, y) => p`row ${y}: ${Array.from({length: row.length}).map((_, x) => cell(x, y)).join(" ")}\n`)}
    </>}
    {history_mode !== "none" && <>
      {p`Completed moves (${history_mode === "last_n" ? p`last ${history_n}` : history_mode}; * means carrying):\n`}
      {shown.length === 0 ? p`No completed moves.\n` : shown.map(e => p`${e.from.x},${e.from.y} ${e.action} ${e.to.x},${e.to.y}${e.carrying_after ? "*" : ""}; `)}
      {p`\n`}
    </>}
    {delivered ? p`Delivery complete.\n` : <>
      {p`Available moves:\n`}
      {moves.map(d => <aff label={d.name} description={"Move one cell " + d.name + "."}
        args_schema={{ type: "object", properties: {}, additionalProperties: false }}
        onMove={() => {
          const next_agent = { x: agent.x + d.dx, y: agent.y + d.dy };
          const next_carrying = carrying || (next_agent.x === parcel.x && next_agent.y === parcel.y);
          return history_mode === "none" ? {agent: next_agent, carrying: next_carrying} :
            {agent: next_agent, carrying: next_carrying, history: [...history, {
              from: agent, to: next_agent, action: d.name,
              carrying_before: carrying, carrying_after: next_carrying
            }]};
        }}>{p`${d.name}: (${agent.x + d.dx}, ${agent.y + d.dy})\n`}</aff>)}
    </>}
  </>;
}

export default function CourierApp({ board, agent, parcel, depot, carrying = false, layout = "map", history = [], history_mode = "none", history_n = 4 }) {
  return <Courier board={board} agent={agent} parcel={parcel} depot={depot} carrying={carrying} layout={layout} history={history} history_mode={history_mode} history_n={history_n}/>;
}

// The courier as a chat model's tools, for any model: the binding names the format.
export function CourierChat(props) {
  return <ModelContext system="Choose one available move that completes the delivery efficiently."><Courier {...props}/></ModelContext>;
}
