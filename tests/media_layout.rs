//! Media as layout: a picture takes the tokens the style sheet gives it, and the file that is
//! sent has that size. Each test is something a user would say, run on a real agent, the real
//! SGLang binding against a real HTTP server, and a model's registered format.

use affordance::View;
use affordance::effects::Outcome;
use affordance::emit::Encoder;
use affordance::runtime::{Command, Input, Instance, InstanceConfig};
use serde_json::{Value as J, json};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn encoder() -> Encoder {
    Encoder::from_file(root().join("ref/tokenizers/Qwen__Qwen3.5-4B.json").to_str().unwrap()).unwrap()
}

/// Qwen3.5's picture metrics, as its row in views/index.json has them.
const METRICS: &str = r#"{kind: "grid", cell: 32, min: 65536, max: 16777216}"#;

/// A picture of `width` by `height`: a white one made the size wanted.
fn picture(width: u32, height: u32) -> String {
    let white = "iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAIAAABLbSncAAAAEklEQVR4nGP4z8CAFWEXHbQSACj/P8Fu7N9hAAAAAElFTkSuQmCC";
    affordance::media::resized(&affordance::media::bytes(white).unwrap(), width, height).unwrap()
}

/// An agent whose first message is a picture, under a style sheet.
fn app(enc: &Encoder, sheet: &str, picture: &str) -> View {
    app_with(enc, sheet, picture, METRICS)
}

/// The same, with a picture rule of the test's own.
fn app_with(enc: &Encoder, sheet: &str, picture: &str, metrics: &str) -> View {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("aff-media-layout-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("app.tss"), sheet).unwrap();
    let src = format!(
        r#"
import {{ Qwen36App }} from "{root}/views/template_apps/qwen36.tsx";
import {{ TurnStrategy }} from "{root}/views/strategies/turn_strategy.tsx";
import {{ useLoop }} from "{root}/views/lib/loop.tsx";
import {{ useLog }} from "{root}/views/lib/log.tsx";
import "./app.tss";
export default function Main() {{
  const log = useLog();
  useLoop(log);
  return <Qwen36App>
    <TurnStrategy strategy="unrolled" log={{log.records}}>
      {{"Look: "}}<img src="{picture}" t="<|image_pad|>" metrics={{{metrics}}} alt="[a picture that did not fit]"/>{{"\n"}}
    </TurnStrategy>
  </Qwen36App>;
}}
"#,
        root = root().display(),
        metrics = metrics
    );
    let path = dir.join("app.tsx");
    std::fs::write(&path, src).unwrap();
    View::load(&path, "Main", enc).unwrap()
}

/// The first model call an agent on `view` makes: its ids, its files and where its media are.
fn first_call(view: &View, enc: &Encoder) -> (Vec<u32>, Vec<String>, Vec<(usize, usize, u32)>) {
    let mut agent = Instance::new(InstanceConfig {
        context: Some(32_768),
        retry: Default::default(),
        template: None,
        id: "a".into(),
        props: json!({}),
        env: BTreeMap::new(),
        max_new: 64,
        max_turns: Some(1),
        max_prompt: None,
        start_ms: 1_791_000_000_000,
        seed: 7,
        machines: vec![],
        model: Some("default".into()),
    })
    .unwrap();
    let mut queue: std::collections::VecDeque<Command> = agent.start(view, enc).unwrap().into();
    while let Some(cmd) = queue.pop_front() {
        match cmd {
            Command::Generate { ids, images, media, .. } => return (ids.to_vec(), images, media),
            Command::Call { task, .. } => queue.extend(
                agent
                    .handle(
                        view,
                        enc,
                        Input::Result {
                            task,
                            outcome: Outcome::Ok(J::Null),
                            clock_ms: 1_791_000_000_010,
                        },
                    )
                    .unwrap(),
            ),
            _ => {}
        }
    }
    panic!("the agent made no model call");
}

fn size_of(source: &str) -> (u32, u32) {
    affordance::media::dimensions(&affordance::media::bytes(source).unwrap()).unwrap()
}

/// "A picture takes the tokens its model's processor gives it." With no rule, a 1280 by 720
/// picture is laid out on Qwen3.5's 32-pixel grid: 1280 by 704 (720 / 32 = 22.5 rounds to the even 22, as Python's
/// round() does), 40 by 22 cells, 880 tokens.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_picture_takes_the_tokens_its_models_processor_gives_it() {
    let enc = encoder();
    let pad = enc.token_id("<|image_pad|>").unwrap();
    let (ids, files, media) = first_call(&app(&enc, "", &picture(1280, 720)), &enc);
    assert_eq!(ids.iter().filter(|&&t| t == pad).count(), 880);
    assert_eq!(media.len(), 1);
    assert_eq!(media[0].1, 880);
    assert!(ids[media[0].0..media[0].0 + 880].iter().all(|&t| t == pad));
    assert_eq!(size_of(&files[0]), (1280, 704));
}

