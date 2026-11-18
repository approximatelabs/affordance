//! A malformed reply is a parse error, never a panic (a panic in a parser would kill a render
//! worker and strand every agent on it).
use super::{atem, dsml, harmony, llama4, python_value};
use crate::reply::{ActionDef, ActionSet, Syntax, parse_reply_typed};
use serde_json::{Value as J, json};

#[test]
fn an_invoke_header_closing_after_its_end_tag_is_an_error_not_a_panic() {
    let d = "｜DSML｜";
    let text = format!(
        "<{d} calls>\n<{d} invoke name=\"shell</{d} invoke>\n<{d} invoke name=\"x\">\n<{d} parameter name=\"command\" string=\"true\">ls</{d} parameter>\n</{d} invoke>\n</{d} calls>"
    );
    let parts = dsml(&text);
    assert!(
        parts.errors.iter().any(|e| e.contains("malformed invoke")),
        "{:?}",
        parts.errors
    );
}

/// Atem's invoke and parameter headers, closing after their end tags.
#[test]
fn an_atem_header_closing_after_its_end_tag_is_an_error_not_a_panic() {
    let call = |inner: &str| {
        atem(&format!(
            "<|start|>assistant to=functions.shell<|message|><atem:function_calls>{inner}</atem:function_calls>"
        ))
    };
    let parts = call("<atem:invoke name=\"</atem:invoke>\">");
    assert!(
        parts.errors.iter().any(|e| e.contains("malformed atem:invoke")),
        "{:?}",
        parts.errors
    );
    let parts = call("<atem:invoke name=\"x\"><atem:parameter name=\"</atem:parameter>\"></atem:invoke>");
    assert!(
        parts.calls.is_empty()
            && parts
                .errors
                .iter()
                .any(|e| e.contains("malformed atem:parameter in the call to x")),
        "{parts:?}"
    );
}

/// Harmony quotes the start of a message without `<|message|>`: by chars, not bytes.
#[test]
fn a_harmony_message_quoted_mid_character_is_an_error_not_a_panic() {
    let parts = harmony(&format!("{}— and more", "a".repeat(59)));
    assert_eq!(
        parts.errors,
        [format!(
            "message without <|message|>: {:?}",
            format!("{}—", "a".repeat(59))
        )]
    );
}

/// A Llama 4 literal never closed is a bad value, not a length past the text.
#[test]
fn an_unclosed_llama4_literal_is_an_error_not_a_panic() {
    let parts = llama4("[f(x=['a\"]");
    assert!(
        parts.calls.is_empty() && parts.errors.iter().any(|e| e.contains("bad value for `x`")),
        "{parts:?}"
    );
}

/// A Python literal nested past any a model writes is not a value, not a stack overflow.
#[test]
fn a_deeply_nested_python_literal_is_not_a_value() {
    let deep = format!("{}{}", "[".repeat(100_000), "]".repeat(100_000));
    assert_eq!(python_value(&deep), None);
    assert_eq!(
        python_value("[[1], (2,), {'a': None}]"),
        Some(json!([[1], [2], {"a": null}]))
    );
}

/// Reply arguments are decoded, never rewritten. Each row is (syntax, in_think, reply, the
/// properties of the one action `run`) -> (the calls' arguments, problems).
#[test]
fn reply_arguments_decode_as_written() {
    let d = "｜DSML｜";
    let nullable = json!({"type": ["string", "null"]});
    let glm = |args: &[(&str, &str)]| {
        args.iter().fold("<tool_call>run".to_string(), |s, (k, v)| {
            s + &format!("<arg_key>{k}</arg_key><arg_value>{v}</arg_value>")
        }) + "</tool_call>"
    };
    let rows: Vec<(Syntax, bool, String, J, J, &[&str])> = vec![
        // A Python dict keeps `None` and `True` inside its strings, and an apostrophe in a string.
        (
            Syntax::Glm,
            false,
            glm(&[("x", "{'note': 'None yet', 'mode': 'True', 'q': \"it's\"}")]),
            json!({"x": {"type": "object"}}),
            json!([{"x": {"note": "None yet", "mode": "True", "q": "it's"}}]),
            &[],
        ),
        (
            Syntax::Llama4,
            false,
            r"[run(x={'a': ['it\'s', (1, 2), (3), None, 'a\\b'], 'b': {}})]".into(),
            json!({"x": {"type": "object"}}),
            json!([{"x": {"a": ["it's", [1, 2], 3, null, "a\\b"], "b": {}}}]),
            &[],
        ),
        // A nullable string keeps text that would parse as another type.
        (
            Syntax::Glm,
            false,
            glm(&[("a", "2026"), ("b", "true"), ("c", "null"), ("d", "hello")]),
            json!({"a": nullable, "b": nullable, "c": nullable, "d": nullable}),
            json!([{"a": "2026", "b": "true", "c": null, "d": "hello"}]),
            &[],
        ),
        // The call list is the one that reaches the end of the reply, not a mention before it.
        (
            Syntax::Llama4,
            false,
            "I could [run(x=1)] but [run(x=2)]".into(),
            json!({"x": {"type": "integer"}}),
            json!([{"x": 2}]),
            &["prose_before_action"],
        ),
        // An XML function body is JSON or parameters, and an empty one is no arguments.
        (
            Syntax::Xml,
            false,
            "<tool_call>\n<function=run>hello</function>\n</tool_call>".into(),
            json!({}),
            json!([]),
            &["bad_syntax", "no_action"],
        ),
        (
            Syntax::Xml,
            false,
            "<tool_call>\n<function=run>\n</function>\n</tool_call>".into(),
            json!({}),
            json!([{}]),
            &[],
        ),
        // With no `<think>` open, a `</think>` ends thinking only before the first call.
        (
            Syntax::Dsml,
            false,
            format!(
                "<{d}function_calls>\n<{d}invoke name=\"run\">\n<{d}parameter name=\"x\" string=\"true\">a </think> b</{d}parameter>\n</{d}invoke>\n</{d}function_calls>"
            ),
            json!({"x": {"type": "string"}}),
            json!([{"x": "a </think> b"}]),
            &[],
        ),
        (
            Syntax::Glm,
            false,
            glm(&[("x", "a </think> b")]),
            json!({"x": {"type": "string"}}),
            json!([{"x": "a </think> b"}]),
            &[],
        ),
        (
            Syntax::Glm,
            false,
            format!("hm</think>{}", glm(&[("x", "a </think> b")])),
            json!({"x": {"type": "string"}}),
            json!([{"x": "a </think> b"}]),
            &[],
        ),
    ];
    let mut wrong = vec![];
    for (syntax, in_think, reply, props, calls, problems) in rows {
        let actions = ActionSet {
            pattern: None,
            actions: vec![ActionDef {
                name: "run".into(),
                schema: json!({"type": "object", "properties": props}),
                default: None,
                code: None,
            }],
        };
        let r = parse_reply_typed(&reply, syntax, &actions, in_think, false);
        let got = (
            J::Array(r.calls.iter().map(|c| c.args.clone()).collect()),
            r.problem_names(),
        );
        if got != (calls.clone(), problems.to_vec()) {
            wrong.push(format!(
                "{syntax:?} {reply:?}\n  got {got:?} {:?}\n  want {calls} {problems:?}",
                r.problems
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
