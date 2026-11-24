//! What an agent has outstanding: the commands to resubmit after a restore, and a summary.
use super::*;

/// What the coordinator's status needs from the outstanding commands, read in place: the count,
/// the phase, the timers, and a model call being retried (its failures so far and the last
/// one's kind; the most failed, if several). It runs after every input, so it copies no command
/// payloads.
pub struct OutstandingSummary {
    /// Commands and timers outstanding.
    pub count: usize,
    /// `idle`, `model`, `tool` or `waiting`.
    pub phase: &'static str,
    /// When its timers fire.
    pub timers: Vec<i64>,
    /// A model call being retried: its failures so far and the last one's kind.
    pub retrying: Option<(u32, &'static str)>,
}

pub(super) fn agent_outstanding_summary(agent: &Instance) -> OutstandingSummary {
    let mut timers: Vec<i64> = agent
        .tasks
        .values()
        .filter_map(|(_, c)| match c {
            Command::Timer { at_ms } => Some(*at_ms),
            _ => None,
        })
        .collect();
    let task_timers = timers.len();
    // Finite waits and render retries are timers too (as in `outstanding()`).
    timers.extend(
        agent
            .open
            .values()
            .filter_map(|o| o.wait.as_ref().map(|w| w.until_ms).filter(|t| *t != i64::MAX))
            .chain(agent.open.values().filter_map(|o| o.retry.as_ref().and_then(|r| r.at))),
    );
    let count = agent.tasks.len() + (timers.len() - task_timers);
    let phase = if count == 0 {
        "idle"
    } else if agent.tasks.values().any(|(_, c)| matches!(c, Command::Generate { .. })) {
        "model"
    } else if agent.tasks.values().any(|(_, c)| matches!(c, Command::Call { .. })) {
        "tool"
    } else {
        "waiting"
    };
    let retrying = agent
        .open
        .values()
        .filter_map(|o| o.retry.as_ref())
        .map(|r| (r.failures, r.kind))
        .max_by_key(|(failures, _)| *failures);
    OutstandingSummary {
        count,
        phase,
        timers,
        retrying,
    }
}

pub(super) fn agent_outstanding(agent: &Instance) -> Vec<Command> {
    let mut out: Vec<Command> = agent.tasks.values().map(|(_, c)| c.clone()).collect();
    // A model call told to stop is told again: on restore it answers at once, with what it has.
    out.extend(
        agent
            .aborting
            .iter()
            .filter(|t| agent.tasks.contains_key(t))
            .map(|&task| Command::Abort { task }),
    );
    for o in agent.open.values() {
        if let Some(at_ms) = o.wait.as_ref().map(|w| w.until_ms).filter(|t| *t != i64::MAX) {
            out.push(Command::Timer { at_ms });
        }
        if let Some(at_ms) = o.retry.as_ref().and_then(|r| r.at) {
            out.push(Command::Timer { at_ms });
        }
    }
    out
}
