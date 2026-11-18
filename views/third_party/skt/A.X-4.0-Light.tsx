// Converted from the chat template of skt/A.X-4.0-Light by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let tools_0 = ctx["tools"] ?? null;
  let messages_0 = ctx["messages"];
  let content_0 = ctx["content"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", tools_0), 0)) : v)(J.test("iterable", tools_0)))) {
    out.push(J.str("<|im_start|><|system|>"), J.str("\ub2f9\uc2e0\uc740 \ub3c4\uad6c \ud638\ucd9c \uae30\ub2a5\uc744 \uac16\ucd98 \uc720\uc6a9\ud55c \ub3c4\uc6b0\ubbf8\uc785\ub2c8\ub2e4. \uc0ac\uc6a9\uc790\uc758 \uc694\uccad\uc744 \ucc98\ub9ac\ud558\uae30 \uc704\ud574\uc11c \ud544\uc694\ud55c \ub3c4\uad6c\uac00 \uc8fc\uc5b4\uc9c4 \ubaa9\ub85d\uc5d0 \uc788\ub294 \uacbd\uc6b0 \ub3c4\uad6c \ud638\ucd9c\ub85c \uc751\ub2f5\ud558\uc138\uc694.\n\ud544\uc694\ud55c \ub3c4\uad6c\uac00 \ubaa9\ub85d\uc5d0 \uc5c6\ub294 \uacbd\uc6b0\uc5d0\ub294 \ub3c4\uad6c \ud638\ucd9c \uc5c6\uc774 \uc0ac\uc6a9\uc790\uac00 \uc694\uad6c\ud55c \uc815\ubcf4\ub97c \uc81c\uacf5\ud558\uc138\uc694.\n\ud544\uc694\ud55c \ub3c4\uad6c\uac00 \ubaa9\ub85d\uc5d0 \uc788\uc9c0\ub9cc \ud574\ub2f9 \ub3c4\uad6c\ub97c \ud638\ucd9c\ud558\ub294\ub370 \ud544\uc694\ud55c argument \uc815\ubcf4\uac00 \ubd80\uc871\ud55c \uacbd\uc6b0 \ud574\ub2f9 \uc815\ubcf4\ub97c \uc0ac\uc6a9\uc790\uc5d0\uac8c \uc694\uccad\ud558\uc138\uc694.\n\uc0ac\uc6a9\uc790\uc758 \uc694\uccad\uc744 \ucc98\ub9ac\ud558\uae30 \uc704\ud574 \uc5ec\ub7ec\ubc88 \ub3c4\uad6c\ub97c \ud638\ucd9c\ud560 \uc218 \uc788\uc5b4\uc57c \ud569\ub2c8\ub2e4.\n\ub3c4\uad6c \ud638\ucd9c \uc774\ud6c4 \ub3c4\uad6c \uc2e4\ud589 \uacb0\uacfc\ub97c \uc785\ub825\uc73c\ub85c \ubc1b\uc73c\uba74 \ud574\ub2f9 \uacb0\uacfc\ub97c \ud65c\uc6a9\ud558\uc5ec \ub2f5\ubcc0\uc744 \uc0dd\uc131\ud558\uc138\uc694.\n\n\ub2e4\uc74c\uc740 \uc811\uadfc\ud560 \uc218 \uc788\ub294 \ub3c4\uad6c\ub4e4\uc758 \ubaa9\ub85d \uc785\ub2c8\ub2e4:\n<tools>\n"));
    {
      let _items1 = J.iter(tools_0);
      const _loop1 = J.loop(_items1);
      for (let _i1 = 0; _i1 < _items1.length; _i1++) {
        _loop1.index0 = _i1; const _it1 = _items1[_i1]; let t_1 = _it1;
        out.push(J.str(J.filter("tojson", t_1)), J.str("\n"));
      }
    }
    out.push(J.str("</tools>"), J.str("\n\n\ub3c4\uad6c\ub97c \ud638\ucd9c\ud558\ub824\uba74 \uc544\ub798\uc758 JSON\uc73c\ub85c \uc751\ub2f5\ud558\uc138\uc694.\n\ub3c4\uad6c \ud638\ucd9c \ud615\uc2dd: <tool_call>{\"name\": \ub3c4\uad6c \uc774\ub984, \"arguments\": dictionary \ud615\ud0dc\uc758 \ub3c4\uad6c \uc778\uc790\uac12}</tool_call>"), J.str("<|im_end|>"));
  }
  {
    let _items2 = J.iter(messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
      let content_3 = content_0;
      if (J.t((J.eq(J.attr(message_2, "role"), "system")))) {
        out.push(J.str(J.add(J.add("<|im_start|><|system|>", J.attr(message_2, "content")), "<|im_end|>")));
      } else if (J.t((J.eq(J.attr(message_2, "role"), "user")))) {
        out.push(J.str(J.add(J.add("<|im_start|><|user|>", J.attr(message_2, "content")), "<|im_end|>")));
      } else if (J.t((J.eq(J.attr(message_2, "role"), "assistant")))) {
        out.push(J.str("<|im_start|><|assistant|>"));
        content_3 = "";
        if (J.t((J.attr(message_2, "content") !== undefined))) {
          content_3 = J.attr(message_2, "content");
        }
        if (J.t(((v) => J.t(v) ? !J.t(((v) => J.t(v) ? !J.t((J.attr(message_2, "reasoning_content") === null)) : v)((J.attr(message_2, "reasoning_content") !== undefined))) : v)(add_generation_prompt_0))) {
          if (J.t((J.has("</think>", J.attr(message_2, "content"))))) {
            content_3 = J.call(J.attr(J.item(J.call(J.attr(J.attr(message_2, "content"), "split"), J.call(J.attr("</think>", "strip"))), J.neg(1)), "lstrip"), "\n");
          }
        }
        out.push(J.str(content_3));
        if (J.t((J.attr(message_2, "tool_calls") !== undefined))) {
          {
            let _items3 = J.iter(J.attr(message_2, "tool_calls"));
            const _loop3 = J.loop(_items3);
            for (let _i3 = 0; _i3 < _items3.length; _i3++) {
              _loop3.index0 = _i3; const _it3 = _items3[_i3]; let tool_call_4 = _it3;
              if (J.t((J.attr(tool_call_4, "function") !== undefined))) {
                tool_call_4 = J.attr(tool_call_4, "function");
              }
              out.push(J.str("<tool_call>"), J.str("{"), J.str("\"name\": \""), J.str(J.attr(tool_call_4, "name")), J.str("\""));
              if (J.t((J.attr(tool_call_4, "arguments") !== undefined))) {
                out.push(J.str(", "), J.str("\"arguments\": "), J.str(J.filter("tojson", J.attr(tool_call_4, "arguments"))));
              }
              out.push(J.str("}"), J.str("</tool_call>"));
            }
          }
        }
        out.push(J.str("<|im_end|>"));
      } else if (J.t((J.eq(J.attr(message_2, "role"), "tool")))) {
        out.push(J.str(J.add(J.add("<|im_start|><|extra_id_13|><tool_output>", J.attr(message_2, "content")), "</tool_output><|im_end|>")));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push(J.str("<|im_start|><|assistant|>"));
  }
  return out.join("");
}
