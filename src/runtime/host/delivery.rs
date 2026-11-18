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
        let Some(shared) = self.shared.upgrade() else { return };
        match reply {
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
    /// Settle nothing: the sender gets no answer, so its call stays outstanding in its journal and
    /// journal recovery issues it again (a pause's queued deliveries end this way, not as failures).
    pub(super) fn abandon(mut self) {
        self.reply = None;
    }
}
impl Drop for Ack {
    fn drop(&mut self) {
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
    // A name may hold `:`. The short form is used only where it cannot be read as another
    // sender's long form (`a:1` task 5 against `a`, incarnation 1, task 5).
    payload["_msg"] = if incarnation == 0 && !sender.contains(':') {
        format!("{sender}:{task}")
    } else {
        format!("{sender}:{incarnation}:{task}")
    }
    .into();
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
                    op: None,
                },
            );
        }
    }
    Ok(())
}
