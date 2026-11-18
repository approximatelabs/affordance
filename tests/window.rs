//! Every prompt has a window: the model binding's
//! `context` less the reply's room. A sheet caps the history in shares of it
//! (`history { max-tokens: 50win; overflow: drop-start; drop-step: 25% }`), whole turns drop in
//! steps, so a chat that runs for hundreds of turns never sends a prompt over the window and its
//! cut moves rarely. Without a cap, a prompt over the context is never sent: the call fails
//! with a ContextError naming the boxes.
use affordance::View;
use affordance::emit::Encoder;
use affordance::runtime::{Agent, AgentConfig, Command, Input};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
const T0: i64 = 1_791_000_000_000;
const CONTEXT: usize = 2000;
const MAX_NEW: usize = 256;

fn chat(enc: &Encoder, name: &str, tss: &str) -> View {
    let dir = std::env::temp_dir().join(format!("aff-window-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("s.tss"), tss).unwrap();
    std::fs::write(
        dir.join("app.tsx"),
        format!(
            r#"import "./s.tss";
import {{ TurnStrategy }} from "{root}/views/strategies/turn_strategy.tsx";
import {{ useConversation }} from "{root}/views/lib/loop.tsx";
import {{ useLog }} from "{root}/views/lib/log.tsx";
import {{ GLM53App }} from "{root}/views/template_apps/glm53.tsx";
export default function Main() {{
  const log = useLog();
  useConversation(log);
  const [layout, setLayout] = useState(null, "layout");
  useEvent("layout", facts => setLayout(facts));
  const app = <aff event="message" onCallback={{({{ text }}) => log.append({{kind: "input", content: text}})}}/>;
  return <GLM53App><TurnStrategy strategy="unrolled" log={{log.records}}>{{app}}</TurnStrategy></GLM53App>;
}}"#,
            root = root().display()
        ),
    )
    .unwrap();
    View::load(&dir.join("app.tsx"), "Main", enc).unwrap()
}

/// Run a chat of `turns` exchanges; returns every prompt sent and the agent's last error, if any.
fn run(tss: &str, name: &str, turns: usize) -> (Vec<Vec<u32>>, Encoder, Agent) {
    let enc = Encoder::from_file(root().join("ref/tokenizers/zai-org__GLM-5.3.json").to_str().unwrap()).unwrap();
    let view = chat(&enc, name, tss);
    let mut agent = Agent::new(AgentConfig {
        context: Some(CONTEXT),
        retry: Default::default(),
        template: None,
        id: "a".into(),
        props: json!({}),
        env: BTreeMap::new(),
        max_new: MAX_NEW,
        max_turns: None,
        max_prompt: None,
        start_ms: T0,
        seed: 7,
        machines: vec![],
        model: Some("default".into()),
    })
    .unwrap();
    agent.start(&view, &enc).unwrap();
    let mut clock = T0;
    let mut prompts = vec![];
    for i in 0..turns {
        clock += 10;
        let text = format!("Message {i}: tell me one more thing about the history of the project and its plans.");
        let cmds = agent
            .handle(
                &view,
                &enc,
                Input::Event {
                    name: "message".into(),
                    payload: json!({"text": text}),
                    op: None,
                    clock_ms: clock,
                },
            )
            .unwrap();
        let Some((task, ids)) = cmds.iter().find_map(|c| match c {
            Command::Generate { task, ids, .. } => Some((*task, ids.to_vec())),
            _ => None,
        }) else {
            break;
        };
        prompts.push(ids);
        let mut reply = enc
            .tok
            .encode(
                format!("Answer {i}: the project started small and grew by one feature each week.").as_str(),
                false,
            )
            .unwrap()
            .get_ids()
            .to_vec();
        reply.push(enc.token_id("<|user|>").unwrap());
        clock += 10;
        agent
            .handle(
                &view,
                &enc,
                Input::Reply {
                    server: None,
                    task,
                    ids: reply,
                    cut_off: false,
                    clock_ms: clock,
                    prompt: None,
                    aborted: false,
                },
            )
            .unwrap();
    }
    (prompts, enc, agent)
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn a_capped_history_keeps_every_prompt_in_the_window_and_cuts_rarely() {
    let (prompts, enc, _) = run(
        "history { max-tokens: 50win; overflow: drop-start; drop-step: 25%; }",
        "capped",
        150,
    );
    assert_eq!(prompts.len(), 150, "every message was answered");
    let window = CONTEXT - MAX_NEW;
    assert!(
        prompts.iter().all(|p| p.len() <= window),
        "largest prompt {}",
        prompts.iter().map(Vec::len).max().unwrap()
    );
    // The history did overflow (the chat is far longer than the cap): early turns are gone,
    // the latest are there...
    let last = enc.tok.decode(prompts.last().unwrap(), false).unwrap();
    assert!(
        !last.contains("Message 0:") && last.contains("Message 149:") && last.contains("Answer 148:"),
        "{last}"
    );
    // ...and the cut moved in steps: the first kept user turn changes rarely, not every turn.
    let first_turn = |p: &Vec<u32>| {
        let text = enc.tok.decode(p, false).unwrap();
        text.split("<|user|>")
            .nth(1)
            .map(|t| t.chars().take(12).collect::<String>())
            .unwrap_or_default()
    };
    let cuts = prompts
        .windows(2)
        .filter(|w| first_turn(&w[0]) != first_turn(&w[1]))
        .count();
    assert!(
        cuts > 0 && cuts < prompts.len() / 4,
        "{cuts} cut moves over {} prompts",
        prompts.len()
    );
}

#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn without_a_cap_a_prompt_over_the_context_is_never_sent() {
    let (prompts, _, agent) = run("", "uncapped", 150);
    assert!(prompts.len() < 150, "the chat outgrew the context");
    assert!(prompts.iter().all(|p| p.len() + MAX_NEW <= CONTEXT));
    // The call fails with a ContextError (e.name), which the conversation loop records as it stops.
    let stopped = agent.stopped().cloned().unwrap_or_default();
    assert_eq!(stopped["reason"], "conversation_error", "{stopped}");
    assert!(
        stopped["error"]
            .as_str()
            .is_some_and(|e| e.contains("do not fit the model's context (2000)")),
        "{stopped}"
    );
}

/// After each prompt is laid out, `useEvent("layout")` hears what its boxes did (as
/// ResizeObserver reports sizes): an app schedules a fold from it, never by measuring.
#[test]
#[cfg_attr(not(feature = "reference-tests"), ignore = "needs ref/ data: docs/DEVELOPING.md")]
fn the_layout_event_reports_what_the_boxes_did() {
    let (prompts, _, agent) = run(
        "history { max-tokens: 50win; overflow: drop-start; drop-step: 25%; }",
        "events",
        80,
    );
    let facts = &agent.state()["__ui"]["@layout"];
    assert_eq!(facts["window"], json!(CONTEXT - MAX_NEW), "{facts}");
    assert_eq!(facts["prompt_tokens"], json!(prompts.last().unwrap().len()), "{facts}");
    let history = facts["boxes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["label"] == "history")
        .expect("the history box");
    assert!(
        history["dropped_items"].as_u64().unwrap() > 0
            && history["tokens"].as_u64().unwrap() <= ((CONTEXT - MAX_NEW) / 2) as u64,
        "{history}"
    );
}
