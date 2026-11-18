//! Saved function execution. The expression evaluator supplies ordinary language operations.
use super::*;
use crate::bytecode::{Code, Op, Test};

#[derive(Debug)]
enum Completion {
    Normal,
    Flow(Flow),
    Throw(String, Value),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TryPhase {
    Body,
    Catch,
    Finally,
}
#[derive(Debug)]
enum Control {
    Target {
        end: usize,
        next: Option<usize>,
        label: Option<Sym>,
        block: bool,
        scope: usize,
        marks: usize,
    },
    Try {
        catch: Option<usize>,
        finally: Option<usize>,
        end: usize,
        name: Option<Sym>,
        phase: TryPhase,
        pending: Completion,
        scope: usize,
        marks: usize,
    },
}
struct Scope {
    locals: usize,
    registers: std::ops::Range<usize>,
}
pub(crate) struct Frame {
    code: Rc<Code>,
    pc: usize,
    base: usize,
    marks: Vec<Scope>,
    controls: Vec<Control>,
    elements: HashMap<usize, (Element, Capture)>,
    refs: HashMap<usize, Reference>,
    iterators: HashMap<usize, IteratorState>,
}
#[derive(Clone)]
pub(super) enum Reference {
    Variable(Sym),
    Property(Value, ReferenceKey),
    Temporary(Value),
}
struct IteratorState {
    source: Value,
    index: usize,
    items: std::vec::IntoIter<Value>,
    _walk: Option<crate::jsbuiltins::CollWalk>,
}
impl IteratorState {
    fn new(source: Value) -> RResult<Self> {
        let walk = if let Value::Coll(c) = &source {
            Some(crate::jsbuiltins::CollWalk::new(c))
        } else {
            None
        };
        let items = if matches!(source, Value::List(_) | Value::Coll(_)) {
            vec![]
        } else {
            iterate(&source)?
        };
        Ok(Self {
            source,
            index: 0,
            items: items.into_iter(),
            _walk: walk,
        })
    }
    fn next(&mut self) -> Option<Value> {
        match &self.source {
            Value::List(l) => {
                let value = l.read().get(self.index).cloned();
                self.index += 1;
                value
            }
            Value::Coll(c) => crate::jsbuiltins::coll_next(c, &mut self.index),
            _ => self.items.next(),
        }
    }
}
pub(crate) struct Context {
    scopes: Vec<(Sym, Slot)>,
    frames: Vec<usize>,
    temps: Vec<Value>,
    scope: Rc<crate::modules::Instance>,
    line: u32,
    depth: usize,
    run: u64,
}
pub(crate) struct Ui {
    path: Vec<String>,
    siblings: Vec<Siblings>,
    hooks: Vec<usize>,
    seen: Map,
    memo: HashMap<(usize, String), usize>,
    updates: Map,
}
pub(crate) struct Activation {
    frame: Frame,
    context: Option<Context>,
    pub result: Rc<crate::effects::Promise>,
    pub waiting: Option<(Rc<crate::effects::Promise>, usize)>,
}
enum Run {
    Done(Flow),
    Await(Value, usize),
}
impl Context {
    fn take(it: &mut Interp<'_>) -> Self {
        Self {
            scopes: std::mem::take(&mut it.scopes),
            frames: std::mem::replace(&mut it.frames, vec![0]),
            temps: std::mem::take(&mut it.temps),
            scope: it.scope.clone(),
            line: it.line,
            depth: it.depth,
            run: it.run,
        }
    }
    fn put(self, it: &mut Interp<'_>) {
        it.scopes = self.scopes;
        it.frames = self.frames;
        it.temps = self.temps;
        it.scope = self.scope;
        it.line = self.line;
        it.depth = self.depth;
        it.run = self.run;
    }
}
impl Frame {
    fn new(code: Rc<Code>, base: usize) -> Self {
        Self {
            code,
            pc: 0,
            base,
            marks: vec![],
            controls: vec![],
            elements: HashMap::new(),
            refs: HashMap::new(),
            iterators: HashMap::new(),
        }
    }
    fn leave(&mut self, it: &mut Interp<'_>) {
        let scope = self.marks.pop().expect("balanced lexical scope");
        it.scopes.truncate(scope.locals);
        self.refs.retain(|r, _| !scope.registers.contains(r));
        self.iterators.retain(|r, _| !scope.registers.contains(r));
        let mut abandoned = vec![];
        self.elements.retain(|r, (element, _)| {
            if scope.registers.contains(r) {
                abandoned.push(element.mark);
                false
            } else {
                true
            }
        });
        it.abort_elements(abandoned);
        it.temps[scope.registers].fill(Value::Undef);
    }
    fn restore(&mut self, it: &mut Interp<'_>, scope: usize, marks: usize) {
        while self.marks.len() > marks {
            self.leave(it);
        }
        it.scopes.truncate(scope);
    }
    fn complete(&mut self, it: &mut Interp<'_>, mut completion: Completion) -> Option<RResult<Flow>> {
        while let Some(control) = self.controls.pop() {
            match control {
                Control::Target {
                    end,
                    next,
                    label,
                    block,
                    scope,
                    marks,
                } => {
                    self.restore(it, scope, marks);
                    match &completion {
                        Completion::Flow(Flow::Break(want))
                            if (want.is_none() && !block) || (want.is_some() && want == &label) =>
                        {
                            self.pc = end;
                            return None;
                        }
                        Completion::Flow(Flow::Continue(want))
                            if next.is_some() && (want.is_none() || want == &label) =>
                        {
                            self.pc = next.unwrap();
                            self.controls.push(Control::Target {
                                end,
                                next,
                                label,
                                block,
                                scope,
                                marks,
                            });
                            return None;
                        }
                        _ => {}
                    }
                }
                Control::Try {
                    catch,
                    finally,
                    end,
                    name,
                    mut phase,
                    pending,
                    scope,
                    marks,
                } => {
                    self.restore(it, scope, marks);
                    if phase == TryPhase::Body
                        && let Some(pc) = catch
                        && let Completion::Throw(text, value) = &completion
                        && !text.starts_with("out of fuel")
                    {
                        if let Some(name) = &name {
                            it.scopes.push((name.clone(), Slot::Val(value.clone())));
                        }
                        phase = TryPhase::Catch;
                        self.pc = pc;
                        self.controls.push(Control::Try {
                            catch,
                            finally,
                            end,
                            name,
                            phase,
                            pending: Completion::Normal,
                            scope,
                            marks,
                        });
                        return None;
                    }
                    if phase != TryPhase::Finally
                        && let Some(finally) = finally
                    {
                        self.pc = finally;
                        self.controls.push(Control::Try {
                            catch,
                            finally: Some(finally),
                            end,
                            name,
                            phase: TryPhase::Finally,
                            pending: completion,
                            scope,
                            marks,
                        });
                        return None;
                    }
                    if phase == TryPhase::Finally && matches!(completion, Completion::Normal) {
                        completion = pending;
                    }
                    if matches!(completion, Completion::Normal) {
                        self.pc = end;
                        return None;
                    }
                }
            }
        }
        while !self.marks.is_empty() {
            self.leave(it);
        }
        it.abort_elements(self.elements.drain().map(|(_, (element, _))| element.mark).collect());
        it.scopes.truncate(self.base);
        Some(match completion {
            Completion::Normal => Ok(Flow::Normal),
            Completion::Flow(flow) => Ok(flow),
            Completion::Throw(text, value) => {
                it.thrown = Some(value);
                it.thrown_text = crate::error_message(&text).into();
                Err(text)
            }
        })
    }
}
impl Interp<'_> {
    pub(super) fn exec_code(&mut self, code: Rc<Code>, entry: Option<Rc<crate::effects::Promise>>) -> RResult<Flow> {
        if let Some(result) = entry {
            let base = *self.frames.last().unwrap();
            return self.start_activation(code, base, result).map(Flow::Return);
        }
        let registers = code.registers;
        let previous = std::mem::replace(&mut self.temps, vec![Value::Undef; registers]);
        let mut frame = Frame::new(code, self.scopes.len());
        let result = self.step_frame(&mut frame, false).map(|r| match r {
            Run::Done(flow) => flow,
            Run::Await(..) => unreachable!(),
        });
        self.scopes.truncate(frame.base);
        self.temps = previous;
        result
    }
    fn start_activation(&mut self, code: Rc<Code>, base: usize, result: Rc<crate::effects::Promise>) -> RResult<Value> {
        let scopes = self.scopes.split_off(base);
        let parent = Context::take(self);
        self.scopes = scopes;
        self.frames = vec![0];
        self.temps = vec![Value::Undef; code.registers];
        let mut activation = Activation {
            frame: Frame::new(code, 0),
            context: None,
            result: result.clone(),
            waiting: None,
        };
        let outcome = self.step_frame(&mut activation.frame, true);
        self.finish_activation(activation, outcome);
        parent.put(self);
        Ok(Value::Promise(result))
    }
    pub(crate) fn resume_activation(&mut self, mut activation: Activation) -> RResult<()> {
        let parent = Context::take(self);
        let context = activation.context.take().expect("saved locals");
        context.put(self);
        self.depth = 1;
        let mut outcome = None;
        if let Some((source, to)) = activation.waiting.take() {
            match source.completed().expect("await reaction") {
                crate::effects::Settle::Ok(value) => self.temps[to] = value,
                crate::effects::Settle::Err(error) => {
                    if let Some(result) = activation
                        .frame
                        .complete(self, Completion::Throw(error.text, error.reason))
                    {
                        outcome = Some(result.map(Run::Done));
                    }
                }
                crate::effects::Settle::Pending => unreachable!(),
            }
        }
        let outcome = outcome.unwrap_or_else(|| self.step_frame(&mut activation.frame, true));
        // The frame is restored before another owner can run; no Rust stack survives an await.
        self.finish_activation(activation, outcome);
        parent.put(self);
        Ok(())
    }
    fn finish_activation(&mut self, mut activation: Activation, outcome: RResult<Run>) {
        let realm = self.realm.as_ref().expect("execution realm").clone();
        match outcome {
            Ok(Run::Await(value, to)) => {
                let source = realm.borrow_mut().promise(self.task, value);
                activation.waiting = Some((source, to));
                activation.context = Some(Context::take(self));
                realm.borrow_mut().wait_body(self.task, activation);
            }
            Ok(Run::Done(flow)) => {
                let value = match flow {
                    Flow::Return(v) => v,
                    _ => Value::Undef,
                };
                realm.borrow_mut().resolve(self.task, &activation.result, value);
            }
            Err(error) if error == SUSPEND => {}
            Err(text) => {
                let reason = self.caught(&text);
                realm.borrow_mut().settle(
                    &activation.result,
                    crate::effects::Settle::Err(crate::effects::Rejection { text, reason }),
                );
            }
        }
    }
    pub(crate) fn take_ui(&mut self) -> Ui {
        Ui {
            path: std::mem::take(&mut self.path),
            siblings: std::mem::take(&mut self.sibling_counts),
            hooks: std::mem::take(&mut self.hook_counts),
            seen: std::mem::take(&mut self.ui_seen),
            memo: std::mem::take(&mut self.memo_counts),
            updates: std::mem::take(&mut self.ui_updates),
        }
    }
    pub(crate) fn put_ui(&mut self, ui: Ui) {
        self.path = ui.path;
        self.sibling_counts = ui.siblings;
        self.hook_counts = ui.hooks;
        self.ui_seen = ui.seen;
        self.memo_counts = ui.memo;
        self.ui_updates = ui.updates;
    }
    fn step_frame(&mut self, frame: &mut Frame, suspend: bool) -> RResult<Run> {
        let code = frame.code.clone();
        loop {
            self.spend()?;
            let Some(op) = code.ops.get(frame.pc) else {
                return Ok(Run::Done(Flow::Normal));
            };
            frame.pc += 1;
            if suspend && let Op::Await(value, to) = op {
                return Ok(Run::Await(self.temps[*value].clone(), *to));
            }
            let result: RResult<Option<Completion>> = (|| {
                Ok(match op {
                    Op::Eval(expr, to) => {
                        let value = self.eval(expr)?;
                        self.temps[*to] = value;
                        None
                    }
                    Op::EvalLink(expr, to, end) => {
                        match self.eval(expr) {
                            Ok(value) => self.temps[*to] = value,
                            Err(e) if e == crate::interp::SHORT_CIRCUIT => frame.pc = *end,
                            Err(e) => return Err(e),
                        }
                        None
                    }
                    Op::Statement(stmt) => match self.exec_simple(std::slice::from_ref(stmt))? {
                        Flow::Normal => None,
                        flow => Some(Completion::Flow(flow)),
                    },
                    Op::Copy(from, to) => {
                        self.temps[*to] = self.temps[*from].clone();
                        None
                    }
                    Op::Clear(r) => {
                        self.temps[*r] = Value::Undef;
                        None
                    }
                    Op::Await(value, to) => {
                        let value = self.temps[*value].clone();
                        self.temps[*to] = self.resolve_promise(&value)?;
                        None
                    }
                    Op::Return(r) => Some(Completion::Flow(Flow::Return(self.temps[*r].clone()))),
                    Op::Break(label) => Some(Completion::Flow(Flow::Break(label.clone()))),
                    Op::Continue(label) => Some(Completion::Flow(Flow::Continue(label.clone()))),
                    Op::Enter { start, end } => {
                        frame.marks.push(Scope {
                            locals: self.scopes.len(),
                            registers: *start..*end,
                        });
                        None
                    }
                    Op::Leave => {
                        frame.leave(self);
                        None
                    }
                    Op::Jump(pc) => {
                        frame.pc = *pc;
                        None
                    }
                    Op::Branch(r, test, pc) => {
                        let v = &self.temps[*r];
                        let pass = match test {
                            Test::Falsy => !crate::jsvalue::truthy(v),
                            Test::Truthy => crate::jsvalue::truthy(v),
                            Test::Nullish => matches!(v, Value::None | Value::Undef),
                            Test::Present => !matches!(v, Value::None | Value::Undef),
                        };
                        if pass {
                            frame.pc = *pc;
                        }
                        None
                    }
                    Op::Target {
                        end,
                        next,
                        label,
                        block,
                    } => {
                        frame.controls.push(Control::Target {
                            end: *end,
                            next: *next,
                            label: label.clone(),
                            block: *block,
                            scope: self.scopes.len(),
                            marks: frame.marks.len(),
                        });
                        None
                    }
                    Op::EndTarget => {
                        assert!(matches!(frame.controls.pop(), Some(Control::Target { .. })));
                        None
                    }
                    Op::Try {
                        catch,
                        finally,
                        end,
                        name,
                    } => {
                        frame.controls.push(Control::Try {
                            catch: *catch,
                            finally: *finally,
                            end: *end,
                            name: name.clone(),
                            phase: TryPhase::Body,
                            pending: Completion::Normal,
                            scope: self.scopes.len(),
                            marks: frame.marks.len(),
                        });
                        None
                    }
                    Op::EndTry => Some(Completion::Normal),
                    Op::Fork(names) => {
                        for name in names {
                            if let Some((_, Slot::Cell(cell))) = self.scopes.iter_mut().rev().find(|(n, _)| n == name) {
                                let value = cell.value.borrow().clone();
                                *cell = Rc::new(VarCell::new(self.run, value));
                            }
                        }
                        None
                    }
                    Op::Call {
                        callee,
                        function,
                        receiver,
                    } => {
                        let (f, r) = self.eval_callee(callee)?;
                        self.temps[*function] = f;
                        self.temps[*receiver] = r.unwrap_or(Value::Undef);
                        None
                    }
                    Op::ArrayFill {
                        constructor,
                        count,
                        function,
                        receiver,
                        construct,
                    } => {
                        let ctor = self.temps[*constructor].clone();
                        let count = self.temps[*count].clone();
                        if matches!(&ctor,Value::Native(n) if &**n=="Array") {
                            self.temps[*function] = Value::Bound(Rc::new((
                                Value::Native(Rc::<str>::from("__arrayFill").into()),
                                Value::Undef,
                                vec![count],
                            )));
                            self.temps[*receiver] = Value::Undef;
                        } else {
                            let value = if *construct {
                                self.construct_value(&ctor, vec![count])?
                            } else {
                                self.call_value(&ctor, vec![count])?
                            };
                            self.temps[*function] = self.get_prop(&value, "fill")?;
                            self.temps[*receiver] = value;
                        }
                        None
                    }
                    Op::Invoke {
                        function,
                        receiver,
                        args,
                        result,
                        construct,
                    } => {
                        let f = self.temps[*function].clone();
                        let r = self.temps[*receiver].clone();
                        let mut values = vec![];
                        for (reg, spread) in args {
                            if *spread {
                                values.extend(iterate(&self.temps[*reg])?);
                            } else {
                                values.push(self.temps[*reg].clone());
                            }
                        }
                        self.temps[*result] = if *construct {
                            self.construct_value(&f, values)?
                        } else {
                            self.call_with_receiver(&f, r, values)?
                        };
                        None
                    }
                    Op::Reference { target, slot } => {
                        let reference = self.reference(target)?;
                        frame.refs.insert(*slot, reference);
                        None
                    }
                    Op::Read { slot, result } => {
                        let reference = frame.refs[slot].clone();
                        self.temps[*result] = self.read_reference(&reference)?;
                        None
                    }
                    Op::Write { slot, value } => {
                        let reference = frame.refs[slot].clone();
                        self.write_reference(&reference, self.temps[*value].clone())?;
                        None
                    }
                    Op::Forget(slot) => {
                        frame.refs.remove(slot);
                        None
                    }
                    Op::Iterate { source, slot } => {
                        frame
                            .iterators
                            .insert(*slot, IteratorState::new(self.temps[*source].clone())?);
                        None
                    }
                    Op::Next { slot, value, end } => {
                        match frame.iterators.get_mut(slot).expect("live iterator").next() {
                            Some(item) => self.temps[*value] = item,
                            None => frame.pc = *end,
                        };
                        None
                    }
                    Op::OpenElement {
                        node,
                        slot,
                        result,
                        end,
                    } => {
                        let mut capture = Capture::default();
                        if let Some(element) = self.element_open(node, &mut capture)? {
                            frame.elements.insert(*slot, (element, capture));
                        } else {
                            self.temps[*result] = Value::Frag(Rc::new(capture.items));
                            frame.pc = *end;
                        }
                        None
                    }
                    Op::CloseElement { slot, children, result } => {
                        if let Some((element, mut capture)) = frame.elements.remove(slot) {
                            self.element_close(element, children.map(|r| self.temps[r].clone()), &mut capture)?;
                            self.temps[*result] = Value::Frag(Rc::new(capture.items));
                        }
                        None
                    }
                    Op::Rest { source, keys, result } => {
                        let entries =
                            crate::jsbuiltins::object_global("Object.entries", &[self.temps[*source].clone()])
                                .expect("Object.entries")?;
                        let mut rest = Map::default();
                        for entry in iterate(&entries)? {
                            let pair = iterate(&entry)?;
                            if !keys
                                .iter()
                                .any(|k| crate::jsvalue::strict_eq(&self.temps[*k], &pair[0]))
                            {
                                rest.insert(Rc::from(pair[0].as_str().expect("object key")), pair[1].clone());
                            }
                        }
                        self.temps[*result] = Value::map(rest);
                        None
                    }
                    Op::EndIterator(slot) => {
                        frame.iterators.remove(slot);
                        None
                    }
                })
            })();
            match result {
                Ok(None) => {}
                Ok(Some(completion)) => {
                    if let Some(result) = frame.complete(self, completion) {
                        return result.map(Run::Done);
                    }
                }
                Err(e) if e == SUSPEND => return Err(e),
                Err(e) => {
                    let value = self.caught(&e);
                    if let Some(result) = frame.complete(self, Completion::Throw(e, value)) {
                        return result.map(Run::Done);
                    }
                }
            }
        }
    }
    pub(super) fn reference(&mut self, target: &Target) -> RResult<Reference> {
        Ok(match target {
            Target::Var(name) => Reference::Variable(name.clone()),
            Target::Temp(r) => Reference::Temporary(self.temps[*r].clone()),
            Target::Expr(e) => Reference::Temporary(self.eval(e)?),
            Target::Attr(base, name) => Reference::Property(
                self.target_value(base)?,
                ReferenceKey::Public(Value::Str(name.clone().into())),
            ),
            Target::Index(base, key) => {
                let base = self.target_value(base)?;
                let key = self.eval(key)?;
                let key = self.primitive(&key, true)?;
                Reference::Property(
                    base,
                    ReferenceKey::Public(Value::string(crate::jsvalue::to_string(&key))),
                )
            }
            Target::Private(base, name) => Reference::Property(
                self.target_value(base)?,
                ReferenceKey::Private(self.private_owner(name)?, name.clone()),
            ),
        })
    }
    pub(super) fn target_value(&mut self, target: &Target) -> RResult<Value> {
        let reference = self.reference(target)?;
        self.read_reference(&reference)
    }
    pub(super) fn read_reference(&mut self, reference: &Reference) -> RResult<Value> {
        match reference {
            Reference::Variable(name) => self.eval(&Expr::Var(name.clone())),
            Reference::Temporary(value) => Ok(value.clone()),
            Reference::Property(base, ReferenceKey::Public(key)) => self.get_dynamic(base, key),
            Reference::Property(base, ReferenceKey::Private(owner, name)) => self.get_private(base, owner, name),
        }
    }
    pub(super) fn write_reference(&mut self, reference: &Reference, value: Value) -> RResult<()> {
        match reference {
            Reference::Variable(name) => self.set_var(name, value),
            Reference::Temporary(_) => err("internal temporary is not assignable"),
            Reference::Property(base, ReferenceKey::Public(key)) => self.set_member(base, key, value),
            Reference::Property(base, ReferenceKey::Private(owner, name)) => self.set_private(base, owner, name, value),
        }
    }
}
