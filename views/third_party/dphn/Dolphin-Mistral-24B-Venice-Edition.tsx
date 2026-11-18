// Converted from the chat template of dphn/Dolphin-Mistral-24B-Venice-Edition by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let yesterday_day_0 = ctx["yesterday_day"];
  let yesterday_month_0 = ctx["yesterday_month"];
  let yesterday_year_0 = ctx["yesterday_year"];
  let today_date_0 = ctx["today_date"];
  let yesterday_date_0 = ctx["yesterday_date"];
  let default_system_message_0 = ctx["default_system_message"];
  let system_message_0 = ctx["system_message"];
  let loop_messages_0 = ctx["loop_messages"];
  let tools_description_0 = ctx["tools_description"];
  let has_tools_0 = ctx["has_tools"];
  let ns_0 = ctx["ns"];
  let bos_token_0 = ctx["bos_token"];
  let messages_0 = ctx["messages"];
  let tools_0 = ctx["tools"] ?? null;
  let content_0 = ctx["content"];
  let tool_call_id_0 = ctx["tool_call_id"];
  let eos_token_0 = ctx["eos_token"];
  let arguments_0 = ctx["arguments"];
  yesterday_day_0 = J.call(J.strftime_now, "%d");
  yesterday_month_0 = J.call(J.strftime_now, "%m");
  yesterday_year_0 = J.call(J.strftime_now, "%Y");
  today_date_0 = J.add(J.add(J.add(J.add(yesterday_year_0, "-"), yesterday_month_0), "-"), yesterday_day_0);
  if (J.t((J.eq(yesterday_day_0, "01")))) {
    if (J.t((J.eq(yesterday_month_0, "01")))) {
      yesterday_day_0 = "31";
      yesterday_month_0 = "12";
      if (J.t((J.eq(yesterday_year_0, "2024")))) {
        yesterday_year_0 = "2023";
      } else if (J.t((J.eq(yesterday_year_0, "2025")))) {
        yesterday_year_0 = "2024";
      } else if (J.t((J.eq(yesterday_year_0, "2026")))) {
        yesterday_year_0 = "2025";
      } else if (J.t((J.eq(yesterday_year_0, "2027")))) {
        yesterday_year_0 = "2026";
      } else if (J.t((J.eq(yesterday_year_0, "2028")))) {
        yesterday_year_0 = "2027";
      } else if (J.t((J.eq(yesterday_year_0, "2029")))) {
        yesterday_year_0 = "2028";
      } else if (J.t((J.eq(yesterday_year_0, "2030")))) {
        yesterday_year_0 = "2029";
      } else if (J.t((J.eq(yesterday_year_0, "2031")))) {
        yesterday_year_0 = "2030";
      } else if (J.t((J.eq(yesterday_year_0, "2032")))) {
        yesterday_year_0 = "2031";
      } else if (J.t((J.eq(yesterday_year_0, "1970")))) {
        yesterday_year_0 = "1970";
      } else {
        out.push(J.str(J.call(J.raise_exception, J.add(J.add("Unsloth custom template does not support years > 2032. Error year = [", yesterday_year_0), "]"))));
      }
    } else if (J.t((J.eq(yesterday_month_0, "02")))) {
      yesterday_day_0 = "31";
      yesterday_month_0 = "01";
    } else if (J.t((J.eq(yesterday_month_0, "03")))) {
      yesterday_month_0 = "02";
      yesterday_day_0 = "28";
      if (J.t((J.eq(yesterday_year_0, "2024")))) {
        yesterday_day_0 = "29";
      } else if (J.t((J.eq(yesterday_year_0, "2028")))) {
        yesterday_day_0 = "29";
      } else if (J.t((J.eq(yesterday_year_0, "2032")))) {
        yesterday_day_0 = "29";
      } else if (J.t((J.eq(yesterday_year_0, "1970")))) {
        yesterday_day_0 = "29";
      } else {
        out.push(J.str(J.call(J.raise_exception, J.add(J.add("Unsloth custom template does not support years > 2032. Error year = [", yesterday_year_0), "]"))));
      }
    } else if (J.t((J.eq(yesterday_month_0, "04")))) {
      yesterday_day_0 = "31";
      yesterday_month_0 = "03";
    } else if (J.t((J.eq(yesterday_month_0, "05")))) {
      yesterday_day_0 = "30";
      yesterday_month_0 = "04";
    } else if (J.t((J.eq(yesterday_month_0, "06")))) {
      yesterday_day_0 = "31";
      yesterday_month_0 = "05";
    } else if (J.t((J.eq(yesterday_month_0, "07")))) {
      yesterday_day_0 = "30";
      yesterday_month_0 = "06";
    } else if (J.t((J.eq(yesterday_month_0, "08")))) {
      yesterday_day_0 = "31";
      yesterday_month_0 = "07";
    } else if (J.t((J.eq(yesterday_month_0, "09")))) {
      yesterday_day_0 = "31";
      yesterday_month_0 = "08";
    } else if (J.t((J.eq(yesterday_month_0, "10")))) {
      yesterday_day_0 = "30";
      yesterday_month_0 = "09";
    } else if (J.t((J.eq(yesterday_month_0, "11")))) {
      yesterday_day_0 = "31";
      yesterday_month_0 = "10";
    } else if (J.t((J.eq(yesterday_month_0, "12")))) {
      yesterday_day_0 = "30";
      yesterday_month_0 = "11";
    }
  } else if (J.t((J.eq(yesterday_day_0, "02")))) {
    yesterday_day_0 = "01";
  } else if (J.t((J.eq(yesterday_day_0, "03")))) {
    yesterday_day_0 = "02";
  } else if (J.t((J.eq(yesterday_day_0, "04")))) {
    yesterday_day_0 = "03";
  } else if (J.t((J.eq(yesterday_day_0, "05")))) {
    yesterday_day_0 = "04";
  } else if (J.t((J.eq(yesterday_day_0, "06")))) {
    yesterday_day_0 = "05";
  } else if (J.t((J.eq(yesterday_day_0, "07")))) {
    yesterday_day_0 = "06";
  } else if (J.t((J.eq(yesterday_day_0, "08")))) {
    yesterday_day_0 = "07";
  } else if (J.t((J.eq(yesterday_day_0, "09")))) {
    yesterday_day_0 = "08";
  } else if (J.t((J.eq(yesterday_day_0, "10")))) {
    yesterday_day_0 = "09";
  } else if (J.t((J.eq(yesterday_day_0, "11")))) {
    yesterday_day_0 = "10";
  } else if (J.t((J.eq(yesterday_day_0, "12")))) {
    yesterday_day_0 = "11";
  } else if (J.t((J.eq(yesterday_day_0, "13")))) {
    yesterday_day_0 = "12";
  } else if (J.t((J.eq(yesterday_day_0, "14")))) {
    yesterday_day_0 = "13";
  } else if (J.t((J.eq(yesterday_day_0, "15")))) {
    yesterday_day_0 = "14";
  } else if (J.t((J.eq(yesterday_day_0, "16")))) {
    yesterday_day_0 = "15";
  } else if (J.t((J.eq(yesterday_day_0, "17")))) {
    yesterday_day_0 = "16";
  } else if (J.t((J.eq(yesterday_day_0, "18")))) {
    yesterday_day_0 = "17";
  } else if (J.t((J.eq(yesterday_day_0, "19")))) {
    yesterday_day_0 = "18";
  } else if (J.t((J.eq(yesterday_day_0, "20")))) {
    yesterday_day_0 = "19";
  } else if (J.t((J.eq(yesterday_day_0, "21")))) {
    yesterday_day_0 = "20";
  } else if (J.t((J.eq(yesterday_day_0, "22")))) {
    yesterday_day_0 = "21";
  } else if (J.t((J.eq(yesterday_day_0, "23")))) {
    yesterday_day_0 = "22";
  } else if (J.t((J.eq(yesterday_day_0, "24")))) {
    yesterday_day_0 = "23";
  } else if (J.t((J.eq(yesterday_day_0, "25")))) {
    yesterday_day_0 = "24";
  } else if (J.t((J.eq(yesterday_day_0, "26")))) {
    yesterday_day_0 = "25";
  } else if (J.t((J.eq(yesterday_day_0, "27")))) {
    yesterday_day_0 = "26";
  } else if (J.t((J.eq(yesterday_day_0, "28")))) {
    yesterday_day_0 = "27";
  } else if (J.t((J.eq(yesterday_day_0, "29")))) {
    yesterday_day_0 = "28";
  } else if (J.t((J.eq(yesterday_day_0, "30")))) {
    yesterday_day_0 = "29";
  } else if (J.t((J.eq(yesterday_day_0, "31")))) {
    yesterday_day_0 = "30";
  }
  yesterday_date_0 = J.add(J.add(J.add(J.add(yesterday_year_0, "-"), yesterday_month_0), "-"), yesterday_day_0);
  default_system_message_0 = J.add(J.add(J.add(J.add("You are Mistral-Small-3.2-24B-Instruct-2506, a Large Language Model (LLM) created by Mistral AI, a French startup headquartered in Paris.\nYou power an AI assistant called Le Chat.\nYour knowledge base was last updated on 2023-10-01.\nThe current date is ", today_date_0), ".\n\nWhen you're not sure about some information or when the user's request requires up-to-date or specific data, you must use the available tools to fetch the information. Do not hesitate to use tools whenever they can provide a more accurate or complete response. If no relevant tools are available, then clearly state that you don't have the information and avoid making up anything.\nIf the user's question is not clear, ambiguous, or does not provide enough context for you to accurately answer the question, you do not try to answer it right away and you rather ask the user to clarify their request (e.g. \"What are some good restaurants around me?\" => \"Where are you?\" or \"When is the next flight to Tokyo\" => \"Where do you travel from?\").\nYou are always very attentive to dates, in particular you try to resolve dates (e.g. \"yesterday\" is "), yesterday_date_0), ") and when asked about information at specific dates, you discard information that is at another date.\nYou follow these instructions in all languages, and always respond to the user in the language they use or request.\nNext sections describe the capabilities that you have.\n\n# WEB BROWSING INSTRUCTIONS\n\nYou cannot perform any web search or access internet to open URLs, links etc. If it seems like the user is expecting you to do so, you clarify the situation and ask the user to copy paste the text directly in the chat.\n\n# MULTI-MODAL INSTRUCTIONS\n\nYou have the ability to read images, but you cannot generate images. You also cannot transcribe audio files or videos.\nYou cannot read nor transcribe audio files or videos.\n\n# TOOL CALLING INSTRUCTIONS\n\nYou may have access to tools that you can use to fetch information or perform actions. You must use these tools in the following situations:\n\n1. When the request requires up-to-date information.\n2. When the request requires specific data that you do not have in your knowledge base.\n3. When the request involves actions that you cannot perform without tools.\n\nAlways prioritize using tools to provide the most accurate and helpful response. If tools are not available, inform the user that you cannot perform the requested action at the moment.");
  out.push(J.str(bos_token_0));
  if (J.t((J.eq(J.item(J.item(messages_0, 0), "role"), "system")))) {
    if (J.t(J.test("string", J.item(J.item(messages_0, 0), "content")))) {
      system_message_0 = J.item(J.item(messages_0, 0), "content");
    } else if (J.t(J.test("iterable", J.item(J.item(messages_0, 0), "content")))) {
      system_message_0 = J.item(J.item(J.item(J.item(messages_0, 0), "content"), 0), "text");
    } else {
      system_message_0 = J.filter("string", J.item(J.item(messages_0, 0), "content"));
    }
    loop_messages_0 = J.slice(messages_0, 1, undefined, undefined);
  } else {
    system_message_0 = default_system_message_0;
    loop_messages_0 = messages_0;
  }
  out.push(J.str(J.add(J.add("[SYSTEM_PROMPT]", system_message_0), "[/SYSTEM_PROMPT]")));
  tools_description_0 = "";
  has_tools_0 = false;
  ns_0 = J.call(J.namespace, J.kw({"append_tools_index": 0, "append_tools": false}));
  if (J.t(((v) => J.t(v) ? (J.gt(J.filter("length", tools_0), 0)) : v)(((v) => J.t(v) ? !J.t((tools_0 === null)) : v)((tools_0 !== undefined))))) {
    has_tools_0 = true;
    tools_description_0 = J.add(J.add("[AVAILABLE_TOOLS]", J.filter("tojson", tools_0)), "[/AVAILABLE_TOOLS]");
    if (J.t((J.eq(J.item(J.filter("last", loop_messages_0), "role"), "tool")))) {
      J.setattr(ns_0, "append_tools", true);
      {
        let _items1 = J.iter(loop_messages_0);
        const _loop1 = J.loop(_items1);
        for (let _i1 = 0; _i1 < _items1.length; _i1++) {
          _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
          if (J.t((J.eq(J.item(message_1, "role"), "assistant")))) {
            if (J.t(((v) => J.t(v) ? v : (J.item(message_1, "tool_calls") === null))(!J.t((J.item(message_1, "tool_calls") !== undefined))))) {
              J.setattr(ns_0, "append_tools_index", J.attr(_loop1, "index0"));
            }
          }
        }
      }
    }
  }
  {
    let _items2 = J.iter(loop_messages_0);
    const _loop2 = J.loop(_items2);
    for (let _i2 = 0; _i2 < _items2.length; _i2++) {
      _loop2.index0 = _i2; const _it2 = _items2[_i2]; let message_2 = _it2;
      let content_3 = content_0;
      let tool_call_id_4 = tool_call_id_0;
      if (J.t((J.eq(J.item(message_2, "role"), "user")))) {
        if (J.t(((v) => J.t(v) ? J.attr(_loop2, "last") : v)(has_tools_0))) {
          out.push(J.str(tools_description_0));
        }
        if (J.t(((v) => J.t(v) ? (J.eq(J.attr(ns_0, "append_tools_index"), 0)) : v)(J.attr(ns_0, "append_tools")))) {
          out.push(J.str(tools_description_0));
        }
        if (J.t(J.test("string", J.item(message_2, "content")))) {
          out.push(J.str(J.add(J.add("[INST]", J.item(message_2, "content")), "[/INST]")));
        } else {
          out.push(J.str("[INST]"));
          {
            let _items3 = J.iter(J.item(message_2, "content"));
            const _loop3 = J.loop(_items3);
            for (let _i3 = 0; _i3 < _items3.length; _i3++) {
              _loop3.index0 = _i3; const _it3 = _items3[_i3]; let block_5 = _it3;
              if (J.t((J.eq(J.item(block_5, "type"), "text")))) {
                if (J.t((J.item(block_5, "text") !== undefined))) {
                  out.push(J.str(J.item(block_5, "text")));
                } else {
                  out.push(J.str(J.item(block_5, "content")));
                }
              } else if (J.t((J.has(J.item(block_5, "type"), ["image", "image_url"])))) {
                out.push(J.str("[IMG]"));
              } else {
                out.push(J.str(J.call(J.raise_exception, "Only text and image blocks are supported in message content!")));
              }
            }
          }
          out.push(J.str("[/INST]"));
        }
      } else if (J.t((J.eq(J.item(message_2, "role"), "system")))) {
        if (J.t(J.test("string", J.item(message_2, "content")))) {
          out.push(J.str(J.add(J.add("[SYSTEM_PROMPT]", J.item(message_2, "content")), "[/SYSTEM_PROMPT]")));
        } else if (J.t(J.test("iterable", J.item(message_2, "content")))) {
          out.push(J.str(J.add(J.add("[SYSTEM_PROMPT]", J.item(J.item(J.item(message_2, "content"), 0), "text")), "[/SYSTEM_PROMPT]")));
        } else {
          out.push(J.str(J.add(J.add("[SYSTEM_PROMPT]", J.filter("string", J.item(message_2, "content"))), "[/SYSTEM_PROMPT]")));
        }
      } else if (J.t((J.eq(J.item(message_2, "role"), "assistant")))) {
        if (J.t(J.test("string", J.item(message_2, "content")))) {
          out.push(J.str(J.item(message_2, "content")));
        } else if (J.t(J.test("iterable", J.item(message_2, "content")))) {
          out.push(J.str(J.item(J.item(J.item(message_2, "content"), 0), "text")));
        } else {
          out.push(J.str(J.filter("string", J.item(message_2, "content"))));
        }
        if (J.t(((v) => J.t(v) ? (J.eq(J.attr(_loop2, "index0"), J.attr(ns_0, "append_tools_index"))) : v)(has_tools_0))) {
          out.push(J.str(eos_token_0), J.str(tools_description_0));
        }
        if (J.t(((v) => J.t(v) ? !J.t((J.item(message_2, "tool_calls") === null)) : v)((J.item(message_2, "tool_calls") !== undefined)))) {
          {
            let _items4 = J.iter(J.item(message_2, "tool_calls"));
            const _loop4 = J.loop(_items4);
            for (let _i4 = 0; _i4 < _items4.length; _i4++) {
              _loop4.index0 = _i4; const _it4 = _items4[_i4]; let tool_6 = _it4;
              let tool_call_id_7 = tool_call_id_4;
              let arguments_8 = arguments_0;
              if (J.t(!J.t((J.item(tool_6, "id") !== undefined)))) {
                out.push(J.str(J.call(J.raise_exception, "Tool ID must be provided!")));
              }
              tool_call_id_7 = J.item(tool_6, "id");
              if (J.t(((v) => J.t(v) ? v : (J.lt(J.filter("length", tool_call_id_7), 9)))(!J.t(J.test("string", tool_call_id_7))))) {
                out.push(J.str(J.call(J.raise_exception, "Tool call IDs should be alphanumeric strings with length >= 9!")));
              }
              arguments_8 = J.item(J.item(tool_6, "function"), "arguments");
              if (J.t(!J.t(J.test("string", arguments_8)))) {
                arguments_8 = J.filter("tojson", arguments_8);
              }
              out.push(J.str(J.add(J.add(J.add(J.add(J.add("[TOOL_CALLS]", J.item(J.item(tool_6, "function"), "name")), "[CALL_ID]"), tool_call_id_7), "[ARGS]"), arguments_8)));
            }
          }
        }
        if (J.t((!J.eq(J.attr(_loop2, "index0"), J.attr(ns_0, "append_tools_index"))))) {
          out.push(J.str(eos_token_0));
        }
      } else if (J.t(((v) => J.t(v) ? v : (J.eq(J.item(message_2, "role"), "tool")))((J.eq(J.item(message_2, "role"), "tool_results"))))) {
        if (J.t(((v) => J.t(v) ? (J.attr(J.attr(message_2, "content"), "content") !== undefined) : v)((J.attr(message_2, "content") !== undefined)))) {
          content_3 = J.attr(J.attr(message_2, "content"), "content");
        } else {
          content_3 = J.attr(message_2, "content");
        }
        if (J.t(!J.t((J.item(message_2, "tool_call_id") !== undefined)))) {
          out.push(J.str(J.call(J.raise_exception, "tool_call_id must be provided!")));
        }
        tool_call_id_4 = J.item(message_2, "tool_call_id");
        if (J.t(((v) => J.t(v) ? v : (J.lt(J.filter("length", tool_call_id_4), 9)))(!J.t(J.test("string", tool_call_id_4))))) {
          out.push(J.str(J.call(J.raise_exception, "Tool call IDs should be alphanumeric strings with length >= 9!")));
        }
        out.push(J.str(J.add(J.add(J.add(J.add("[TOOL_RESULTS]", tool_call_id_4), "[TOOL_CONTENT]"), J.filter("string", content_3)), "[/TOOL_RESULTS]")));
      } else {
        out.push(J.str(J.call(J.raise_exception, "Only user, systemm assistant and tool roles are supported in the custom template made by Unsloth!")));
      }
    }
  }
  return out.join("");
}
