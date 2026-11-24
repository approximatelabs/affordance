//! `memoScan` (src/memo.rs): a fold whose per-item results an agent keeps between renders. Its
//! result is the plain fold's; across renders of a growing list only the new items are computed;
//! other deps start over; verify mode catches a step
//! that reads outside its arguments.
use crate::tokenizer;
use affordance::View;
use affordance::emit::{Encoder, RunCache};
use affordance::value::Value;
use serde_json::json;
use std::rc::Rc;

fn enc() -> Encoder {
    tokenizer::encoder()
}
fn now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 10, 17)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
}

/// A view whose render is the scan of `harness.items` (shared values, as an agent's transcript).
const SRC: &str = r#"
function step(state, item, i) { state = state ?? {n: 0, seen: 0}; return [{n: state.n + item.v, seen: i + 1}, `${i}:${item.v}|`]; }
export default function Main({ k }) {
  const r = memoScan(harness.items, null, step, k);
  return <text>{r.outs.join("") + `=${r.state.n}/${r.state.seen}`}</text>;
}
"#;

fn view(enc: &Encoder, name: &str, src: &str) -> View {
    let dir = std::env::temp_dir().join(format!("aff-memo-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.tsx"));
    std::fs::write(&path, src).unwrap();
    View::load(&path, "Main", enc).unwrap()
}

/// Render with `items` as `harness.items` (the same values across renders, as a transcript).
fn render(v: &View, enc: &Encoder, cache: &mut RunCache, items: &[Value], k: i64) -> Result<String, String> {
    let mut h = affordance::value::Map::default();
    h.insert(Rc::from("items"), Value::list(items.to_vec()));
    let state = affordance::value::state(&json!({"k": k}));
    let (r, _) = v.render_prepared(enc, state, cache, now(), Value::map(h), None)?;
    Ok(enc.tok.decode(&r.ids, false).unwrap())
}

fn item(v: i64) -> Value {
    Value::from_json(&json!({"v": v}))
}

#[test]
fn memo_scan_reuses_the_kept_prefix_and_equals_the_plain_fold() {
    let enc = enc();
    let v = view(&enc, "scan", SRC);
    let mut items: Vec<Value> = (1..=3).map(item).collect();
    // No memo (a plain render): the fold itself.
    assert_eq!(
        render(&v, &enc, &mut RunCache::new(false), &items, 0).unwrap(),
        "0:1|1:2|2:3|=6/3"
    );
    let mut cache = RunCache::new(true);
    assert_eq!(render(&v, &enc, &mut cache, &items, 0).unwrap(), "0:1|1:2|2:3|=6/3");
    // Two more items: only those are computed.
    items.extend([item(10), item(20)]);
    assert_eq!(
        render(&v, &enc, &mut cache, &items, 0).unwrap(),
        "0:1|1:2|2:3|3:10|4:20|=36/5"
    );
    let (reused, computed) = {
        let m = cache.memo.borrow();
        (m.reused, m.computed)
    };
    assert_eq!((reused, computed), (3, 3 + 2));
    // A changed item (a new value at index 1) recomputes from there.
    items[1] = item(5);
    assert_eq!(
        render(&v, &enc, &mut cache, &items, 0).unwrap(),
        "0:1|1:5|2:3|3:10|4:20|=39/5"
    );
    assert_eq!(cache.memo.borrow().reused, 3 + 1);
    // Other deps: from scratch.
    let computed = cache.memo.borrow().computed;
    assert_eq!(
        render(&v, &enc, &mut cache, &items, 1).unwrap(),
        "0:1|1:5|2:3|3:10|4:20|=39/5"
    );
    assert_eq!(cache.memo.borrow().computed, computed + 5);
}

#[test]
fn memo_scan_verify_catches_a_step_that_reads_outside_its_arguments() {
    let enc = enc();
    // The step reads `k` from the component, not from deps: a stale result once k changes.
    let bad = r#"
export default function Main({ k }) {
  const r = memoScan(harness.items, 0, (s, item, i) => [s + item.v * k, String(s)], null);
  return <text>{String(r.state)}</text>;
}
"#;
    let v = view(&enc, "bad", bad);
    let items: Vec<Value> = (1..=3).map(item).collect();
    let mut cache = RunCache::new(true);
    cache.memo.borrow_mut().verify = true;
    render(&v, &enc, &mut cache, &items, 1).unwrap();
    let e = render(&v, &enc, &mut cache, &items, 2).unwrap_err();
    assert!(e.contains("differs from a fresh one"), "{e}");
}

