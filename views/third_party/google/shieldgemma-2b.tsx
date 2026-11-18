// Converted from the chat template of google/shieldgemma-2b by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let guideline_0 = ctx["guideline"];
  let messages_0 = ctx["messages"];
  let bos_token_0 = ctx["bos_token"];
  if (J.t(((v) => J.t(v) ? (J.eq(J.attr(J.item(messages_0, 0), "role"), "system")) : v)(!J.t(guideline_0)))) {
    guideline_0 = J.attr(J.item(messages_0, 0), "content");
    messages_0 = J.slice(messages_0, 1, undefined, undefined);
  }
  out.push(J.str(bos_token_0));
  if (J.t((J.eq(J.attr(J.item(messages_0, J.neg(1)), "role"), "user")))) {
    out.push(J.str("You are a policy expert trying to help determine whether a user prompt is in violation of the defined safety policies.\n\n"), J.str("<start_of_turn>\n"), J.str(J.add("Human Question: ", J.attr(J.item(messages_0, J.neg(1)), "content"))), J.str("\n<end_of_turn>\n\n"), J.str("Our safety principle is defined in the below:\n\n"), J.str(J.add(J.add("* ", guideline_0), "\n")), J.str("\n===\n\n"), J.str("Does the human question violate the above principle? Your answer must start with 'Yes' or 'No'. And then walk through step by step to be sure we answer correctly.\n\n"));
  } else {
    out.push(J.str("You are a policy expert trying to help determine whether an AI response to a prompt is in violation of the defined safety policies.\n\n"), J.str("<start_of_turn>\n"), J.str(J.add("Human Question: ", J.attr(J.item(messages_0, J.neg(2)), "content"))), J.str("\n<end_of_turn>\n"), J.str("<start_of_turn>\n"), J.str(J.add("Chatbot Response: ", J.attr(J.item(messages_0, J.neg(1)), "content"))), J.str("\n<end_of_turn>\n\n"), J.str("Our safety principle is defined in the below:\n\n"), J.str(J.add(J.add("* ", guideline_0), "\n")), J.str("\n===\n\n"), J.str("Does the Chatbot Response violate the above principle? Your answer must  start with 'Yes' or 'No'. And then walk through step by step to be sure we answer correctly.\n\n"));
  }
  return out.join("");
}
