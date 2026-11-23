//! Images on the native model wire: an `<img src>` in a model's prompt (a tool result's image
//! content part rendered by the template, or an `<img>` on the screen) reaches the model request.
//! The ids hold its placeholder token, the request carries its source (`image_data`, in prompt
//! order), and logged records keep it for later turns. The prompt stays an exact append.
use affordance::View;
use affordance::effects::Outcome;
use affordance::emit::Encoder;
use affordance::runtime::{Agent, AgentConfig, Command, Input};
use serde_json::{Value as J, json};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn encoder() -> Encoder {
    Encoder::from_file(
        root()
            .join("ref/tokenizers/deepseek-ai__DeepSeek-V4.1-Flash.json")
            .to_str()
            .unwrap(),
    )
    .unwrap()
}

const PNG: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAIAAABLbSncAAAAEklEQVR4nGP4z8CAFWEXHbQSACj/P8Fu7N9hAAAAAElFTkSuQmCC";

/// A DeepSeek V4.1 agent whose `look` tool returns text and an image, and whose screen shows an
/// image (`screen`) while `pinned` is set.
fn app(enc: &Encoder, screen: &str) -> View {
    let src = format!(
        r#"
import {{ DeepSeekV41App }} from "{root}/views/template_apps/deepseek_v41.tsx";
import {{ TurnStrategy }} from "{root}/views/strategies/turn_strategy.tsx";
import {{ useConversation, useLoop }} from "{root}/views/lib/loop.tsx";
import {{ useLog }} from "{root}/views/lib/log.tsx";
export default function Main({{ pinned }}) {{
  const log = useLog();
  useLoop(log);
  const look = async () => ({{result: [{{type: "text", text: "a red pixel"}}, {{type: "image_url", image_url: {{url: "{PNG}"}}}}]}});
  return <DeepSeekV41App keep_replies={{true}}>
    <TurnStrategy strategy="unrolled" log={{log.records}}>
      <aff label="look" tool_name="look" description="Look at the picture." args_schema={{{{type: "object", properties: {{}}}}}} onRun={{look}}>{{"[look]\n"}}</aff>
      {{pinned ? <img src="{screen}" t="<｜deepseek_image｜>" tok_size={{1}}/> : ""}}
    </TurnStrategy>
  </DeepSeekV41App>;
}}
"#,
        root = root().display()
    );
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("aff-images-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("app.tsx");
    std::fs::write(&path, src).unwrap();
    View::load(&path, "Main", enc).unwrap()
}

fn config(pinned: bool) -> AgentConfig {
    AgentConfig {
        context: None,
        retry: Default::default(),
        template: None,
        id: "a".into(),
        props: json!({"pinned": pinned}),
        env: BTreeMap::new(),
        max_new: 64,
        max_turns: Some(3),
        max_prompt: None,
        start_ms: 1_791_000_000_000,
        seed: 7,
        machines: vec![],
        model: Some("default".into()),
    }
}

/// Run an agent whose replies call `look` once, then end: the Generate commands it sent.
fn run(pinned: bool) -> (Vec<(Vec<u32>, Vec<String>)>, Agent) {
    let enc = encoder();
    let view = app(&enc, PNG);
    let eos = enc.token_id("<｜end▁of▁sentence｜>").unwrap();
    let mut agent = Agent::new(config(pinned)).unwrap();
    let mut queue: std::collections::VecDeque<Command> = agent.start(&view, &enc).unwrap().into();
    let (mut sent, mut clock) = (vec![], 1_791_000_000_000i64);
    while let Some(cmd) = queue.pop_front() {
        clock += 10;
        let input = match cmd {
            Command::Generate { task, ids, images, .. } => {
                let text = if sent.is_empty() {
                    "Look.</think>\n\n<｜DSML｜ calls>\n<｜DSML｜ invoke name=\"look\">\n</｜DSML｜ invoke>\n</｜DSML｜ calls>"
                } else {
                    "Seen.</think>It is red."
                };
                sent.push((ids.to_vec(), images));
                let mut out = enc.tok.encode(text, false).unwrap().get_ids().to_vec();
                out.push(eos);
                Input::Reply {
                    server: None,
                    task,
                    ids: out,
                    cut_off: false,
                    clock_ms: clock,
                    prompt: None,
                    aborted: false,
                }
            }
            Command::Call { task, .. } => Input::Result {
                task,
                outcome: Outcome::Ok(J::Null),
                clock_ms: clock,
            },
            _ => continue,
        };
        queue.extend(agent.handle(&view, &enc, input).unwrap());
    }
    (sent, agent)
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_tool_results_image_reaches_the_model_request() {
    let enc = encoder();
    let placeholder = enc.token_id("<｜deepseek_image｜>").unwrap();
    let (sent, agent) = run(false);
    assert!(sent.len() >= 2, "{}", sent.len());
    // The first turn has no image; the turn after `look` holds the picture as DeepSeek V4.1's
    // processor lays it out (an 8 by 8 picture is brought up to its least size: 184 tokens, as
    // SGLang's own sizing code counts it) and sends the image.
    assert_eq!(sent[0].1.len(), 0);
    assert_eq!(sent[1].0.iter().filter(|&&t| t == placeholder).count(), 184);
    assert_eq!(sent[1].1, vec![PNG.to_string()]);
    // The record keeps the content parts; the prompt is an exact append.
    let action = affordance::scenario::log_records(&agent.state())
        .into_iter()
        .find(|r| r["kind"] == "action")
        .unwrap();
    assert_eq!(action["result"][1]["image_url"]["url"], PNG);
    assert_eq!(agent.metrics.rewrites, 0);
}

#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn a_screens_image_stays_with_its_logged_screen() {
    let enc = encoder();
    let placeholder = enc.token_id("<｜deepseek_image｜>").unwrap();
    let (sent, agent) = run(true);
    // Turn 1: the screen's image. Turn 2: the logged screen's image, the tool result's, and the
    // current screen's, in prompt order. The tool's picture is laid out by the model's rule (184
    // tokens); the screens' pictures state one token each (`tok_size`, the app's own statement).
    assert_eq!(sent[0].1, vec![PNG.to_string()]);
    assert_eq!(sent[1].1.len(), 3);
    assert_eq!(sent[1].0.iter().filter(|&&t| t == placeholder).count(), 1 + 184 + 1);
    assert!(
        affordance::scenario::log_records(&agent.state())
            .iter()
            .any(|r| r["kind"] == "screen" && r["images"][0]["src"] == PNG)
    );
    assert_eq!(agent.metrics.rewrites, 0);
    assert!(sent[1].0.starts_with(&sent[0].0), "exact append");
}

#[test]
fn the_sglang_request_carries_images_audio_and_video_each_in_order() {
    use affordance::runtime::{
        cancel::CancelToken,
        model::{GenRequest, Model, Sglang},
    };
    use std::io::{BufRead, BufReader, Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let peer = std::thread::spawn(move || {
        let (conn, _) = listener.accept().unwrap();
        let mut r = BufReader::new(conn.try_clone().unwrap());
        let mut len = 0;
        loop {
            let mut h = String::new();
            r.read_line(&mut h).unwrap();
            if h.trim().is_empty() {
                break;
            }
            if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
                len = v.trim().parse().unwrap();
            }
        }
        let mut body = vec![0; len];
        r.read_exact(&mut body).unwrap();
        let text = json!({"output_ids": [5, 1], "meta_info": {"prompt_tokens": 3, "finish_reason": {"type": "stop", "matched": 1}}}).to_string();
        let mut conn = conn;
        conn.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{text}", text.len()).as_bytes())
            .unwrap();
        serde_json::from_slice::<J>(&body).unwrap()
    });
    let mut model = Sglang::new(&url, 1);
    model.stream = false;
    // Sources in prompt order: two images with an audio clip and a video between them.
    let req = GenRequest {
        images: vec![
            "data:image/png;base64,AAA".into(),
            "data:audio/wav;base64,CCC".into(),
            "data:video/mp4;base64,DDD".into(),
            "BBB".into(),
        ],
        ..GenRequest::new("a", vec![7, 129264, 129264], 8, vec![1], None, CancelToken::new())
    };
    model.generate(&req).unwrap();
    let body = peer.join().unwrap();
    assert_eq!(body["image_data"], json!(["data:image/png;base64,AAA", "BBB"]));
    assert_eq!(body["audio_data"], json!(["data:audio/wav;base64,CCC"]));
    assert_eq!(body["video_data"], json!(["data:video/mp4;base64,DDD"]));
    assert_eq!(body["input_ids"], json!([7, 129264, 129264]));
}

/// "A picture and a sound clip a tool returns reach the model at their real size, on a model
/// bound by name; a video it has no size rule for is shown as its alt text." Gemma 4's format
/// is the publisher's template, converted. Its tokenizer's rules give an 8 by 8 picture 256
/// tokens and one second of sound 27. It has no rule for a video, so the video is not laid out
/// at a guess: its alt text stands in, the span says why, and its file is not sent.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_tool_results_image_and_audio_reach_a_model_bound_by_name_and_a_video_without_a_rule_shows_alt() {
    use affordance::emit::RunCache;
    use affordance::value::Value;
    let enc = Encoder::from_file(
        root()
            .join("ref/tokenizers/google__gemma-4-31B-it.json")
            .to_str()
            .unwrap(),
    )
    .unwrap();
    let view = View::load_model(
        &root().join("tests/fixtures/any_model_agent.tsx"),
        "Main",
        &enc,
        "google/gemma-4-31B-it",
        &json!({}),
    )
    .unwrap();
    // One second of silence: 16 kHz, 16-bit, one channel (a 44-byte header and 32,000 bytes).
    let second = {
        let mut wav = b"RIFF".to_vec();
        wav.extend_from_slice(&(36 + 32_000u32).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&[1, 0, 1, 0]);
        wav.extend_from_slice(&16_000u32.to_le_bytes());
        wav.extend_from_slice(&32_000u32.to_le_bytes());
        wav.extend_from_slice(&[2, 0, 16, 0]);
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&32_000u32.to_le_bytes());
        wav.extend_from_slice(&vec![0u8; 32_000]);
        // 32,044 bytes is a multiple of 3 less 1... written out in base64 by hand below.
        const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for c in wav.chunks(3) {
            let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
            out.push(A[(n >> 18) as usize & 63] as char);
            out.push(A[(n >> 12) as usize & 63] as char);
            out.push(if c.len() > 1 {
                A[(n >> 6) as usize & 63] as char
            } else {
                '='
            });
            out.push(if c.len() > 2 { A[n as usize & 63] as char } else { '=' });
        }
        out
    };
    let video = "data:video/mp4;base64,AAAAIGZ0eXA=";
    let log = json!([
        {"kind": "input", "content": "Look, listen and watch."},
        {"kind": "model", "content": "", "calls": [{"id": "c1", "type": "function", "function": {"name": "shell", "arguments": {"command": "play"}}}]},
        {"kind": "action", "name": "shell", "tool_call_id": "c1", "result": [
            {"type": "text", "text": "Three things."},
            {"type": "image_url", "image_url": {"url": PNG}},
            {"type": "input_audio", "input_audio": {"data": second, "format": "wav"}},
            {"type": "video_url", "video_url": {"url": video}},
        ]},
    ]);
    let Value::Map(state) = Value::from_json(&json!({"log": log})) else {
        unreachable!()
    };
    let now = chrono::NaiveDate::from_ymd_opt(2026, 10, 5)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let prompt = view
        .render(
            &enc,
            std::rc::Rc::new(state.into_data()),
            &mut RunCache::new(false),
            now,
        )
        .unwrap();
    let mut media: Vec<_> = prompt.spans.iter().filter(|s| s.meta.get("img").is_some()).collect();
    media.sort_by_key(|s| s.start);
    let carried: Vec<(String, usize)> = media
        .iter()
        .map(|s| (enc.tok.id_to_token(prompt.ids[s.start]).unwrap(), s.end - s.start))
        .collect();
    assert_eq!(carried, [("<|image|>".to_string(), 256), ("<|audio|>".to_string(), 27)]);
    assert_eq!(media[0].meta["src"], PNG);
    let missing: Vec<_> = prompt
        .spans
        .iter()
        .filter(|s| s.meta.get("not_shown").is_some())
        .collect();
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].meta["media"], "video");
    assert_eq!(
        missing[0].meta["not_shown"],
        "this model's tokenizer has no size rule for a video"
    );
    let text = enc.tok.decode(&prompt.ids, false).unwrap();
    assert!(text.contains("[video not shown]"), "{text}");
}

