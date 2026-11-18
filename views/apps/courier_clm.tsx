// A small parcel-delivery example rendered in Contrastive-CLM's format: the
// public board with the courier, parcel and depot drawn in, the question, and each legal move as
// plain text. The state fields are the dataset's public fields; labels never enter the view.
"use strict_tokens";
import { ClmChoice } from "../decision/clm.tsx";

const QUESTION = "Which move should the courier make next? A is the courier, P the parcel, D the depot and # a wall. The courier picks up the parcel by stepping onto it, then delivers it by stepping onto the depot.";
const MOVES = [["north", 0, -1, "Move north (up)"], ["east", 1, 0, "Move east (right)"],
               ["south", 0, 1, "Move south (down)"], ["west", -1, 0, "Move west (left)"]];
const put = (row, x, c) => row.slice(0, x) + c + row.slice(x + 1);

// The readout prompt: a model binding that scores options (contrastive) reads the state span and
// one span per legal move from this render.
export function Main({ agent, board, carrying, depot, parcel }) {
  let rows = board.map((row, y) => y === depot.y ? put(row, depot.x, "D") : row);
  if (!carrying) rows = rows.map((row, y) => y === parcel.y ? put(row, parcel.x, "P") : row);
  rows = rows.map((row, y) => y === agent.y ? put(row, agent.x, "A") : row);
  const legal = MOVES.filter(m => board[agent.y + m[2]][agent.x + m[1]] !== "#");
  return <ClmChoice state={{map: rows, "carrying the parcel": carrying ? "yes" : "no"}} instructions={QUESTION}
                    options={legal.map(m => ({name: m[0], desc: m[3]}))}/>;
}