#[test]
fn function_inputs_use_identity_while_fresh_output_closures_can_verify() {
    let enc = enc();
    for (name, expression) in [
        (
            "function_item",
            "memoScan([f], 0, (s, fn) => [s + fn(10), () => 1], null)",
        ),
        ("function_dep", "memoScan([10], 0, (s, n) => [s + f(n), () => 1], [f])"),
    ] {
        let v = view(
            &enc,
            name,
            &format!(
                "export default function Main({{k}}){{const f=n=>n*k;const r={expression};return String(r.state);}}"
            ),
        );
        let mut cache = RunCache::new(true);
        cache.memo.borrow_mut().verify = true;
        for k in [1, 2, 1] {
            let fresh = render(&v, &enc, &mut RunCache::new(false), &[], k).unwrap();
            assert_eq!(fresh, (k * 10).to_string());
            assert_eq!(render(&v, &enc, &mut cache, &[], k).unwrap(), fresh);
        }
    }
}

#[test]
fn changed_objects_recompute_items_initial_state_and_dependencies() {
    let enc = enc();
    for (name, expression) in [
        (
            "private",
            "memoScan([new Scale(k)], null, (s,x)=>[x.apply(10),null], null).state",
        ),
        (
            "hidden",
            "memoScan([new Error(String(k))], null, (s,x)=>[Number(x.message)*10,null], null).state",
        ),
        (
            "alias",
            "memoScan([{left:x,right:k===1?x:{}}], null, (s,x)=>[x.left===x.right?10:20,null], null).state",
        ),
        (
            "initial",
            "memoScan([0], new Scale(k), (s,x)=>[s.apply(10),null], null).state",
        ),
        (
            "dependency",
            "memoScan([0], null, (s,x)=>[scale.apply(10),null], [scale]).state",
        ),
        (
            "single_dependency",
            "memoScan([0], null, (s,x)=>[scale.apply(10),null], scale).state",
        ),
        (
            "undefined_dependency",
            "memoScan([0], null, (s,x)=>[dep===undefined?10:20,null], dep).state",
        ),
    ] {
        let v = view(
            &enc,
            name,
            &format!(
                r#"
class Scale {{ #k; constructor(k){{this.#k=k;}} apply(n){{return n*this.#k;}} }}
export default function Main({{k}}){{const x={{}};const scale=new Scale(k);const dep=k===1?undefined:null;return String({expression});}}
"#
            ),
        );
        let mut cache = RunCache::new(true);
        cache.memo.borrow_mut().verify = true;
        for k in [1, 2, 1] {
            let fresh = render(&v, &enc, &mut RunCache::new(false), &[], k).unwrap();
            assert_eq!(fresh, (k * 10).to_string());
            assert_eq!(render(&v, &enc, &mut cache, &[], k).unwrap(), fresh, "{name}");
        }
    }
    // Dependency wrappers may be fresh, but stable entries and a scalar seed preserve reuse.
    for (name, dep) in [
        ("dep_entries", ",[3]"),
        ("omitted_dep", ""),
        ("undefined_dep", ",undefined"),
    ] {
        let v = view(
            &enc,
            name,
            &format!(
                "export default function Main(){{return String(memoScan(harness.items,0,(s,x)=>[s+x*3,null]{dep}).state);}}"
            ),
        );
        let mut cache = RunCache::new(true);
        for _ in 0..2 {
            assert_eq!(render(&v, &enc, &mut cache, &[Value::Int(2)], 3).unwrap(), "6");
        }
        assert_eq!((cache.memo.borrow().reused, cache.memo.borrow().computed), (1, 1));
    }
}

#[test]
fn template_adapters_keep_unchanged_system_and_schema_data_stable() {
    let enc = enc();
    let v = view(
        &enc,
        "template_input",
        &format!(
            r#"
import {{prepareTemplateInput}} from '{0}/views/template_apps/input.tsx';
import {{TurnStrategy}} from '{0}/views/strategies/turn_strategy.tsx';
export default function Main({{k}}){{
 const input=prepareTemplateInput(<TurnStrategy strategy="unrolled" log={{[]}}>
  <aff label="act" tool_name={{'tool'+k}} onAction={{()=>{{}}}}/>
 </TurnStrategy>, 'system'+k, true);
 return memoScan([input.tools,input.turns[0]],null,(s,x)=>[null,JSON.stringify(x)],null).outs.join('');
}}
"#,
            env!("CARGO_MANIFEST_DIR")
        ),
    );
    let mut cache = RunCache::new(true);
    for k in [1, 2] {
        let fresh = render(&v, &enc, &mut RunCache::new(false), &[], k).unwrap();
        assert_eq!(render(&v, &enc, &mut cache, &[], k).unwrap(), fresh);
        let computed = cache.memo.borrow().computed;
        assert_eq!(render(&v, &enc, &mut cache, &[], k).unwrap(), fresh);
        assert_eq!(
            cache.memo.borrow().computed,
            computed,
            "unchanged adapter data must preserve downstream reuse"
        );
    }
}

#[test]
fn fragments_preserve_complete_spans_and_verification_observes_changed_metadata() {
    use affordance::emit::Span;
    let enc = enc();
    let v = view(
        &enc,
        "fragment_input",
        "export default function Main(){return memoScan(harness.items,null,(s,x)=>[null,x],null).outs;}",
    );
    let check = view(
        &enc,
        "fragment_verify",
        "export default function Main(){return memoScan([0],null,(s,x)=>[null,harness.fragment],null).outs;}",
    );
    let ids: Rc<[u32]> = enc.tok.encode("word", false).unwrap().get_ids().to_vec().into();
    let span = Span {
        label: "first".into(),
        kind: "label".into(),
        start: 0,
        end: ids.len(),
        straddle: false,
        meta: json!({"value":1}),
    };
    let fragment = |span: Span| Value::tokens(ids.clone(), Rc::from([span]));
    let render_fragment = |view: &View, cache: &mut RunCache, value: Value| {
        let mut h = affordance::value::Map::default();
        h.insert(Rc::from("items"), Value::list(vec![value.clone()]));
        h.insert(Rc::from("fragment"), value);
        view.render_prepared(&enc, Default::default(), cache, now(), Value::map(h), None)
            .map(|(r, _)| r.spans)
    };
    for field in ["label", "range", "metadata", "straddle"] {
        let mut changed = span.clone();
        match field {
            "label" => changed.label = "second".into(),
            "range" => changed.end -= 1,
            "metadata" => changed.meta = json!({"value":2}),
            _ => changed.straddle = true,
        }
        let mut cache = RunCache::new(true);
        render_fragment(&v, &mut cache, fragment(span.clone())).unwrap();
        let fresh = render_fragment(&v, &mut RunCache::new(false), fragment(changed.clone())).unwrap();
        assert_eq!(
            render_fragment(&v, &mut cache, fragment(changed.clone())).unwrap(),
            fresh,
            "{field}"
        );
        let mut cache = RunCache::new(true);
        cache.memo.borrow_mut().verify = true;
        render_fragment(&check, &mut cache, fragment(span.clone())).unwrap();
        assert!(
            render_fragment(&check, &mut cache, fragment(changed))
                .unwrap_err()
                .contains("differs from a fresh one"),
            "{field}"
        );
    }
}

/// A render that fails partway through a scan keeps what it computed: the next render resumes
/// there, not from index 0. Here a scan grows one costly item per render; recomputed from scratch,
/// it would exceed a render's fuel, so losing the cache to one failed render (a step that throws
/// on an item a later render no longer has) would fail every render after it.
#[test]
fn a_failed_render_keeps_the_scan_prefix_it_computed() {
    let enc = enc();
    let v = view(
        &enc,
        "costly",
        r#"
function step(state, item, i) {
  if (item.bad) throw new Error("bad item");
  let n = 0;
  for (let j = 0; j < 200000; j++) n++;
  return [(state ?? 0) + n, i];
}
export default function Main() { return <text>{String(memoScan(harness.items, null, step, null).state)}</text>; }
"#,
    );
    let mut cache = RunCache::new(true);
    // Each loop iteration is several VM steps: six items exceed a render's fuel (`FUEL`, 10M
    // steps), one item per render does not.
    let mut items = vec![];
    for n in 1..=5 {
        items.push(item(n));
        assert_eq!(
            render(&v, &enc, &mut cache, &items, 0)
                .map_err(|e| format!("{n}: {e}"))
                .unwrap(),
            format!("{}", n * 200000)
        );
    }
    let bad = Value::from_json(&json!({"bad": true}));
    let failed = [items.clone(), vec![bad]].concat();
    assert!(
        render(&v, &enc, &mut cache, &failed, 0)
            .unwrap_err()
            .contains("bad item")
    );
    let fresh = [items.clone(), vec![item(6)]].concat();
    assert!(
        render(&v, &enc, &mut RunCache::new(true), &fresh, 0)
            .unwrap_err()
            .contains("out of fuel"),
        "a whole recompute does not fit"
    );
    let reused = cache.memo.borrow().reused;
    assert_eq!(render(&v, &enc, &mut cache, &fresh, 0).unwrap(), "1200000");
    assert_eq!(
        cache.memo.borrow().reused - reused,
        5,
        "the render after the failure restored at its prefix"
    );
}
