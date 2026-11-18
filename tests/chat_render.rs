//! The chat renderer for model servers: the same ids as a fresh render of the view, whatever it
//! kept from earlier requests, across interleaved conversations and evictions.
use affordance::chat_render::{ChatRenderer, Options, RenderError};
use affordance::emit::{Encoder, RunCache};
use affordance::{View, state_from_json};
use serde_json::{Value as J, json};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn options(model: &str) -> Options {
    let index: J = serde_json::from_str(&std::fs::read_to_string(root().join("views/index.json")).unwrap()).unwrap();
    let mut options = Options::new(
        root().join("views").join(index[model]["view"].as_str().unwrap()),
        index[model]["entry"].as_str().unwrap(),
        root().join(format!("ref/tokenizers/{model}.json")),
    );
    options.special = serde_json::from_str(
        &std::fs::read_to_string(root().join(format!("ref/tokenizers/{model}.special.json"))).unwrap(),
    )
    .unwrap();
    options
}

/// What the view renders for a request with nothing kept.
fn fresh(enc: &Encoder, view: &View, options: &Options, messages: &[J], kwargs: &J) -> Vec<u32> {
    let case = json!({"messages": messages, "tools": null, "kwargs": kwargs});
    let now = chrono::DateTime::from_timestamp(1_800_000_000, 0).unwrap().naive_utc();
    view.render(
        enc,
        state_from_json(&case, &options.special),
        &mut RunCache::new(false),
        now,
    )
    .unwrap()
    .ids
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn interleaved_growing_conversations_render_as_fresh_ones() {
    let kwargs = json!({"add_generation_prompt": true});
    for model in [
        "Qwen__Qwen3.6-27B",
        "zai-org__GLM-5.3",
        "deepseek-ai__DeepSeek-V4.1-Flash",
    ] {
        let mut options = options(model);
        // Two threads keeping two conversations each, for five conversations: some are evicted
        // and rendered from nothing again, some are found.
        options.workers = 2;
        options.kept_conversations = 2;
        let renderer = ChatRenderer::json(options.clone()).unwrap();
        let enc = Encoder::from_file(options.tokenizer.to_str().unwrap()).unwrap();
        let view = View::load(&options.view, &options.entry, &enc).unwrap();
        let fresh = |messages: &[J]| fresh(&enc, &view, &options, messages, &kwargs);
        let mut conversations: Vec<Vec<J>> = (0..5)
            .map(|c| {
                vec![
                    json!({"role": "system", "content": "Be brief."}),
                    json!({"role": "user", "content": format!("task {c}")}),
                ]
            })
            .collect();
        for step in 0..6 {
            for (c, messages) in conversations.iter_mut().enumerate() {
                assert_eq!(
                    renderer.render(messages, None, &kwargs).unwrap(),
                    fresh(messages),
                    "{model}: conversation {c}, step {step}"
                );
                messages.push(json!({"role": "assistant", "content": format!("answer {c}.{step}")}));
                messages.push(json!({"role": "user", "content": format!("question {c}.{step}")}));
            }
        }
        // A conversation that changes in the middle is rendered as what it now is.
        let edited = &mut conversations[0];
        edited[3] = json!({"role": "user", "content": "a different question"});
        assert_eq!(renderer.render(edited, None, &kwargs).unwrap(), fresh(edited));
        // And one that got shorter.
        edited.truncate(4);
        assert_eq!(renderer.render(edited, None, &kwargs).unwrap(), fresh(edited));
    }
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_request_the_template_rejects_is_raised_and_the_renderer_goes_on() {
    let options = options("Qwen__Qwen3.6-27B");
    let renderer = ChatRenderer::json(options).unwrap();
    let kwargs = json!({"add_generation_prompt": true});
    let bad = [
        json!({"role": "user", "content": "hi"}),
        json!({"role": "narrator", "content": "meanwhile"}),
    ];
    match renderer.render(&bad, None, &kwargs) {
        Err(RenderError::Raised(message)) => assert!(message.contains("Unexpected message role"), "{message}"),
        other => panic!("{other:?}"),
    }
    assert!(renderer.render(&bad[..1], None, &kwargs).is_ok());
}

/// A server's own message type: converted once, the first time it is seen at its place.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_message_is_converted_once() {
    #[derive(Debug, Clone, PartialEq)]
    struct Message(&'static str, String);
    static CONVERTED: AtomicUsize = AtomicUsize::new(0);
    fn convert(m: &Message) -> Result<J, String> {
        CONVERTED.fetch_add(1, Ordering::SeqCst);
        Ok(json!({"role": m.0, "content": m.1}))
    }
    let mut options = options("zai-org__GLM-5.3");
    options.workers = 1;
    let renderer = ChatRenderer::new(options, convert).unwrap();
    let kwargs = json!({"add_generation_prompt": true});
    // (A conversation is known by its first two messages, so it starts with two here.)
    let mut messages = vec![Message("system", "Be brief.".into()), Message("user", "one".into())];
    renderer.render(&messages, None, &kwargs).unwrap();
    for i in 0..10 {
        messages.push(Message("assistant", format!("reply {i}")));
        messages.push(Message("user", format!("more {i}")));
        renderer.render(&messages, None, &kwargs).unwrap();
    }
    assert_eq!(CONVERTED.load(Ordering::SeqCst), messages.len());
}

/// A tool-using conversation: the same ids as a fresh render with its tool list, and when the
/// list changes mid-conversation the prompt is the one for the new list.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_conversation_with_tools_renders_as_a_fresh_one() {
    let kwargs = json!({"add_generation_prompt": true});
    let tool = |name: &str| {
        json!({"type": "function", "function": {"name": name, "description": "Run it.",
        "parameters": {"type": "object", "properties": {"command": {"type": "string"}}, "required": ["command"]}}})
    };
    for model in [
        "Qwen__Qwen3.6-27B",
        "zai-org__GLM-5.3",
        "deepseek-ai__DeepSeek-V4.1-Flash",
    ] {
        let options = options(model);
        let renderer = ChatRenderer::json(options.clone()).unwrap();
        let enc = Encoder::from_file(options.tokenizer.to_str().unwrap()).unwrap();
        let view = View::load(&options.view, &options.entry, &enc).unwrap();
        let fresh = |messages: &[J], tools: &J| {
            let case = json!({"messages": messages, "tools": tools, "kwargs": kwargs});
            let now = chrono::DateTime::from_timestamp(1_800_000_000, 0).unwrap().naive_utc();
            view.render(
                &enc,
                state_from_json(&case, &options.special),
                &mut RunCache::new(false),
                now,
            )
            .unwrap()
            .ids
        };
        let mut tools = json!([tool("bash")]);
        let mut messages = vec![
            json!({"role": "system", "content": "Be brief."}),
            json!({"role": "user", "content": "list the files"}),
        ];
        for step in 0..5 {
            assert_eq!(
                renderer.render(&messages, Some(&tools), &kwargs).unwrap(),
                fresh(&messages, &tools),
                "{model}: step {step}"
            );
            messages.push(
                json!({"role": "assistant", "content": "", "tool_calls": [{"id": format!("c{step}"), "type": "function",
                "function": {"name": "bash", "arguments": {"command": format!("ls step{step}")}}}]}),
            );
            messages.push(json!({"role": "tool", "tool_call_id": format!("c{step}"), "content": "a.rs\nb.rs\n"}));
            assert_eq!(
                renderer.render(&messages, Some(&tools), &kwargs).unwrap(),
                fresh(&messages, &tools),
                "{model}: step {step}, after the result"
            );
            messages.push(json!({"role": "assistant", "content": format!("done {step}")}));
            messages.push(json!({"role": "user", "content": "again"}));
            if step == 2 {
                tools = json!([tool("bash"), tool("read")]);
            }
        }
    }
}
