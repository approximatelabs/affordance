"use strict_tokens";
import { DeepSeekV41, isImageBlock, imageSrc } from "../deepseek/v41.tsx";
import { prepareTemplateInput, publisherTurns } from "./input.tsx";
// DeepSeek V4.1 (Flash): DSML tool calls, ending at the end-of-sentence token. In thinking mode
// (the default) the prompt ends inside <think>. `reasoning_effort`: 1..100 or low/medium/high/max.
// `drop_thinking`: the template's own switch, on unless set (a past reply's thinking is dropped).
// `keep_replies`: every reply stays as its own tokens.
// DeepSeek V4.1's picture: lines of 42-pixel cells, as its processor lays it out (SGLang's own
// sizing code: a start, each row of cells and a line end, and an end, all the placeholder).
const PICTURE = {kind: "lines", patch: 14, downsample: 3, max_tokens: 1024, min_pixels: 295936};
// A picture with a source, laid out at the tokens the model reads; the element also carries the
// image to the model request. The view takes it as the app's own text.
const laid = b => isPlainObject(b) && isImageBlock(b) && imageSrc(b)
  ? {type: "text", text: <><img src={imageSrc(b)} t={tok.deepseek_image.text} metrics={PICTURE}/></>}
  : isPlainObject(b) && Array.isArray(b.content) ? {...b, content: b.content.map(laid)} : b;
const withPictures = messages => messages.map(m => Array.isArray(m.content) ? {...m, content: m.content.map(laid)} : m);

export function DeepSeekV41App({ children, system = "Use the available tools to interact with the app.", enable_thinking = true, reasoning_effort, drop_thinking, keep_replies = false }) {
  const input = prepareTemplateInput(children, system, true);
  const format = (messages, generate) => <DeepSeekV41 messages={withPictures(messages)} tools={input.tools} add_generation_prompt={generate}
                       thinking_mode={enable_thinking === false ? "chat" : "thinking"} reasoning_effort={reasoning_effort} drop_thinking={drop_thinking}/>;
  const turns = publisherTurns(input.turns, keep_replies, (messages, generate) => textContent(format(messages, generate)));
  return [format(turns, true),
          <span label="protocol" syntax="dsml" stop={[tok.end_of_sentence.id]} in_think={enable_thinking !== false}/>];
}
