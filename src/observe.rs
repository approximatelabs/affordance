//! Dependencies of a prepared document on its immutable render context.
use crate::value::Value;
use serde_json::Value as J;
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

#[derive(Clone, Default)]
pub(crate) struct Reads {
    all: bool,
    fields: BTreeSet<String>,
}
impl Reads {
    pub(crate) fn unchanged(&self, before: &J, after: &J) -> bool {
        if self.all {
            before == after
        } else {
            self.fields.iter().all(|key| before.get(key) == after.get(key))
        }
    }
}
struct Active {
    root: usize,
    reads: Reads,
}
thread_local! { static ACTIVE: RefCell<Option<Active>> = const { RefCell::new(None) }; }

pub(crate) fn capture<T>(root: &Value, run: impl FnOnce() -> Result<T, String>) -> Result<(T, Reads), String> {
    let root = match root {
        Value::Map(m) => Rc::as_ptr(m) as usize,
        _ => 0,
    };
    let previous = ACTIVE.with(|slot| {
        let mut slot = slot.borrow_mut();
        if let Some(active) = slot.as_mut() {
            active.reads.all = true;
        }
        slot.replace(Active {
            root,
            reads: Reads::default(),
        })
    });
    struct Reset(Option<Active>);
    impl Drop for Reset {
        fn drop(&mut self) {
            ACTIVE.with(|slot| *slot.borrow_mut() = self.0.take());
        }
    }
    let reset = Reset(previous);
    let result = run();
    let reads = ACTIVE.with(|slot| slot.borrow_mut().take().unwrap().reads);
    drop(reset);
    result.map(|value| (value, reads))
}
pub(crate) fn field(value: &Value, key: &str) {
    let Value::Map(map) = value else { return };
    ACTIVE.with(|slot| {
        if let Some(active) = slot.borrow_mut().as_mut()
            && active.root == Rc::as_ptr(map) as usize
            && !active.reads.all
        {
            active.reads.fields.insert(key.into());
        }
    });
}
pub(crate) fn escape(value: &Value) {
    let Value::Map(map) = value else { return };
    ACTIVE.with(|slot| {
        if let Some(active) = slot.borrow_mut().as_mut()
            && active.root == Rc::as_ptr(map) as usize
        {
            active.reads.all = true;
        }
    });
}
pub(crate) fn clock() {
    ACTIVE.with(|slot| {
        if let Some(active) = slot.borrow_mut().as_mut() {
            active.reads.fields.insert("clock_ms".into());
        }
    });
}
