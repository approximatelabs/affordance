//! `model(view, {pipe})`: see `Pipe`.
use crate::emit::Encoder;
use crate::runtime::exec::{CallContext, Executor};
use serde_json::Value as J;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, channel, sync_channel};
use std::time::Duration;

/// How many written pieces may wait for a slow sink before the pipe closes: a sink that falls
/// this far behind is treated as gone, and the model call goes on without it.
const BACKLOG: usize = 1024;
/// How long the end of a reply waits for its pieces to reach the sink before giving up on it.
const DRAIN: Duration = Duration::from_secs(30);

/// `model(view, {pipe})`: a reply's pieces written to a machine as they are generated, each in the
/// frame the app declared. A frame is a string in which `PIECE` (U+E000) stands for the piece,
/// escaped as the inside of a JSON string; a tool call's frame also takes its index (U+E001), id
/// (U+E002) and name (U+E003). A kind with no frame is not written. The app's code does not run
/// for a piece, and nothing of a piece is journaled: the reply is what the app receives.
///
/// The model's thread only fills in frames: a writer thread of the pipe's own calls the machine,
/// so a slow sink (an HTTP client that stopped reading) never holds a model call, and one more
/// than `BACKLOG` pieces behind closes the pipe. A native binding's ids are decoded as they arrive
/// with the binding's tokenizer, whole characters only.
pub(super) struct Pipe {
    send: Option<SyncSender<J>>,
    /// Told when the writer has written everything (or the pipe is gone).
    drained: Receiver<()>,
    call: serde_json::Map<String, J>,
    text: Option<String>,
    reasoning: Option<String>,
    tool_call: Option<String>,
    /// Ids of a native reply not yet written: a character not yet complete.
    pending: Vec<u32>,
    enc: Option<Arc<Encoder>>,
    /// The sink failed or fell behind: nothing more is written. The model call goes on; stopping
    /// it is its `signal`'s business.
    closed: Arc<AtomicBool>,
    /// Something of the reply has been written: the call cannot be sent again without the
    /// client receiving the reply's beginning twice.
    pub(super) wrote: bool,
}

impl Pipe {
    pub(super) fn new(exec: Arc<dyn Executor>, ctx: CallContext, options: &J, enc: Option<Arc<Encoder>>) -> Self {
        let frame = |name: &str| options[name].as_str().map(str::to_owned);
        let (send, receive) = sync_channel::<J>(BACKLOG);
        let closed = Arc::new(AtomicBool::new(false));
        let gone = closed.clone();
        let (done, drained) = channel();
        let _ = std::thread::Builder::new().name("pipe".into()).spawn(move || {
            for args in receive {
                if gone.load(Ordering::SeqCst) {
                    continue;
                }
                if exec.call(&ctx, &args).is_err() {
                    gone.store(true, Ordering::SeqCst);
                }
            }
            let _ = done.send(());
        });
        Pipe {
            send: Some(send),
            drained,
            call: options["call"].as_object().cloned().unwrap_or_default(),
            text: frame("text"),
            reasoning: frame("reasoning"),
            tool_call: frame("tool_call"),
            pending: vec![],
            enc,
            closed,
            wrote: false,
        }
    }

    pub(super) fn write(&mut self, delta: crate::runtime::model::Delta<'_>) {
        use crate::runtime::model::Delta;
        if self.closed.load(Ordering::SeqCst) {
            return;
        }
        let chunk = match delta {
            Delta::Text(text) if !text.is_empty() => {
                self.text.as_deref().map(|f| fill(f, &[('\u{E000}', &inside(text))]))
            }
            Delta::Reasoning(text) if !text.is_empty() => self
                .reasoning
                .as_deref()
                .map(|f| fill(f, &[('\u{E000}', &inside(text))])),
            Delta::Call {
                index,
                id,
                name,
                arguments,
            } => self.tool_call.as_deref().map(|f| {
                fill(
                    f,
                    &[
                        ('\u{E001}', &index.to_string()),
                        ('\u{E002}', &inside(id.unwrap_or_default())),
                        ('\u{E003}', &inside(name.unwrap_or_default())),
                        ('\u{E000}', &inside(arguments)),
                    ],
                )
            }),
            Delta::Ids(ids) => {
                let text = self.decode(ids);
                match (text, self.text.as_deref()) {
                    (Some(text), Some(f)) if !text.is_empty() => Some(fill(f, &[('\u{E000}', &inside(&text))])),
                    _ => None,
                }
            }
            _ => None,
        };
        let Some(chunk) = chunk else { return };
        let mut args = self.call.clone();
        args.insert("chunk".into(), J::String(chunk));
        self.wrote = true;
        match self.send.as_ref().map(|s| s.try_send(J::Object(args))) {
            Some(Ok(())) => {}
            Some(Err(TrySendError::Full(_) | TrySendError::Disconnected(_))) | None => {
                self.closed.store(true, Ordering::SeqCst)
            }
        }
    }

    /// A native reply's new ids as text: whole characters only, the rest kept for the next ids.
    fn decode(&mut self, ids: &[u32]) -> Option<String> {
        let enc = self.enc.as_ref()?;
        self.pending.extend_from_slice(ids);
        let text = enc.tok.decode(&self.pending, false).ok()?;
        if text.ends_with('\u{FFFD}') && self.pending.len() < 16 {
            return None;
        }
        self.pending.clear();
        Some(text)
    }

    /// Wait until every piece written has reached the sink (or the sink is gone), so what the app
    /// writes after the reply arrives after the reply. A sink still stuck after `DRAIN` is let go.
    pub(super) fn finish(mut self) {
        drop(self.send.take());
        if self.drained.recv_timeout(DRAIN).is_err() {
            self.closed.store(true, Ordering::SeqCst);
        }
    }
}

/// The inside of a JSON string: what a frame's placeholder stands in.
fn inside(text: &str) -> String {
    let quoted = serde_json::to_string(text).unwrap_or_default();
    quoted[1..quoted.len().saturating_sub(1)].to_string()
}

/// `frame` with each placeholder replaced by its value, in one pass: a value is never read for
/// placeholders (a tool call's id that holds one stays as it is).
fn fill(frame: &str, values: &[(char, &str)]) -> String {
    let mut out = String::with_capacity(frame.len() + 32);
    for c in frame.chars() {
        match values.iter().find(|(p, _)| *p == c) {
            Some((_, v)) => out.push_str(v),
            None => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_placeholder_in_a_filled_value_stays_as_it_is() {
        let frame = "{\"id\":\"\u{E002}\",\"name\":\"\u{E003}\",\"args\":\"\u{E000}\"}";
        let filled = super::fill(
            frame,
            &[('\u{E002}', "a\u{E003}b"), ('\u{E003}', "shell"), ('\u{E000}', "{}")],
        );
        assert_eq!(filled, "{\"id\":\"a\u{E003}b\",\"name\":\"shell\",\"args\":\"{}\"}");
    }
}
