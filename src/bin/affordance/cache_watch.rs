//! A console warning when a page prompt throws away most of the server's prefix cache, as a
//! browser's developer console reports a forced reflow: it watches the run's trace records and
//! changes nothing in the run.
//!
//! A model server keeps the work it did for the start of the previous prompt (and its reply), and
//! reuses it when the next prompt starts the same way. The agent's `render` record says where the
//! new prompt first differs from the previous one (`append.rewrite_at`) and how long that previous
//! prompt was (`append.of`). A small shared start on a long prompt means the server computes it
//! again: usually a value that changes on every turn near the top of the prompt (a clock, a
//! counter), or a history that drops just enough to fit on each turn.
//!
//! The server's own count, `cached_tokens` on the `generate` record, is checked against that
//! prediction too, more softly: when it reuses much less than Affordance expected, the cause is
//! usually the server (eviction, routing), not the app.
use crate::flags::CacheWarn;
use affordance::runtime::{Observed, Observer};
use serde_json::Value as J;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Instance {
    /// Busts so far, and the count at which the next is printed (1, 2, 4, ...).
    busts: u64,
    next_bust: u64,
    /// The same for the server's own count.
    misses: u64,
    next_miss: u64,
    /// The prefix each model call was expected to reuse, by task, until its `generate` record.
    expected: HashMap<u64, usize>,
    /// The reply that wrote a tool call as text was reported.
    told_text_call: bool,
}

/// Lines printed for single agents before the rest of a run is only summed up, so a fleet of a
/// thousand agents that all bust the cache prints a few lines, not a thousand.
const LINES: u64 = 12;

#[derive(Default)]
struct Run {
    agents: HashMap<String, Instance>,
    /// Model call problems reported so far (`problem`).
    problems: u64,
    /// Lines printed, and warnings since the last summary line once past `LINES`.
    printed: u64,
    held: u64,
    next_summary: u64,
}

/// What a person at the terminal must be told about a model call, as a browser's console reports
/// a failed request: the call failed and what happens next, or the reply wrote a tool call as
/// text. A failing call is reported at its 1st, 2nd, 4th, 8th.. failure.
fn problem(id: &str, a: &mut Instance, rec: &J) -> Option<String> {
    match rec["t"].as_str()? {
        "model_error" | "render_error" => {
            let failures = rec["failures"].as_u64()?;
            let error = rec["error"].as_str().unwrap_or("");
            match rec["retry_in_ms"].as_u64() {
                Some(_) if !failures.is_power_of_two() => None,
                Some(ms) => Some(format!(
                    "affordance: {id}: the model call failed ({error}). Trying again in {} s{}{}.",
                    ms.div_ceil(1000),
                    if failures > 1 {
                        format!(", after {failures} failures")
                    } else {
                        String::new()
                    },
                    " (the binding's `retry` sets how long and which failures)",
                )),
                None => Some(format!(
                    "affordance: {id}: the model call failed ({error}) and is not tried again: the app gets the error."
                )),
            }
        }
        "reply" if rec["text_call"] == true && !a.told_text_call => {
            a.told_text_call = true;
            Some(format!(
                "affordance: warning: {id}: the model wrote a tool call as text, so no tool ran. The server did not parse it: start it with its tool-call parser (vLLM: --enable-auto-tool-choice --tool-call-parser NAME; SGLang: --tool-call-parser NAME; llama.cpp: --jinja)."
            ))
        }
        _ => None,
    }
}

/// An observer that prints a run's model problems, and (with `warn`) its cache warnings, to stderr.
pub fn observer(warn: Option<CacheWarn>) -> Observer {
    let run: Mutex<Run> = Mutex::new(Run::default());
    Observer(Arc::new(move |id: &str, _task: u64, seen: Observed<'_>| {
        let Observed::Trace(rec) = seen else { return };
        let mut run = run.lock().unwrap();
        let a = run.agents.entry(id.to_string()).or_default();
        if let Some(line) = problem(id, a, rec) {
            // A fleet whose server is down would print a line an agent: after LINES of them the
            // rest are counted, and the count is printed as it doubles.
            run.problems += 1;
            if run.problems <= LINES {
                eprintln!("{line}");
            } else if run.problems.is_power_of_two() {
                eprintln!(
                    "affordance: {} model call problems so far (the latest: {line})",
                    run.problems
                );
            }
            return;
        }
        let Some(warn) = &warn else { return };
        let Some(line) = check(warn, id, a, rec) else { return };
        if run.printed < LINES {
            run.printed += 1;
            eprintln!("{line}");
            if run.printed == LINES {
                eprintln!("affordance: further cache warnings are summed up");
            }
            return;
        }
        run.held += 1;
        if run.held >= run.next_summary.max(16) {
            eprintln!(
                "affordance: {} more cache warnings (--flag cache-warn=off to silence)",
                run.held
            );
            run.next_summary = run.held * 2;
            run.held = 0;
        }
    }))
}