/// "A rule in the style sheet caps a picture, and the file that is sent has that size."
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_rule_caps_a_picture_and_the_file_sent_has_that_size() {
    let enc = encoder();
    let pad = enc.token_id("<|image_pad|>").unwrap();
    let (ids, files, media) = first_call(&app(&enc, "img { max-tokens: 100tok; }", &picture(1280, 720)), &enc);
    let n = ids.iter().filter(|&&t| t == pad).count();
    assert!((64..=100).contains(&n), "{n} tokens under a cap of 100");
    let (w, h) = size_of(&files[0]);
    assert_eq!((w % 32, h % 32), (0, 0));
    assert_eq!((w / 32 * (h / 32)) as usize, n, "{w}x{h} is {n} tokens");
    assert!(w > h, "the shape is kept: {w}x{h}");
    assert_eq!(media, vec![(ids.iter().position(|&t| t == pad).unwrap(), n, pad)]);
    // The same picture and rule give the same bytes.
    assert_eq!(
        files,
        first_call(&app(&enc, "img { max-tokens: 100tok; }", &picture(1280, 720)), &enc).1
    );
}

/// "A share of the window caps a picture too." 1% of a 32,768-token context less the reply's
/// 64 is 327 tokens.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_share_of_the_window_caps_a_picture() {
    let enc = encoder();
    let pad = enc.token_id("<|image_pad|>").unwrap();
    let (ids, ..) = first_call(&app(&enc, "img { max-tokens: 1win; }", &picture(1280, 720)), &enc);
    let n = ids.iter().filter(|&&t| t == pad).count();
    assert!((250..=327).contains(&n), "{n} tokens under 1% of the window");
}

/// "A small picture is enlarged when the sheet says so, and not otherwise."
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn object_fit_contain_enlarges_a_small_picture() {
    let enc = encoder();
    let pad = enc.token_id("<|image_pad|>").unwrap();
    let count = |sheet: &str| {
        let (ids, ..) = first_call(&app(&enc, sheet, &picture(320, 320)), &enc);
        ids.iter().filter(|&&t| t == pad).count()
    };
    assert_eq!(count("img { max-tokens: 400tok; }"), 100);
    assert_eq!(count("img { max-tokens: 400tok; object-fit: contain; }"), 400);
    assert_eq!(count("img { min-tokens: 400tok; }"), 400);
}

/// "A picture that cannot fit its box shows its alt text, and its file is not sent."
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_picture_that_cannot_fit_shows_its_alt_text() {
    let enc = encoder();
    let pad = enc.token_id("<|image_pad|>").unwrap();
    // Qwen3.5's processor takes no picture under 65,536 pixels, which is 64 tokens.
    let (ids, files, media) = first_call(&app(&enc, "img { max-tokens: 16tok; }", &picture(1280, 720)), &enc);
    assert_eq!(ids.iter().filter(|&&t| t == pad).count(), 0);
    assert!(files.is_empty() && media.is_empty());
    let text = enc.tok.decode(&ids, false).unwrap();
    assert!(text.contains("Look: [a picture that did not fit]\n"), "{text}");
}

