//! One durable result for single-recipient and multicast sends.
use super::*;

struct Group {
    sender: String,
    incarnation: u64,
    task: u64,
    targets: Vec<String>,
    pending: AtomicUsize,
    failed: Mutex<HashSet<String>>,
}
enum Reply {
    /// An embedder's post (`Control::post`), answered once the event's journal line is committed.
    Posted(Sender<Result<(), String>>),
    Single {
        sender: String,
        incarnation: u64,
        task: u64,
    },
    Group(Arc<Group>),
}

/// Discarding a queued delivery must settle its acknowledgement too.
pub(super) struct Ack {
    shared: std::sync::Weak<Shared>,
    target: String,
    reply: Option<Reply>,
}
impl Ack {
    fn complete(&mut self, delivered: bool) {
        let Some(reply) = self.reply.take() else { return };
        if let Reply::Posted(tx) = reply {
            let _ = tx.send(if delivered {
                Ok(())
            } else {
                Err(format!("{} did not take it (it failed, or stopped)", self.target))
            });
            return;
        }
        let Some(shared) = self.shared.upgrade() else { return };
        match reply {
            Reply::Posted(_) => unreachable!(),
            Reply::Single {
                sender,
                incarnation,
                task,
            } => answer(
                &shared,
                &sender,
                incarnation,
                task,
                json!({"to":self.target,"delivered":delivered}),
            ),
            Reply::Group(group) => {
                if !delivered {
                    group.failed.lock().unwrap().insert(self.target.clone());
                }
                if group.pending.fetch_sub(1, Ordering::SeqCst) == 1 {
                    let failed = group.failed.lock().unwrap();
                    let undelivered: Vec<_> = group.targets.iter().filter(|id| failed.contains(*id)).collect();
                    answer(
                        &shared,
                        &group.sender,
                        group.incarnation,
                        group.task,
                        json!({"delivered":group.targets.len()-undelivered.len(),"failed":undelivered.len(),"undelivered":undelivered}),
                    );
                }
            }
        }
    }
    pub(super) fn finish(mut self, delivered: bool) {
        self.complete(delivered);
    }
    /// An embedder's post is answered only once its line is committed: what it was told is durable.
    pub(super) fn waits_for_commit(&self) -> bool {
        matches!(self.reply, Some(Reply::Posted(_)))
    }
    /// Settle nothing: the sender gets no answer, so its call stays outstanding in its journal and
    /// journal recovery issues it again (a closing run's queued deliveries end this way, not as
    /// failures). An embedder's post is told the run is closing, to post it again after restore.
    pub(super) fn abandon(mut self) {
        if let Some(Reply::Posted(tx)) = self.reply.take() {
            let _ = tx.send(Err(CLOSING.into()));
        }
    }
}
impl Drop for Ack {
    /// A delivery dropped unsettled (its target gone, or the run ending with it still queued) was
    /// not delivered. An embedder's post is told so, to post it again.
    fn drop(&mut self) {
        if let Some(Reply::Posted(tx)) = &self.reply {
            let _ = tx.send(Err(format!(
                "not delivered: the run ended or {} is gone (with the same key, a post is taken once)",
                self.target
            )));
            self.reply = None;
        }
        self.complete(false);
    }
}

fn answer(shared: &Shared, sender: &str, incarnation: u64, task: u64, value: J) {
    shared.post_to(
        sender,
        InputKind::Completion {
            incarnation,
            input: Box::new(InputKind::Result {
                task,
                outcome: Outcome::Ok(value),
            }),
        },
    );
}

pub(super) fn send(shared: &Arc<Shared>, sender: &str, incarnation: u64, task: u64, args: &J) -> Result<(), String> {
    let (targets, multicast) = match &args["to"] {
        J::String(to) => (vec![to.clone()], false),
        J::Array(values) => {
            let mut seen = HashSet::new();
            let mut targets = vec![];
            for value in values {
                let id = value.as_str().ok_or("send targets must be strings")?;
                if seen.insert(id) {
                    targets.push(id.to_owned());
                }
            }
            (targets, true)
        }
        _ => return Err("send needs a target id or an array of ids".into()),
    };
    // A <Mount>'s callbacks are the runtime's to deliver (a child calling its prop), never a peer's.
    let event = args["event"].as_str().unwrap_or("message");
    if event.starts_with("mount:") {
        return Err(format!(
            "send: event {event:?} is reserved (mount:* are a <Mount>'s callbacks)"
        ));
    }
    if targets.is_empty() {
        answer(
            shared,
            sender,
            incarnation,
            task,
            json!({"delivered":0,"failed":0,"undelivered":[]}),
        );
        return Ok(());
    }
    let group = multicast.then(|| {
        Arc::new(Group {
            sender: sender.into(),
            incarnation,
            task,
            pending: AtomicUsize::new(targets.len()),
            targets: targets.clone(),
            failed: Mutex::new(HashSet::new()),
        })
    });
    let mut payload = match &args["payload"] {
        J::Object(o) => J::Object(o.clone()),
        J::Null => json!({}),
        other => json!({"value":other}),
    };
    payload["from"] = sender.into();
    // The message's delivery id, on the event and not in its payload. A name may hold `:`. The
    // short form is used only where it cannot be read as another sender's long form (`a:1` task 5
    // against `a`, incarnation 1, task 5).
    let id = if incarnation == 0 && !sender.contains(':') {
        format!("{sender}:{task}")
    } else {
        format!("{sender}:{incarnation}:{task}")
    };
    for target in targets {
        let reply = match &group {
            Some(group) => Reply::Group(group.clone()),
            None => Reply::Single {
                sender: sender.into(),
                incarnation,
                task,
            },
        };
        let ack = Ack {
            shared: Arc::downgrade(shared),
            target: target.clone(),
            reply: Some(reply),
        };
        if target == sender {
            ack.finish(false);
        } else {
            shared.post_to(
                &target,
                InputKind::Event {
                    name: event.into(),
                    payload: payload.clone(),
                    ack: Some(ack),
                    id: Some(id.clone()),
                },
            );
        }
    }
    Ok(())
}

/// What a post is told when the run is closing, or closed, before its event is journaled.
pub(super) const CLOSING: &str =
    "the run is closing: post it again once the run is restored (with the same key, it is taken once)";

/// An embedder's post: delivered to `target` as the event `event`, answered on `reply` once its
/// journal line is committed, or with why it was not.
pub(super) fn post(
    shared: &Arc<Shared>,
    target: &str,
    event: String,
    payload: J,
    key: Option<String>,
    reply: Sender<Result<(), String>>,
) {
    if event.starts_with("mount:") {
        let _ = reply.send(Err(format!(
            "event {event:?} is reserved (mount:* are a <Mount>'s callbacks)"
        )));
        return;
    }
    if !shared.route.read().unwrap().contains_key(target) {
        let _ = reply.send(Err(format!("no instance {target}")));
        return;
    }
    let ack = Ack {
        shared: Arc::downgrade(shared),
        target: target.into(),
        reply: Some(Reply::Posted(reply)),
    };
    // An instance unmounted just now drops it, and its acknowledgement says it was not taken.
    shared.post_to(
        target,
        InputKind::Event {
            name: event,
            payload,
            ack: Some(ack),
            // Its own namespace: a send's id starts with its sender's name, which never starts with
            // `.`, so a post's key never matches a message from another instance.
            id: key.map(|key| format!(".post:{key}")),
        },
    );
}
