// Cloudflare Clef / Clef-flash (published decision models with a joint schema head): one choice
// field over a state and images, rendered exactly as the checkpoint's encode_record renders it.
// encode_record tokenizes each piece with its own tokenizer call, so each piece here is a <box>
// (a hard tokenization boundary in every render; an empty <span> is one only in the diagnostic
// piecewise render). The head reads the question span and each option's span (<aff option> over
// the option's JSON), so <Decision readout="schema_head"> sends both. Clef scores choice options
// sorted by id, and so does this view. Images: one <|image_pad|> each, expanded by the host
// (SGLang's joint schema route counts positions before expansion).
"use strict_tokens";

const SYSTEM_PROMPT = "Read the complete state and schema. Decide every field jointly. Each answer must be exactly one of that field's allowed options.";
const dumps = value => typeof value === "string" ? value : formatJSON(value, {separators: [",", ":"], sortKeys: true});
const Piece = ({ children }) => <box>{children}</box>;

// options: [{name, desc}]. state: any JSON value or a string. field: the question id.
export function ClefChoice({ state, images = [], field, instructions, options }) {
  return <>
    <Piece>{tok.im_start}{"system\n" + SYSTEM_PROMPT}{tok.im_end}{"\n"}{tok.im_start}{"user\nSTATE:\n"}</Piece>
    {images.length > 0 ? <Piece>{images.map(src => <>{tok.vision_start}<img src={src} tok_size={1} t="<|image_pad|>"/>{tok.vision_end}</>)}{"\n"}</Piece> : ""}
    <Piece>{dumps(state)}</Piece>
    <Piece>{"\n\nSCHEMA FIELDS:\n"}</Piece>
    <Piece>{"\nFIELD 1\nID: " + field + "\nTYPE: choice\nINSTRUCTION: "}</Piece>
    <Piece><span label="question">{dumps(instructions)}</span></Piece>
    <Piece>{"\nALLOWED OPTIONS:\n"}</Piece>
    {[...options].sort((x, y) => x.name < y.name ? -1 : x.name > y.name ? 1 : 0).map((o, i) => <>
      <Piece>{"OPTION " + String(i + 1) + ": "}</Piece>
      <Piece><aff option={o.name}>{dumps(o.desc === null ? {option_id: o.name} : {option_id: o.name, description: o.desc})}</aff></Piece>
      <Piece>{"\n"}</Piece>
    </>)}
    <Piece>{"END FIELD\n"}</Piece>
    <Piece>{"\n"}{tok.im_end}{"\n"}{tok.im_start}{"assistant\n"}{tok.think}{"\n\n"}{tok.think_end}{"\n\nJOINT SCHEMA DECISIONS:"}</Piece>
  </>;
}