/// "SGLang is sent one placeholder for a picture, with the picture at its laid-out size." The
/// real SGLang binding, against a real HTTP server that returns what it was sent.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn sglang_is_sent_one_placeholder_and_the_resized_picture() {
    use affordance::runtime::{
        cancel::CancelToken,
        model::{GenRequest, Model, Sglang},
    };
    use std::io::{BufRead, BufReader, Read, Write};
    let enc = encoder();
    let pad = enc.token_id("<|image_pad|>").unwrap();
    let (ids, files, media) = first_call(&app(&enc, "img { max-tokens: 100tok; }", &picture(1280, 720)), &enc);
    let laid_out = media[0].1;
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
    let diagnostics = affordance::runtime::model::Diagnostics::default();
    let req = GenRequest {
        images: files.clone(),
        media: media.clone(),
        diagnostics: diagnostics.clone(),
        ..GenRequest::new("a", ids.clone(), 8, vec![1], None, CancelToken::new())
    };
    model.generate(&req).unwrap();
    // This server says it processed 3 prompt tokens, which is not what was laid out: a call
    // whose count differs is counted, for the run's summary (`token_mismatches`).
    assert_eq!(diagnostics.token_mismatches(), 1);
    let body = peer.join().unwrap();
    let sent: Vec<u64> = body["input_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_u64().unwrap())
        .collect();
    assert_eq!(sent.iter().filter(|&&t| t == pad as u64).count(), 1);
    assert_eq!(sent.len(), ids.len() - laid_out + 1);
    // Everything else is the prompt, in order.
    let mut expected: Vec<u64> = ids[..media[0].0 + 1].iter().map(|&t| t as u64).collect();
    expected.extend(ids[media[0].0 + laid_out..].iter().map(|&t| t as u64));
    assert_eq!(sent, expected);
    assert_eq!(body["image_data"], json!(files));
}

