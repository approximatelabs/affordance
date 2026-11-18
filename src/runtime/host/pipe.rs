//! `model(view, {pipe})`: see `Pipe`.
use crate::runtime::exec::{CallContext, Executor};
use serde_json::Value as J;
use std::sync::Arc;

/// `model(view, {pipe})`: a reply's pieces written to a machine as they are generated, each in the
/// frame the app declared. A frame is a string in which `PIECE` (U+E000) stands for the piece,
/// escaped as the inside of a JSON string; a tool call's frame also takes its index (U+E001), id
/// (U+E002) and name (U+E003). A kind with no frame is not written. The app's code does not run
/// for a piece, and nothing of a piece is journaled: the reply is what the app receives.
pub(super) struct Pipe {
    exec: Arc<dyn Executor>,
    ctx: CallContext,
    /// The call each piece is written with, to which `chunk` is added (e.g. `{respond: id}`).
    call: serde_json::Map<String, J>,
    text: Option<String>,
    reasoning: Option<String>,
    tool_call: Option<String>,
    /// The sink failed (a client that went away): nothing more is written. The model call goes
    /// on; stopping it is its `signal`'s business.
    closed: bool,
    /// Something of the reply has been written: the call cannot be sent again without the
    /// client receiving the reply's beginning twice.
    pub(super) wrote: bool,
}

impl Pipe {
    pub(super) fn new(exec: Arc<dyn Executor>, ctx: CallContext, options: &J) -> Self {
        let frame = |name: &str| options[name].as_str().map(str::to_owned);
        Pipe {
            exec,
            ctx,
            call: options["call"].as_object().cloned().unwrap_or_default(),
            text: frame("text"),
            reasoning: frame("reasoning"),
            tool_call: frame("tool_call"),
            closed: false,
            wrote: false,
        }
    }

    pub(super) fn write(&mut self, delta: crate::runtime::model::Delta<'_>) {
        use crate::runtime::model::Delta;
        if self.closed {
            return;
        }
        // The inside of a JSON string: what a frame's placeholder stands in.
        let inside = |text: &str| {
            let quoted = serde_json::to_string(text).unwrap_or_default();
            quoted[1..quoted.len().saturating_sub(1)].to_string()
        };
        let chunk = match delta {
            Delta::Text(text) if !text.is_empty() => self.text.as_ref().map(|f| f.replace('\u{E000}', &inside(text))),
            Delta::Reasoning(text) if !text.is_empty() => {
                self.reasoning.as_ref().map(|f| f.replace('\u{E000}', &inside(text)))
            }
            Delta::Call {
                index,
                id,
                name,
                arguments,
            } => self.tool_call.as_ref().map(|f| {
                f.replace('\u{E001}', &index.to_string())
                    .replace('\u{E002}', &inside(id.unwrap_or_default()))
                    .replace('\u{E003}', &inside(name.unwrap_or_default()))
                    .replace('\u{E000}', &inside(arguments))
            }),
            // A native binding's ids are not text yet (they need the reply's parser as a stream).
            _ => None,
        };
        let Some(chunk) = chunk else { return };
        let mut args = self.call.clone();
        args.insert("chunk".into(), J::String(chunk));
        self.wrote = true;
        if self.exec.call(&self.ctx, &J::Object(args)).is_err() {
            self.closed = true;
        }
    }
}
