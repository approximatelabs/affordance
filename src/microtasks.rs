//! Instance-local Promise cells and FIFO jobs. Suspended work belongs to a task, not a replay.
use crate::{
    effects::{Outcome, Request},
    interp::{Interp, RResult},
    value::Value,
};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, VecDeque},
    rc::{Rc, Weak},
};
pub(crate) type Task = u64;
type JobId = u64;
#[derive(Clone, Debug)]
pub(crate) struct Rejection {
    pub text: String,
    pub reason: Value,
}
#[derive(Clone, Debug)]
pub(crate) enum Settle {
    Ok(Value),
    Err(Rejection),
    Pending,
}
impl Settle {
    pub(crate) fn value(&self) -> Option<&Value> {
        match self {
            Self::Ok(v) => Some(v),
            Self::Err(e) => Some(&e.reason),
            Self::Pending => None,
        }
    }
}
#[derive(Debug)]
/// A promise: pending, or settled with a value or a rejection.
pub struct Promise {
    pub(crate) state: RefCell<Settle>,
    pub(crate) waiters: RefCell<Vec<(Weak<RefCell<Queue>>, JobId)>>,
    pub(crate) frozen: Cell<bool>,
    /// Something reads its result: an `await`, a reaction, a combinator, or the host (a task's
    /// root). A rejection nothing reads is reported (`Queue::take_unhandled`).
    pub(crate) handled: Cell<bool>,
}
impl Promise {
    pub(crate) fn pending() -> Rc<Self> {
        Rc::new(Self {
            state: RefCell::new(Settle::Pending),
            waiters: RefCell::new(vec![]),
            frozen: Cell::new(false),
            handled: Cell::new(false),
        })
    }
    pub(crate) fn settled(value: Settle) -> Rc<Self> {
        let p = Self::pending();
        *p.state.borrow_mut() = value;
        p
    }
    pub(crate) fn completed(&self) -> Option<Settle> {
        match &*self.state.borrow() {
            Settle::Pending => None,
            value => Some(value.clone()),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Combinator {
    All,
    Race,
    Any,
    AllSettled,
}
#[derive(Clone)]
pub(crate) enum Reaction {
    Then(Option<Value>, Option<Value>),
    Finally(Option<Value>),
}
struct Join {
    kind: Combinator,
    target: Rc<Promise>,
    values: Vec<Option<Settle>>,
    remaining: usize,
}
enum Job {
    Body(Box<crate::interp::vm::Activation>),
    Reaction {
        source: Rc<Promise>,
        target: Rc<Promise>,
        callback: Reaction,
    },
    Adopt {
        source: Rc<Promise>,
        target: Rc<Promise>,
    },
    Forward {
        source: Rc<Promise>,
        target: Rc<Promise>,
    },
    Finally {
        source: Rc<Promise>,
        target: Rc<Promise>,
        original: Settle,
    },
    Join {
        source: Rc<Promise>,
        join: Rc<RefCell<Join>>,
        index: usize,
    },
}
impl Job {
    fn target(&self) -> Rc<Promise> {
        match self {
            Self::Body(body) => body.result.clone(),
            Self::Reaction { target, .. }
            | Self::Adopt { target, .. }
            | Self::Forward { target, .. }
            | Self::Finally { target, .. } => target.clone(),
            Self::Join { join, .. } => join.borrow().target.clone(),
        }
    }
    fn source(&self) -> Option<Rc<Promise>> {
        match self {
            Self::Body(body) => body.waiting.as_ref().map(|(source, _)| source.clone()),
            Self::Reaction { source, .. }
            | Self::Adopt { source, .. }
            | Self::Forward { source, .. }
            | Self::Finally { source, .. }
            | Self::Join { source, .. } => Some(source.clone()),
        }
    }
}
struct OwnedJob {
    task: Task,
    job: Job,
}
pub(crate) struct Group {
    pub run: u64,
    pub root: Rc<Promise>,
    pub hosts: HashMap<usize, Rc<Promise>>,
    next_host: usize,
    pub ui: Option<crate::interp::vm::Ui>,
}
#[derive(Default)]
pub(crate) struct Queue {
    weak: Weak<RefCell<Queue>>,
    next: JobId,
    jobs: HashMap<JobId, OwnedJob>,
    ready: VecDeque<JobId>,
    pub groups: HashMap<Task, Group>,
    pub fuel: u64,
    /// The task whose code is running or being answered: the owner of what rejects meanwhile.
    pub current: Option<Task>,
    /// Promises that rejected with nothing reading them, and the task they rejected in.
    rejected: Vec<(Option<Task>, Rc<Promise>)>,
}
pub(crate) type Realm = Rc<RefCell<Queue>>;
impl Queue {
    pub(crate) fn new() -> Realm {
        Rc::new_cyclic(|weak| {
            RefCell::new(Self {
                weak: weak.clone(),
                next: 0,
                jobs: HashMap::new(),
                ready: VecDeque::new(),
                groups: HashMap::new(),
                fuel: 0,
                current: None,
                rejected: vec![],
            })
        })
    }
    pub(crate) fn begin(&mut self, task: Task, run: u64) -> Rc<Promise> {
        let root = Promise::pending();
        // The host reads a task's result.
        root.handled.set(true);
        self.groups.insert(
            task,
            Group {
                run,
                root: root.clone(),
                hosts: HashMap::new(),
                next_host: 0,
                ui: None,
            },
        );
        root
    }
    pub(crate) fn next_task(&mut self) -> Option<Task> {
        loop {
            let id = *self.ready.front()?;
            if let Some(job) = self.jobs.get(&id) {
                return Some(job.task);
            }
            self.ready.pop_front();
        }
    }
    fn put(&mut self, task: Task, job: Job, wait: bool) {
        if let Some(read) = job.source() {
            read.handled.set(true);
        }
        let source = if wait { job.source() } else { None };
        let id = self.next;
        self.next += 1;
        self.jobs.insert(id, OwnedJob { task, job });
        if let Some(source) = source {
            if source.completed().is_some() {
                self.ready.push_back(id);
            } else {
                source.waiters.borrow_mut().push((self.weak.clone(), id));
            }
        } else {
            self.ready.push_back(id);
        }
    }
    pub(crate) fn wait_body(&mut self, task: Task, body: crate::interp::vm::Activation) {
        self.put(task, Job::Body(Box::new(body)), true);
    }
    pub(crate) fn has_jobs(&self, task: Task) -> bool {
        self.jobs.values().any(|j| j.task == task)
    }
    pub(crate) fn is_idle(&self) -> bool {
        self.jobs.is_empty() && self.groups.is_empty() && self.ready.is_empty()
    }
    /// Drop `task`'s group if nothing of it is left: no job, no host promise, no suspended UI.
    /// Its root stays pending (no one awaits it); the task's next use begins a fresh group.
    pub(crate) fn forget_spent(&mut self, task: Task) {
        if self
            .groups
            .get(&task)
            .is_some_and(|g| g.hosts.is_empty() && g.ui.is_none())
            && !self.has_jobs(task)
        {
            self.groups.remove(&task);
        }
    }
    pub(crate) fn finish(&mut self, task: Task) {
        let mut removed = self
            .jobs
            .iter()
            .filter(|(_, job)| job.task == task)
            .map(|(id, job)| (*id, job.job.source(), job.job.target()))
            .collect::<Vec<_>>();
        removed.sort_by_key(|(id, _, _)| *id);
        let mut targets = vec![];
        for (id, source, target) in removed {
            self.jobs.remove(&id);
            if let Some(source) = source {
                source
                    .waiters
                    .borrow_mut()
                    .retain(|(origin, want)| !(*want == id && Weak::ptr_eq(origin, &self.weak)));
            }
            targets.push(target);
        }
        if let Some(group) = self.groups.remove(&task) {
            targets.push(group.root);
            let mut hosts = group.hosts.into_iter().collect::<Vec<_>>();
            hosts.sort_by_key(|(index, _)| *index);
            targets.extend(hosts.into_iter().map(|(_, promise)| promise));
        }
        let cancelled = Settle::Err(error("AbortError: Task cancelled"));
        for target in targets {
            // Its task is over: nothing is left to read it, and that is not an error.
            target.handled.set(true);
            self.settle(&target, cancelled.clone());
        }
    }
    /// The rejections nothing has read by now (the end of a round of jobs), each with the task it
    /// happened in and its message: the platform's `unhandledrejection`. One read later is not
    /// reported again.
    pub(crate) fn take_unhandled(&mut self) -> Vec<(Option<Task>, String)> {
        std::mem::take(&mut self.rejected)
            .into_iter()
            .filter(|(_, p)| !p.handled.get())
            .filter_map(|(task, p)| match p.completed() {
                Some(Settle::Err(e)) => Some((task, e.text)),
                _ => None,
            })
            .collect()
    }
    pub(crate) fn external(&mut self, task: Task, index: usize) -> Rc<Promise> {
        let group = self.groups.get_mut(&task).expect("active task");
        assert_eq!(index, group.next_host);
        group.next_host += 1;
        let p = Promise::pending();
        group.hosts.insert(index, p.clone());
        p
    }
    pub(crate) fn settle(&mut self, p: &Rc<Promise>, result: Settle) {
        if p.completed().is_some() {
            return;
        }
        if p.frozen.get()
            && let Some(value) = result.value()
        {
            crate::value::freeze(value);
        }
        if matches!(result, Settle::Err(..)) && !p.handled.get() {
            self.rejected.push((self.current, p.clone()));
        }
        *p.state.borrow_mut() = result;
        for (origin, id) in p.waiters.borrow_mut().drain(..) {
            if Weak::ptr_eq(&origin, &self.weak) {
                self.ready.push_back(id);
            } else if let Some(origin) = origin.upgrade() {
                origin.borrow_mut().ready.push_back(id);
            }
        }
    }
    pub(crate) fn resolve(&mut self, task: Task, target: &Rc<Promise>, value: Value) {
        if let Value::Promise(source) = value {
            if Rc::ptr_eq(target, &source) {
                self.settle(
                    target,
                    Settle::Err(error("TypeError: Chaining cycle detected for promise")),
                );
            } else {
                self.put(
                    task,
                    Job::Adopt {
                        source,
                        target: target.clone(),
                    },
                    false,
                );
            }
        } else if custom_thenable(&value) {
            self.settle(
                target,
                Settle::Err(error("TypeError: Custom thenables are not supported; return a Promise")),
            );
        } else {
            self.settle(target, Settle::Ok(value));
        }
    }
    pub(crate) fn promise(&mut self, task: Task, value: Value) -> Rc<Promise> {
        match value {
            Value::Promise(p) => p,
            value => {
                let p = Promise::pending();
                self.resolve(task, &p, value);
                p
            }
        }
    }
    pub(crate) fn react(&mut self, task: Task, source: Rc<Promise>, callback: Reaction) -> Rc<Promise> {
        let target = Promise::pending();
        self.put(
            task,
            Job::Reaction {
                source,
                target: target.clone(),
                callback,
            },
            true,
        );
        target
    }
    pub(crate) fn combine(&mut self, task: Task, kind: Combinator, values: Vec<Value>) -> Rc<Promise> {
        let target = Promise::pending();
        let remaining = values.len();
        let join = Rc::new(RefCell::new(Join {
            kind,
            target: target.clone(),
            values: vec![None; remaining],
            remaining,
        }));
        if remaining == 0 {
            match kind {
                Combinator::All | Combinator::AllSettled => self.settle(&target, Settle::Ok(Value::list(vec![]))),
                Combinator::Any => self.settle(&target, Settle::Err(aggregate(vec![]))),
                Combinator::Race => {}
            }
        }
        for (index, value) in values.into_iter().enumerate() {
            let source = self.promise(task, value);
            self.put(
                task,
                Job::Join {
                    source,
                    join: join.clone(),
                    index,
                },
                true,
            );
        }
        target
    }
    pub(crate) fn reply(&mut self, task: Task, index: usize, request: &Request, outcome: &Outcome) {
        let Some(p) = self.groups.get_mut(&task).and_then(|g| g.hosts.remove(&index)) else {
            return;
        };
        self.current = Some(task);
        let result = match outcome {
            Outcome::Ok(v) => Settle::Ok(
                if matches!(request, Request::Render { .. }) || matches!(request,Request::Call{app,..}if app=="model") {
                    crate::effects::model_output_value(v)
                } else {
                    Value::from_json(v)
                },
            ),
            Outcome::Error(e) => Settle::Err(error(&if crate::effects::typed_error(e).is_some() {
                e.clone()
            } else {
                format!("host effect failed: {e}")
            })),
        };
        self.settle(&p, result);
    }
    pub(crate) fn run(realm: &Realm, it: &mut Interp<'_>, task: Task) -> RResult<()> {
        Self::run_job(realm, Some(it), task)
    }
    pub(crate) fn native(realm: &Realm, task: Task) -> RResult<()> {
        Self::run_job(realm, None, task)
    }
    fn run_job(realm: &Realm, mut it: Option<&mut Interp<'_>>, task: Task) -> RResult<()> {
        if let Some(it) = it.as_deref_mut() {
            it.spend()?;
        } else {
            let mut q = realm.borrow_mut();
            q.fuel += 1;
            if q.fuel > crate::interp::FUEL {
                return Err("out of fuel in Promise jobs".into());
            }
        }
        let owned = {
            let mut q = realm.borrow_mut();
            let id = q.ready.pop_front().expect("ready job");
            q.jobs.remove(&id).expect("live job")
        };
        assert_eq!(owned.task, task);
        match owned.job {
            Job::Body(body) => it.expect("activation context").resume_activation(*body),
            Job::Adopt { source, target } => {
                realm.borrow_mut().put(task, Job::Forward { source, target }, true);
                Ok(())
            }
            Job::Forward { source, target } => {
                let value = source.completed().expect("settled dependency");
                realm.borrow_mut().settle(&target, value);
                Ok(())
            }
            Job::Finally {
                source,
                target,
                original,
            } => {
                let result = match source.completed().expect("settled cleanup") {
                    Settle::Ok(..) => original,
                    result => result,
                };
                realm.borrow_mut().settle(&target, result);
                Ok(())
            }
            Job::Join { source, join, index } => {
                let result = source.completed().expect("settled input");
                let mut join = join.borrow_mut();
                let target = join.target.clone();
                if target.completed().is_some() {
                    return Ok(());
                }
                join.remaining -= 1;
                join.values[index] = Some(result.clone());
                match (&join.kind, &result) {
                    (Combinator::Race, _) | (Combinator::Any, Settle::Ok(..)) | (Combinator::All, Settle::Err(..)) => {
                        realm.borrow_mut().settle(&target, result)
                    }
                    _ if join.remaining == 0 => {
                        let values = join.values.iter().map(|v| v.clone().unwrap()).collect::<Vec<_>>();
                        let result = match join.kind {
                            Combinator::All => Settle::Ok(Value::list(
                                values
                                    .into_iter()
                                    .map(|v| match v {
                                        Settle::Ok(v) => v,
                                        _ => unreachable!(),
                                    })
                                    .collect(),
                            )),
                            Combinator::Any => Settle::Err(aggregate(
                                values
                                    .into_iter()
                                    .map(|v| match v {
                                        Settle::Err(e) => e.reason,
                                        _ => unreachable!(),
                                    })
                                    .collect(),
                            )),
                            Combinator::AllSettled => Settle::Ok(Value::list(
                                values
                                    .into_iter()
                                    .map(|v| {
                                        let mut m = crate::value::Map::default();
                                        match v {
                                            Settle::Ok(v) => {
                                                m.insert(Rc::from("status"), Value::str("fulfilled"));
                                                m.insert(Rc::from("value"), v);
                                            }
                                            Settle::Err(e) => {
                                                m.insert(Rc::from("status"), Value::str("rejected"));
                                                m.insert(Rc::from("reason"), e.reason);
                                            }
                                            _ => unreachable!(),
                                        }
                                        Value::map(m)
                                    })
                                    .collect(),
                            )),
                            Combinator::Race => unreachable!(),
                        };
                        realm.borrow_mut().settle(&target, result);
                    }
                    _ => {}
                }
                Ok(())
            }
            Job::Reaction {
                source,
                target,
                callback,
            } => {
                let it = it.expect("reaction context");
                let original = source.completed().expect("settled reaction");
                let (f, args, finally) = match (callback, &original) {
                    (Reaction::Then(f, _), Settle::Ok(v)) => (f, vec![v.clone()], false),
                    (Reaction::Then(_, f), Settle::Err(e)) => (f, vec![e.reason.clone()], false),
                    (Reaction::Finally(f), _) => (f, vec![], true),
                    _ => unreachable!(),
                };
                let Some(f) = f else {
                    realm.borrow_mut().settle(&target, original);
                    return Ok(());
                };
                match it.call_value(&f, args) {
                    Ok(value) => {
                        if finally {
                            let mut q = realm.borrow_mut();
                            let cleanup = q.promise(task, value);
                            let next = Promise::pending();
                            q.put(
                                task,
                                Job::Finally {
                                    source: cleanup,
                                    target: next.clone(),
                                    original,
                                },
                                true,
                            );
                            q.resolve(task, &target, Value::Promise(next));
                        } else {
                            realm.borrow_mut().resolve(task, &target, value);
                        }
                    }
                    Err(text) => {
                        let reason = it.caught(&text);
                        realm
                            .borrow_mut()
                            .settle(&target, Settle::Err(Rejection { text, reason }));
                    }
                }
                Ok(())
            }
        }
    }
}
impl Drop for Queue {
    fn drop(&mut self) {
        self.jobs.clear();
        self.groups.clear();
        crate::value::sweep();
    }
}
pub(crate) fn error(e: &str) -> Rejection {
    let text = crate::error_message(e)
        .trim_start_matches("raise: ")
        .trim_start_matches("host effect failed: ");
    let (name, message) = match crate::effects::typed_error(text) {
        Some(n) => (n, &text[n.len() + 2..]),
        None => ("Error", text),
    };
    Rejection {
        text: e.into(),
        reason: crate::interp::make_error(name, message, Some(e.into()), None),
    }
}
fn aggregate(values: Vec<Value>) -> Rejection {
    let reason = crate::interp::make_error("AggregateError", "All promises were rejected", None, None);
    if let Value::Map(m) = &reason {
        m.write()
            .expect("fresh error")
            .insert(Rc::from("errors"), Value::list(values));
    }
    Rejection {
        text: "AggregateError: All promises were rejected".into(),
        reason,
    }
}

pub(crate) const DOCUMENT: Task = u64::MAX;
thread_local! {static CURRENT:RefCell<Option<Realm>>=const{RefCell::new(None)};}
pub(crate) fn current() -> Option<Realm> {
    CURRENT.with(|r| r.borrow().clone())
}
pub(crate) fn within<T>(realm: Realm, body: impl FnOnce() -> T) -> T {
    struct Restore(Option<Realm>);
    impl Drop for Restore {
        fn drop(&mut self) {
            CURRENT.with(|r| *r.borrow_mut() = self.0.take());
        }
    }
    let previous = CURRENT.with(|r| r.replace(Some(realm)));
    let _restore = Restore(previous);
    body()
}

fn custom_thenable(value: &Value) -> bool {
    if matches!(crate::interp::get_attr(value,"then"),Ok(v) if v.is_callable()) {
        return true;
    }
    matches!(value,Value::Map(m) if m.ext().is_some_and(|ext|ext.class.find("then",|c|&c.methods).is_some()||ext.class.find("then",|c|&c.getters).is_some()))
}
