// Converted from the chat template of zai-org/GLM-Z1-Rumination-32B-0414 by tools/jinja2tsx.py. Regenerate it; do not edit.
import { J } from "../../common/jinja.tsx";

export default function Main(ctx) {
  const out = [];
  let messages_0 = ctx["messages"];
  let role_0 = ctx["role"];
  let content_0 = ctx["content"];
  let visible_0 = ctx["visible"];
  let meta_0 = ctx["meta"];
  let add_generation_prompt_0 = ctx["add_generation_prompt"];
  out.push("[gMASK]<sop>\n<|system|>\n\u4f60\u662f\u4e00\u4e2a\u4e13\u4e1a\u7684\u6df1\u5ea6\u7814\u7a76\u52a9\u624b\uff0c\u901a\u8fc7\u63d0\u4f9b\u7684\u5de5\u5177\u4e0e\u6a21\u62df\u6d4f\u89c8\u5668\u4ea4\u4e92\uff0c\u6765\u5e2e\u52a9\u7528\u6237\u5b8c\u6210\u6df1\u5ea6\u4fe1\u606f\u8c03\u7814\u548c\u62a5\u544a\u64b0\u5199\u4efb\u52a1\u3002\u4eca\u5e74\u662f 2025 \u5e74\u3002\n\n<\u6838\u5fc3\u8981\u6c42>\n- \u9996\u5148\u5206\u89e3\u7528\u6237\u8bf7\u6c42\uff0c\u5f97\u5230\u5305\u542b\u591a\u4e2a\u5b50\u8981\u6c42\u7684\u5217\u8868\n- \u5236\u5b9a\u521d\u59cb\u7814\u7a76\u8ba1\u5212\n- \u8fdb\u884c\u591a\u8f6e\u8fed\u4ee3\u641c\u7d22\u548c\u9875\u9762\u6d4f\u89c8\uff08at least 10 function calls\uff09\uff1a\n    * \u6839\u636e\u5df2\u83b7\u5f97\u7684\u4fe1\u606f\u8c03\u6574\u7814\u7a76\u8ba1\u5212\u548c\u5173\u952e\u8bcd\n    * \u6253\u5f00\u9875\u9762\u9605\u8bfb\uff0c\u4ece\u53d1\u73b0\u7684\u5185\u5bb9\u4e2d\u8bc6\u522b\u65b0\u7684\u5173\u952e\u6982\u5ff5/\u540d\u8bcd\n    * \u4ece\u641c\u7d22\u7ed3\u679c\u4e2d\u63d0\u53d6\u65b0\u7684\u5173\u952e\u8bcd\u7ee7\u7eed\u641c\u7d22\n    * \u8bbf\u95ee\u5e76\u4ed4\u7ec6\u9605\u8bfb\u76f8\u5173\u9875\u9762\uff0c\u8bc6\u522b\u65b0\u7684\u5173\u952e\u6982\u5ff5/\u540d\u8bcd\n\n<\u91cd\u8981\u914d\u7f6e>\n- \u91c7\u7528\u8bed\u8a00\n    * \u641c\u7d22\u5173\u952e\u8bcd\uff1a\u82f1\u6587\n    * \u601d\u8003\uff1a\u82f1\u6587\n\n<\u53ef\u8c03\u7528\u7684\u5de5\u5177\u5217\u8868>\n[{\"name\": \"search\", \"description\": \"Execute a search query and return search results. Use this function when you need to find information about a specific topic.\", \"parameters\": {\"type\": \"object\", \"properties\": {\"query\": {\"type\": \"string\", \"description\": \"Search query string, use English words unless it is a proper name in Chinese\"}}, \"required\": [\"query\"], \"additionalProperties\": false}}, {\"name\": \"click\", \"description\": \"Click a link in the search results and navigate to the corresponding page. Use this function when you need to view detailed content of a specific search result.\", \"parameters\": {\"type\": \"object\", \"properties\": {\"link_id\": {\"type\": \"integer\", \"description\": \"The link ID to click (from the sequence number in search results)\"}}, \"required\": [\"link_id\"], \"additionalProperties\": false}}, {\"name\": \"open\", \"description\": \"Open a specific website. Get content from any website with its URL.\", \"parameters\": {\"type\": \"object\", \"properties\": {\"url\": {\"type\": \"string\", \"description\": \"The target website URL or domain\"}}, \"required\": [\"url\"], \"additionalProperties\": false}}, {\"name\": \"finish\", \"description\": \"Finish the task. Use this function when you have found the information you need.\", \"parameters\": {\"type\": \"object\", \"properties\": {}, \"additionalProperties\": false}}]");
  {
    let _items1 = J.iter(messages_0);
    _items1 = _items1.filter(_it1 => { let message_1 = _it1; return J.t((!J.eq(J.attr(message_1, "role"), "system"))); });
    const _loop1 = J.loop(_items1);
    for (let _i1 = 0; _i1 < _items1.length; _i1++) {
      _loop1.index0 = _i1; const _it1 = _items1[_i1]; let message_1 = _it1;
      let role_2 = role_0;
      let content_3 = content_0;
      let visible_4 = visible_0;
      let meta_5 = meta_0;
      role_2 = J.item(message_1, "role");
      content_3 = J.item(message_1, "content");
      visible_4 = J.call(J.attr(J.item(J.call(J.attr(content_3, "split"), "</think>"), J.neg(1)), "strip"));
      meta_5 = J.call(J.attr(message_1, "get"), "metadata", "");
      if (J.t((J.eq(role_2, "user")))) {
        out.push("<|user|>\n", J.str(visible_4));
      } else if (J.t(((v) => J.t(v) ? !J.t(meta_5) : v)((J.eq(role_2, "assistant"))))) {
        out.push("<|assistant|>\n", J.str(visible_4));
      } else if (J.t(((v) => J.t(v) ? meta_5 : v)((J.eq(role_2, "assistant"))))) {
        out.push("<|assistant|>", J.str(meta_5), "\n", J.str(visible_4));
      } else if (J.t((J.eq(role_2, "observation")))) {
        out.push("<|observation|>\n", J.str(visible_4));
      }
    }
  }
  if (J.t(add_generation_prompt_0)) {
    out.push("<|assistant|>");
  }
  return out.join("");
}