/// "A picture a tool returns is laid out at its real size on a model bound by name." Qwen3.5-4B
/// has metrics in the registry; an 8 by 8 picture is brought up to its processor's least, 256
/// by 256, which is 64 tokens (it was counted as 1).
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_tool_results_picture_is_counted_at_its_real_size_on_a_model_bound_by_name() {
    use affordance::emit::RunCache;
    use affordance::value::Value;
    let enc = encoder();
    let view = View::load_model(
        &root().join("tests/fixtures/any_model_agent.tsx"),
        "Main",
        &enc,
        "Qwen/Qwen3.5-4B",
        &json!({}),
    )
    .unwrap();
    let small = picture(8, 8);
    let log = json!([
        {"kind": "input", "content": "Look."},
        {"kind": "model", "content": "", "calls": [{"id": "c1", "type": "function", "function": {"name": "shell", "arguments": {"command": "look"}}}]},
        {"kind": "action", "name": "shell", "tool_call_id": "c1", "result": [
            {"type": "text", "text": "One picture."},
            {"type": "image_url", "image_url": {"url": small}},
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
    let pad = enc.token_id("<|image_pad|>").unwrap();
    assert_eq!(prompt.ids.iter().filter(|&&t| t == pad).count(), 64);
    let span = prompt.spans.iter().find(|s| s.meta.get("img").is_some()).unwrap();
    assert_eq!(span.end - span.start, 64);
    assert_eq!(span.meta["size"], json!([256, 256]));
    assert_eq!(span.meta["natural"], json!([8, 8]));
}

/// "A video is shown as its frames, and the sheet sizes each frame." Eight shots half a
/// second apart, four of them shown (the first, the last and two between), each a picture of
/// at most 64 tokens on Qwen3.5-4B, with the time it was taken written before it.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_video_is_shown_as_frames_each_sized_by_the_sheet() {
    use affordance::emit::RunCache;
    use affordance::value::Value;
    let enc = encoder();
    let view = View::load_model(
        &root().join("tests/fixtures/video_frames_agent.tsx"),
        "Main",
        &enc,
        "Qwen/Qwen3.5-4B",
        &json!({}),
    )
    .unwrap();
    let shots: Vec<String> = (0..8).map(|_| picture(640, 360)).collect();
    let Value::Map(state) = Value::from_json(&json!({"shots": shots})) else {
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
    let pictures: Vec<_> = prompt.spans.iter().filter(|s| s.meta.get("img").is_some()).collect();
    assert_eq!(pictures.len(), 4);
    for p in &pictures {
        let n = p.end - p.start;
        assert!((1..=64).contains(&n), "a frame of {n} tokens");
        assert_eq!(p.meta["tok_size"], json!(n));
    }
    let text = enc.tok.decode(&prompt.ids, false).unwrap();
    for at in ["[0.0 s]", "[1.0 s]", "[2.5 s]", "[3.5 s]"] {
        assert!(text.contains(at), "{at} in {text}");
    }
    assert!(!text.contains("[0.5 s]"));
}

/// "A sound clip takes a token for each 40 ms on Gemma 4, and a rule trims it." One second is
/// 25 tokens and 2 around the clip. Under `audio { max-tokens: 12tok; }` the first 400 ms are
/// kept: 10 tokens and the 2.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_sound_clip_is_counted_by_its_length_and_a_rule_trims_it() {
    use affordance::emit::RunCache;
    use affordance::value::Value;
    let enc = Encoder::from_file(
        root()
            .join("ref/tokenizers/google__gemma-4-E2B-it.json")
            .to_str()
            .unwrap(),
    )
    .unwrap();
    let load = |fixture: &str| {
        View::load_model(&root().join(fixture), "Main", &enc, "google/gemma-4-E2B-it", &json!({})).unwrap()
    };
    let (plain, trimmed) = (
        load("tests/fixtures/sound_agent.tsx"),
        load("tests/fixtures/sound_trimmed_agent.tsx"),
    );
    // One second of silence: 16 kHz, 16-bit, one channel.
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
    // (Base64 by way of the library: a PNG data URL is not what is wanted here.)
    let clip = {
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
    let now = chrono::NaiveDate::from_ymd_opt(2026, 10, 5)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let laid_out = |view: &View| {
        let Value::Map(state) = Value::from_json(&json!({"clip": clip})) else {
            unreachable!()
        };
        let prompt = view
            .render(
                &enc,
                std::rc::Rc::new(state.into_data()),
                &mut RunCache::new(false),
                now,
            )
            .unwrap();
        let span = prompt
            .spans
            .iter()
            .find(|s| s.meta["media"] == "audio")
            .unwrap()
            .clone();
        (span.end - span.start, span.meta["clip"]["ms"].as_u64())
    };
    assert_eq!(laid_out(&plain), (27, None));
    assert_eq!(laid_out(&trimmed), (12, Some(400)));
}

/// "The newest picture gets more room than the ones before it." Four pictures under
/// `img { max-tokens: 64tok; } img[last] { max-tokens: 220tok; }`: three of 64 tokens, and the
/// last at its natural 220 (640 by 352 on a 32-pixel grid).
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn the_newest_picture_gets_more_room_than_the_ones_before_it() {
    use affordance::emit::RunCache;
    use affordance::value::Value;
    let enc = encoder();
    let view = View::load_model(
        &root().join("tests/fixtures/video_frames_last_agent.tsx"),
        "Main",
        &enc,
        "Qwen/Qwen3.5-4B",
        &json!({}),
    )
    .unwrap();
    let shots: Vec<String> = (0..4).map(|_| picture(640, 352)).collect();
    let Value::Map(state) = Value::from_json(&json!({"shots": shots})) else {
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
    let mut pictures: Vec<_> = prompt.spans.iter().filter(|s| s.meta.get("img").is_some()).collect();
    pictures.sort_by_key(|s| s.start);
    let sizes: Vec<usize> = pictures.iter().map(|s| s.end - s.start).collect();
    assert_eq!(sizes, [64, 64, 64, 220]);
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_picture_under_one_familys_own_rule_takes_its_tokens_and_markers() {
    // A rule from src/media_kinds (MiniMax M3's: the grid's cells between a start and an end
    // token), here with Qwen's tokens: a 640 by 480 picture is 20 by 15 cells of 32 pixels.
    let enc = encoder();
    let rule = r#"{kind: "marked_grid", cell: 32, min: 65536, max: 16777216, start: "<|vision_start|>", end: "<|vision_end|>"}"#;
    let view = app_with(&enc, "", &picture(640, 480), rule);
    let (ids, images, _) = first_call(&view, &enc);
    let (start, pad, end) = (
        enc.token_id("<|vision_start|>").unwrap(),
        enc.token_id("<|image_pad|>").unwrap(),
        enc.token_id("<|vision_end|>").unwrap(),
    );
    let at = ids.iter().position(|&t| t == pad).unwrap();
    assert_eq!(ids[at - 1], start);
    assert_eq!(ids[at..at + 300], vec![pad; 300][..]);
    assert_eq!(ids[at + 300], end);
    assert_eq!(images.len(), 1);
}

/// The prompt a model bound by its registered name gives a tool result of these pictures.
fn bound_prompt(model: &str, pictures: &[String]) -> (Encoder, Vec<u32>) {
    bound_prompt_in(model, pictures, true)
}

/// The same, with the pictures in a tool result or (for a format with no tool turns) in the
/// user's message.
fn bound_prompt_in(model: &str, pictures: &[String], in_tool_result: bool) -> (Encoder, Vec<u32>) {
    use affordance::emit::RunCache;
    use affordance::value::Value;
    let enc = Encoder::from_file(
        root()
            .join(format!("ref/tokenizers/{}.json", model.replace('/', "__")))
            .to_str()
            .unwrap(),
    )
    .unwrap();
    let view = View::load_model(
        &root().join("tests/fixtures/any_model_agent.tsx"),
        "Main",
        &enc,
        model,
        &json!({}),
    )
    .unwrap();
    let mut result = vec![json!({"type": "text", "text": "Pictures."})];
    result.extend(
        pictures
            .iter()
            .map(|p| json!({"type": "image_url", "image_url": {"url": p}})),
    );
    let log = if in_tool_result {
        json!([
            {"kind": "input", "content": "Look."},
            {"kind": "model", "content": "", "calls": [{"id": "c1", "type": "function", "function": {"name": "shell", "arguments": {"command": "look"}}}]},
            {"kind": "action", "name": "shell", "tool_call_id": "c1", "result": result},
        ])
    } else {
        json!([{"kind": "input", "content": result}])
    };
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
    (enc, prompt.ids.to_vec())
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn minicpm_numbers_each_picture_by_its_place_in_the_prompt() {
    // MiniCPM-V's processor writes <image_id>k</image_id> before the k-th picture, from 0.
    let (enc, ids) = bound_prompt("openbmb/MiniCPM-V-4.6", &[picture(224, 224), picture(224, 224)]);
    let text = enc.tok.decode(&ids, false).unwrap();
    assert!(text.contains("<image_id>0</image_id><image>"), "{text}");
    assert!(text.contains("<image_id>1</image_id><image>"), "{text}");
    let pad = enc.token_id("<|image_pad|>").unwrap();
    // 224 by 224 is one source block of 64 (the processor's count is 69 with its markers).
    assert_eq!(ids.iter().filter(|&&t| t == pad).count(), 2 * 64);
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn ling_vl_has_its_processors_newline_after_a_pictures_end_marker() {
    let (enc, ids) = bound_prompt("inclusionAI/Ling-3.0-flash-VL", &[picture(640, 480)]);
    let (pad, end) = (
        enc.token_id("<|image_pad|>").unwrap(),
        enc.token_id("<|vision_end|>").unwrap(),
    );
    // 640 by 480 is 20 by 15 cells of 32 pixels.
    assert_eq!(ids.iter().filter(|&&t| t == pad).count(), 300);
    let at = ids.iter().position(|&t| t == end).unwrap();
    assert_eq!(ids[at - 1], pad);
    assert!(enc.tok.decode(&ids[at + 1..at + 2], false).unwrap().starts_with('\n'));
}

/// "Every model with a picture rule shows a picture." For each registered model whose row has a
/// picture rule, a small and a large picture in a tool result are laid out as pictures: never the
/// alt text, never a render error. (A rule missing its markers, or a marker that is not a token of
/// the model, once turned every picture of a family into "[picture not shown]" without a word.)
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn every_model_with_a_picture_rule_shows_a_picture() {
    let index: J = serde_json::from_str(&std::fs::read_to_string(root().join("views/index.json")).unwrap()).unwrap();
    let models: Vec<String> = index
        .as_object()
        .unwrap()
        .iter()
        .filter(|(k, e)| !k.starts_with('_') && e["agent"]["media_metrics"]["image"].is_object())
        .filter(|(k, _)| root().join(format!("ref/tokenizers/{k}.json")).exists())
        .map(|(k, _)| k.clone())
        .collect();
    assert!(models.len() > 20, "{} models with a picture rule", models.len());
    let pictures = [picture(224, 224), picture(1600, 1200)];
    let results = crate::each::in_parallel(&models, |model| {
        let name = model.replacen("__", "/", 1);
        let render = |in_tool_result| {
            std::panic::catch_unwind(|| {
                let (enc, ids) = bound_prompt_in(&name, &pictures, in_tool_result);
                enc.tok.decode(&ids, false).unwrap()
            })
        };
        // A format whose publisher refuses tool turns is shown the pictures in the user's message.
        let shown = render(true).or_else(|_| render(false));
        match shown {
            Err(e) => Some(format!(
                "{model}: the render failed: {}",
                e.downcast_ref::<String>()
                    .map(String::as_str)
                    .or(e.downcast_ref::<&str>().copied())
                    .unwrap_or("?")
                    .lines()
                    .next()
                    .unwrap_or("")
            )),
            Ok(text) if text.contains("[picture not shown]") => Some(format!("{model}: a picture was not shown")),
            Ok(_) => None,
        }
    });
    // Formats that require user and assistant turns to alternate: the app's latest user message and
    // its screen are two turns, which the app joins only when both are text, so a picture in either
    // cannot be given to these formats yet.
    const ALTERNATING: [&str; 9] = [
        "google__gemma-3-4b-it",
        "google__gemma-3-12b-it",
        "google__gemma-3-27b-it",
        "google__gemma-3n-E2B-it",
        "google__gemma-3n-E4B-it",
        "mlabonne__gemma-3-27b-it-abliterated",
        "meta-llama__Llama-Guard-4-12B",
        "allenai__Molmo2-8B",
        "mistral-experimental__pixtral-12b",
    ];
    let failed: Vec<String> = results.into_iter().flatten().collect();
    let unexpected: Vec<&String> = failed
        .iter()
        .filter(|f| !ALTERNATING.iter().any(|m| f.starts_with(&format!("{m}:"))))
        .collect();
    assert!(
        unexpected.is_empty(),
        "{} of {}:\n{}",
        unexpected.len(),
        models.len(),
        unexpected.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n")
    );
    // A listed format that shows its pictures now no longer belongs in the list.
    for m in ALTERNATING.iter().filter(|m| models.iter().any(|x| x == *m)) {
        assert!(
            failed.iter().any(|f| f.starts_with(&format!("{m}:"))),
            "{m} shows its pictures now: remove it from ALTERNATING"
        );
    }
}
