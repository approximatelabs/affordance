//! Bounded scheduling windows and per-instance durable frames.
use super::*;
const WINDOW: usize = 256;
const MAX_BYTES: usize = 1024 * 1024;

fn eligible(input: &InputKind) -> bool {
    match input {
        InputKind::Event { .. } | InputKind::Result { .. } => true,
        InputKind::Completion { input, .. } => matches!(&**input, InputKind::Result { .. }),
        _ => false,
    }
}
pub(super) fn fill(queue: &mut VecDeque<Msg>, rx: &Receiver<Msg>, limit: usize) {
    let window = if limit == 1 { 1 } else { WINDOW };
    while queue.len() < window {
        let Ok(message) = rx.try_recv() else { break };
        let barrier = !matches!(&message,Msg::Input(_,kind) if eligible(kind));
        queue.push_back(message);
        if barrier {
            break;
        }
    }
}
pub(super) fn collect(queue: &mut VecDeque<Msg>, id: &str, first: InputKind, limit: usize) -> Vec<InputKind> {
    let mut inputs = vec![first];
    if limit == 1 || !eligible(&inputs[0]) {
        return inputs;
    }
    let count = queue
        .iter()
        .take(WINDOW - 1)
        .take_while(|msg| matches!(msg,Msg::Input(_,kind) if eligible(kind)))
        .count();
    let mut rest = Vec::with_capacity(count);
    for _ in 0..count {
        match queue.pop_front().unwrap() {
            Msg::Input(other, kind) if other == id && inputs.len() < limit => inputs.push(kind),
            other => rest.push(other),
        }
    }
    for item in rest.into_iter().rev() {
        queue.push_front(item);
    }
    inputs
}

/// While the run closes, a queued delivery is not applied, so closing never waits behind a
/// backlog. A `send` is abandoned: the sender's call stays outstanding in its journal and a restore
/// issues it again. An embedder's post is told the run is closing, to post it again. Everything
/// else (model and machine completions, ticks, mounts, props, parent callbacks) is applied. The
/// caller counts the dropped inputs out of `inflight` with the rest.
fn unless_reissued(kind: InputKind) -> Option<InputKind> {
    match kind {
        InputKind::Event { ack: Some(ack), .. } => {
            ack.abandon();
            None
        }
        other => Some(other),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn feed_group(
    shared: &Arc<Shared>,
    w: usize,
    views: &mut Views,
    slots: &mut BTreeMap<String, Slot>,
    id: &str,
    kinds: Vec<InputKind>,
    timers: &mut Timers,
    local: &mut VecDeque<Msg>,
) {
    let kinds = if shared.closing.load(Ordering::SeqCst) {
        kinds.into_iter().filter_map(unless_reissued).collect()
    } else {
        kinds
    };
    if kinds.is_empty() {
        return;
    }
    if kinds.len() == 1 {
        feed(
            shared,
            w,
            views,
            slots,
            id,
            kinds.into_iter().next().unwrap(),
            timers,
            local,
        );
        return;
    }
    let Some(slot) = slots.get_mut(id) else { return };
    if slot.error.is_some() {
        return;
    }
    let mut inputs = vec![];
    let mut acks = vec![];
    let mut bytes = 32;
    for kind in kinds {
        let kind = match kind {
            InputKind::Completion { incarnation, input } => {
                if incarnation != slot.incarnation {
                    continue;
                }
                *input
            }
            other => other,
        };
        let clock_ms = shared.clock_ms().max(slot.agent.clock_ms());
        let (mut input, ack) = match kind {
            InputKind::Event { name, payload, ack, id } => (
                Input::Event {
                    name,
                    payload,
                    id,
                    clock_ms,
                },
                ack,
            ),
            InputKind::Result { task, outcome } => (
                Input::Result {
                    task,
                    outcome,
                    clock_ms,
                },
                None,
            ),
            _ => unreachable!("only eligible inputs enter a frame"),
        };
        if let Err(error) = slot.agent.lower_input(&mut input) {
            slot.error = Some(error);
            flush(shared, slot, timers);
            return;
        }
        bytes += serde_json::to_vec(&input).unwrap().len() + 1;
        inputs.push(input);
        acks.push(ack);
    }
    if inputs.is_empty() {
        return;
    }
    let entry = slot.spec.entry.as_deref().unwrap_or(&shared.cfg.entry);
    let (view, enc) = match views.get(shared, &slot.app, entry, slot.agent.cfg.model.as_deref()) {
        Ok(v) => v,
        Err(error) => {
            slot.error = Some(error);
            return;
        }
    };
    if bytes > MAX_BYTES || inputs.len() == 1 {
        for (input, ack) in inputs.into_iter().zip(acks) {
            let journaled = apply(shared, w, &view, &enc, slot, input, timers, local);
            settle(shared, w, slot, ack, journaled, timers, local);
        }
    } else {
        let journaled = apply(shared, w, &view, &enc, slot, Input::Batch { inputs }, timers, local);
        for ack in acks {
            settle(shared, w, slot, ack, journaled, timers, local);
        }
    }
}