/// One trace record: a line to print, if it calls for one.
fn check(warn: &CacheWarn, id: &str, a: &mut Instance, rec: &J) -> Option<String> {
    let task = rec["task"].as_u64();
    match rec["t"].as_str()? {
        "render" => {
            let shared = rec["shared"].as_u64()? as usize;
            let of = rec["append"]["of"].as_u64().map_or(usize::MAX, |n| n as usize);
            if let Some(task) = task {
                a.expected.insert(task, shared);
            }
            if of == usize::MAX || of < warn.min_tokens || (shared as f64) >= warn.below * of as f64 {
                return None;
            }
            a.busts += 1;
            if a.busts < a.next_bust {
                return None;
            }
            a.next_bust = a.busts * 2;
            let turn = rec["turn"].as_u64().unwrap_or(0);
            Some(format!(
                "affordance: warning: {id} turn {turn}: the prompt changed at token {shared} of {of}, so the server recomputes {lost} cached tokens{times}. Is something near the start changing every turn? A history can drop in steps (drop-step).",
                lost = of - shared,
                times = repeat(a.busts),
            ))
        }
        "generate" => {
            let expected = a.expected.remove(&task?)?;
            let cached = rec["cached_tokens"].as_u64()? as usize;
            if expected < warn.min_tokens || (cached as f64) >= warn.below * expected as f64 {
                return None;
            }
            a.misses += 1;
            if a.misses < a.next_miss {
                return None;
            }
            a.next_miss = a.misses * 2;
            Some(format!(
                "affordance: note: {id}: the server reused {cached} cached tokens where the prompt allowed {expected}{times}. The cause is usually the server (an evicted cache, another replica), not the app.",
                times = repeat(a.misses),
            ))
        }
        _ => None,
    }
}

fn repeat(n: u64) -> String {
    if n == 1 {
        String::new()
    } else {
        format!(" ({n} times so far)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const W: CacheWarn = CacheWarn {
        below: 0.2,
        min_tokens: 1000,
    };

    #[test]
    fn a_large_early_rewrite_warns_and_repeats_are_rate_limited() {
        let mut a = Instance::default();
        let bust = |turn| json!({"t": "render", "turn": turn, "task": turn, "prompt_tokens": 5000, "shared": 40, "append": {"rewrite_at": 40, "of": 4800}});
        let printed: Vec<bool> = (1..=9).map(|t| check(&W, "e0", &mut a, &bust(t)).is_some()).collect();
        // The 1st, 2nd, 4th and 8th are printed.
        assert_eq!(printed, [true, true, false, true, false, false, false, true, false]);
        let line = check(&W, "e0", &mut Instance::default(), &bust(3)).unwrap();
        assert!(
            line.contains("changed at token 40 of 4800") && line.contains("recomputes 4760"),
            "{line}"
        );
    }

    #[test]
    fn exact_appends_late_rewrites_and_short_prompts_are_quiet() {
        let mut a = Instance::default();
        for rec in [
            json!({"t": "render", "turn": 1, "task": 1, "prompt_tokens": 5000, "shared": 4900, "append": "exact"}),
            json!({"t": "render", "turn": 2, "task": 2, "prompt_tokens": 5000, "shared": 4000, "append": {"rewrite_at": 4000, "of": 4800}}),
            json!({"t": "render", "turn": 3, "task": 3, "prompt_tokens": 500, "shared": 10, "append": {"rewrite_at": 10, "of": 600}}),
            json!({"t": "render", "turn": 4, "task": 4, "prompt_tokens": 500, "append": null}),
        ] {
            assert_eq!(check(&W, "e0", &mut a, &rec), None, "{rec}");
        }
    }

    #[test]
    fn the_server_reusing_much_less_than_expected_is_noted() {
        let mut a = Instance::default();
        check(
            &W,
            "e0",
            &mut a,
            &json!({"t": "render", "turn": 2, "task": 7, "prompt_tokens": 5000, "shared": 5000, "append": "exact"}),
        );
        let line = check(
            &W,
            "e0",
            &mut a,
            &json!({"t": "generate", "task": 7, "cached_tokens": 64}),
        )
        .unwrap();
        assert!(
            line.contains("reused 64 cached tokens where the prompt allowed 5000"),
            "{line}"
        );
        // As expected: quiet. A call it has no prediction for: quiet.
        check(
            &W,
            "e0",
            &mut a,
            &json!({"t": "render", "turn": 3, "task": 8, "prompt_tokens": 5200, "shared": 5100, "append": "exact"}),
        );
        assert_eq!(
            check(
                &W,
                "e0",
                &mut a,
                &json!({"t": "generate", "task": 8, "cached_tokens": 5100})
            ),
            None
        );
        assert_eq!(
            check(
                &W,
                "e0",
                &mut a,
                &json!({"t": "generate", "task": 9, "cached_tokens": 0})
            ),
            None
        );
    }
}