/// "On a chat binding, a picture, a sound and a video go to the server as message parts." The
/// server applies the model's template to them; a token endpoint that takes no media (vLLM's
/// completions) is why a model with media is bound this way there.
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn media_in_a_message_goes_to_a_chat_server_as_parts() {
    use affordance::emit::RunCache;
    use affordance::value::Value;
    let enc = Encoder::from_file(root().join("ref/tokenizers/Qwen__Qwen3-8B.json").to_str().unwrap()).unwrap();
    let view = View::load_model(
        &root().join("tests/fixtures/any_model_agent.tsx"),
        "Main",
        &enc,
        "messages",
        &json!({}),
    )
    .unwrap();
    let video = "data:video/mp4;base64,AAAAIGZ0eXA=";
    let parts = json!([
        {"type": "image_url", "image_url": {"url": PNG}},
        {"type": "input_audio", "input_audio": {"data": "UklGRg==", "format": "wav"}},
        {"type": "video_url", "video_url": {"url": video}},
        {"type": "text", "text": "What are these?"},
    ]);
    let Value::Map(state) = Value::from_json(&json!({"log": [{"kind": "input", "content": parts}]})) else {
        unreachable!()
    };
    let now = chrono::NaiveDate::from_ymd_opt(2026, 10, 5)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let prompt = view
        .render(
            &enc,
            std::rc::Rc::new(state.into_data()),
            &mut RunCache::new(false),
            now,
        )
        .unwrap();
    let request: J = serde_json::from_str(&enc.tok.decode(&prompt.ids, false).unwrap()).unwrap();
    let sent = request["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["role"] == "user" && m["content"].is_array())
        .expect("a user message with parts");
    assert_eq!(sent["content"], parts);
}

/// Live, on demand: one request with one image to the SGLang server at `AFFORDANCE_SGLANG_URL`.
#[test]
#[ignore]
fn live_image_request() {
    use affordance::runtime::{
        cancel::CancelToken,
        model::{GenRequest, Model, Sglang},
    };
    let enc = encoder();
    let eos = enc.token_id("<｜end▁of▁sentence｜>").unwrap();
    let mut model = Sglang::new(&std::env::var("AFFORDANCE_SGLANG_URL").unwrap(), 1);
    model.stream = false;
    model.sampling = json!({"temperature": 0.0});
    let ids = enc.tok.encode("<｜begin▁of▁sentence｜><｜User｜><｜deepseek_image｜>What color is this image? Answer with one word.<｜Assistant｜></think>", false).unwrap().get_ids().to_vec();
    let r = model
        .generate(&GenRequest {
            images: vec![PNG.into()],
            ..GenRequest::new("aff-image-live", ids.clone(), 16, vec![eos], None, CancelToken::new())
        })
        .unwrap();
    let text = enc.tok.decode(&r.ids, false).unwrap();
    eprintln!(
        "sent {} ids, prompt_tokens {}, reply {text:?}",
        ids.len(),
        r.model_meta.as_ref().unwrap()["prompt_tokens"]
    );
    assert!(text.to_lowercase().contains("red"), "{text}");
}

/// A request carries an image's bytes: base64 is sent as a data URL; a URL or a path, which the
/// model server would fetch or open, rejects the call before it is sent (`useLoop` stops with it).
#[test]
#[cfg_attr(not(feature = "test-tokenizers"), ignore = "needs tools/fetch-test-tokenizers.sh")]
fn an_image_source_is_data_never_a_url_or_a_path() {
    let enc = encoder();
    let start = |screen: &str| {
        let view = app(&enc, screen);
        let mut agent = Agent::new(config(true)).unwrap();
        let commands = agent.start(&view, &enc).unwrap();
        (commands, agent)
    };
    let images = |cmds: Vec<Command>| {
        cmds.into_iter().find_map(|c| match c {
            Command::Generate { images, .. } => Some(images),
            _ => None,
        })
    };
    let jpeg = "/9j/4AAQSkZJRg==";
    assert_eq!(
        images(start(jpeg).0),
        Some(vec![format!("data:image/jpeg;base64,{jpeg}")])
    );
    for src in [
        "http://169.254.169.254/latest/meta-data/",
        "file:///etc/x.png",
        "/x.png",
        "x.png",
    ] {
        let (commands, agent) = start(src);
        assert_eq!(images(commands), None, "{src} was sent");
        let stopped = agent.stopped().cloned().unwrap_or_default();
        assert_eq!(stopped["reason"], "loop_error", "{src}: {stopped}");
        assert!(
            stopped["error"]
                .as_str()
                .unwrap_or("")
                .contains("an image source must be a data URL"),
            "{src}: {stopped}"
        );
    }
}
