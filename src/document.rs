//! Document construction: element identity, computed styles, actions and lifecycle declarations.
//! Produces the shared structured token stream; `emit::Live` owns measurement and viewport layout.
use super::*;

/// Identity edits are reversible while an element is being constructed. A failed subtree
/// cannot reserve a sibling key or shift its fallback's position.
#[derive(Clone, Default)]
pub(super) struct Siblings {
    values: HashMap<Sym, usize>,
    edits: Vec<(Sym, Option<usize>)>,
}
impl Siblings {
    fn set(&mut self, key: Sym, value: usize) -> Option<usize> {
        let old = self.values.insert(key.clone(), value);
        self.edits.push((key, old));
        old
    }
    fn next(&mut self, name: &Sym) -> usize {
        let slot = self.values.get(name).copied().unwrap_or(0);
        self.set(name.clone(), slot + 1);
        slot
    }
    fn rollback(&mut self, to: usize) {
        while self.edits.len() > to {
            let (key, old) = self.edits.pop().unwrap();
            match old {
                Some(value) => {
                    self.values.insert(key, value);
                }
                None => {
                    self.values.remove(&key);
                }
            }
        }
    }
}

struct ComponentCall {
    function: Value,
    props: Vec<(Sym, Value)>,
}
#[derive(Clone, Copy)]
pub(super) struct DocumentMark {
    path: usize,
    ancestry: usize,
    siblings: usize,
    identity_edits: usize,
    hooks: usize,
    handlers: usize,
    effects: usize,
    events: usize,
    callbacks: usize,
    mounts: usize,
    seen: usize,
}
pub(super) struct Element {
    closes: usize,
    component: Option<ComponentCall>,
    pub(super) mark: DocumentMark,
}

/// The token box a computed style makes (its cap, overflow, marker; `pad` and `scroll` resolved
/// by the caller).
pub(crate) fn box_kind(
    style: &crate::style::Computed,
    pad: Option<u32>,
    scroll: Option<crate::emit::Scroll>,
) -> SpanKind {
    box_kind_graded(style, pad, scroll, None)
}

/// `box_kind`, with a positional alternative (a result under `result:nth-last`).
pub(crate) fn box_kind_graded(
    style: &crate::style::Computed,
    pad: Option<u32>,
    scroll: Option<crate::emit::Scroll>,
    graded: Option<Rc<crate::emit::Graded>>,
) -> SpanKind {
    use crate::{emit::Drop, style::Overflow};
    SpanKind::Box {
        max_tokens: style.max_tokens,
        tok_size: style.tok_size,
        clip: match style.overflow {
            Overflow::ClipStart => crate::emit::Clip::Start,
            Overflow::ClipMiddle => crate::emit::Clip::Middle,
            Overflow::Elide => crate::emit::Clip::Elide,
            _ => crate::emit::Clip::End,
        },
        drop_step: style.drop_step,
        graded,
        pad,
        more: style.more.clone(),
        drop: match style.overflow {
            Overflow::DropStart => Drop::Start,
            Overflow::DropEnd => Drop::End,
            Overflow::DropChildren => Drop::Children,
            _ => Drop::None,
        },
        scroll,
    }
}

impl<'p> Interp<'p> {
    fn enter_element(&mut self, name: &Sym, props: &[(Sym, Value)]) -> RResult<()> {
        let siblings = self.sibling_counts.last_mut().unwrap();
        let slot = siblings.next(name);
        let segment = if let Some((_, value)) = props.iter().find(|(n, _)| &**n == "key") {
            if !matches!(value, Value::Str(_) | Value::Int(_) | Value::Float(_)) {
                return err("element key must be a string or number");
            }
            let key = crate::jsvalue::to_string(value);
            if siblings.set(Rc::from(format!("\0key:{key}")), 1).is_some() {
                return err(format!("duplicate sibling key `{key}`"));
            }
            format!("{name}#{}:{key}", key.len())
        } else {
            format!("{name}:{slot}")
        };
        self.path.push(segment);
        if self.descendants {
            self.ancestry.push((name.clone(), Rc::new(props.to_vec())));
        }
        self.sibling_counts.push(Siblings::default());
        self.hook_counts.push(0);
        Ok(())
    }

    // ---------------- markup ----------------

    pub(super) fn document_mark(&self) -> DocumentMark {
        DocumentMark {
            path: self.path.len(),
            ancestry: self.ancestry.len(),
            siblings: self.sibling_counts.len(),
            identity_edits: self.sibling_counts.last().unwrap().edits.len(),
            hooks: self.hook_counts.len(),
            handlers: self.handlers.len(),
            effects: self.effect_decls.len(),
            events: self.event_decls.len(),
            callbacks: self.callback_decls.len(),
            mounts: self.mount_decls.len(),
            seen: self.ui_seen.len(),
        }
    }
    fn leave_document(&mut self, mark: DocumentMark) {
        self.path.truncate(mark.path);
        self.ancestry.truncate(mark.ancestry);
        self.sibling_counts.truncate(mark.siblings);
        self.hook_counts.truncate(mark.hooks);
    }
    pub(super) fn abort_document(&mut self, mark: DocumentMark) {
        self.leave_document(mark);
        self.sibling_counts.last_mut().unwrap().rollback(mark.identity_edits);
        self.handlers.truncate(mark.handlers);
        self.effect_decls.truncate(mark.effects);
        self.event_decls.truncate(mark.events);
        self.callback_decls.truncate(mark.callbacks);
        self.mount_decls.truncate(mark.mounts);
        self.ui_seen.truncate(mark.seen);
    }
    pub(super) fn abort_elements(&mut self, mut marks: Vec<DocumentMark>) {
        marks.sort_by_key(|mark| std::cmp::Reverse(mark.path));
        for mark in marks {
            self.abort_document(mark);
        }
    }
    /// Shared computed-style lowering. Native boxes and component wrappers use the same box.
    fn open_style_box(
        &mut self,
        name: &str,
        props: &[(Sym, Value)],
        style: &crate::style::Computed,
        intrinsic: bool,
        out: &mut dyn Out,
    ) -> RResult<usize> {
        if self.initializing > 0 && style.scrolls() {
            return err("scroll viewports belong in component functions, not module initializers");
        }
        if !intrinsic && !style.boxed() {
            return Ok(0);
        }
        let label: Rc<str> = props
            .iter()
            .find(|(n, _)| &**n == "label")
            .or_else(|| props.iter().find(|(n, _)| &**n == "id"))
            .map(|(_, v)| Rc::from(crate::jsvalue::to_string(v)))
            .unwrap_or_else(|| Rc::from(if intrinsic { "" } else { name }));
        let scroll = if style.scrolls() {
            let key: Sym = Rc::from(format!("{}/$scroll", self.path.join("/")));
            let offset = self
                .ui_value(&key)
                .and_then(|v| match v {
                    Value::Int(n) => usize::try_from(n).ok(),
                    _ => None,
                })
                .unwrap_or(0);
            self.ui_seen.insert(key.clone(), Value::Int(offset as i64));
            let callback = props
                .iter()
                .find(|(n, _)| &**n == "onScroll")
                .map_or(Value::Undef, |(_, v)| v.clone());
            let handler = self.handlers.len();
            self.handlers.push(Value::Bound(Rc::new((
                Value::Native(Rc::<str>::from("\0scroll").into()),
                Value::Undef,
                vec![Value::Setter(key), callback],
            ))));
            let target =
                if label.is_empty() || (!intrinsic && !props.iter().any(|(n, _)| matches!(&**n, "label" | "id"))) {
                    self.path.join("/")
                } else {
                    label.to_string()
                };
            let action = Rc::new(SpanSpec {
                label: Rc::from("scroll"),
                kind: SpanKind::Affordance,
                meta: serde_json::json!({"name":"scroll","target":target,"params":["by"],"required_params":["by"],"object_param":false,"handler":handler,"props":{},"args_schema":{"type":"object","properties":{"by":{"type":"integer"}},"required":["by"]}}),
            });
            Some(crate::emit::Scroll { offset, action })
        } else {
            None
        };
        let pad = style.pad.as_deref().map(|s| self.special_id(s)).transpose()?;
        out.open(Rc::new(SpanSpec {
            label,
            kind: box_kind(style, pad, scroll),
            meta: if intrinsic {
                serde_json::json!({})
            } else {
                serde_json::json!({"component":name})
            },
        }));
        Ok(1)
    }

    fn attr_vals(&mut self, attrs: &[Attr]) -> RResult<Vec<(Sym, Value)>> {
        let mut v: Vec<(Sym, Value)> = Vec::with_capacity(attrs.len());
        // As in JSX, a later attribute of the same name (spread or written) wins.
        let set = |v: &mut Vec<(Sym, Value)>, name: Sym, val: Value| match v.iter_mut().find(|(n, _)| *n == name) {
            Some(slot) => slot.1 = val,
            None => v.push((name, val)),
        };
        for a in attrs {
            let val = match &a.value {
                AttrVal::Str(s) => Value::Str(s.into()),
                AttrVal::Expr(e) => self.eval(e)?,
                AttrVal::Bare => Value::Bool(true),
                AttrVal::Spread(e) => {
                    match self.eval(e)? {
                        Value::Map(m) => {
                            for (k, x) in m.read().iter() {
                                set(&mut v, k.clone(), x.clone());
                            }
                        }
                        Value::None | Value::Undef => {}
                        other => {
                            return err(format!(
                                "TypeError: {{...x}} in an element needs an object, got {}",
                                crate::jsvalue::type_of(&other)
                            ));
                        }
                    }
                    continue;
                }
            };
            set(&mut v, a.name.clone(), val);
        }
        Ok(v)
    }

    pub(super) fn nodes(&mut self, nodes: &[Node], out: &mut dyn Out) -> RResult<Flow> {
        let base = self.scopes.len();
        let mark = self.document_mark();
        let r = self.nodes_inner(nodes, out);
        self.scopes.truncate(base);
        if r.is_err() {
            self.abort_document(mark);
        }
        r
    }

    fn nodes_inner(&mut self, nodes: &[Node], out: &mut dyn Out) -> RResult<Flow> {
        for n in nodes {
            match n {
                Node::Text(t) => out.text_rc(t),
                Node::Child(e) => match self.eval(e)? {
                    // Each element of an array child is one item (for drop-start/drop-end boxes).
                    Value::List(l) => {
                        for x in l.read().iter() {
                            out.segment();
                            emit_child(x, out)?;
                        }
                    }
                    v => emit_child(&v, out)?,
                },
                Node::Call { children, .. } | Node::Builtin { children, .. } => {
                    let Some(element) = self.element_open(n, out)? else {
                        continue;
                    };
                    if element.component.is_some() {
                        let child = if children.is_empty() {
                            None
                        } else {
                            let mut cap = Capture::default();
                            self.nodes(children, &mut cap)?;
                            Some(Value::Frag(Rc::new(cap.items)))
                        };
                        self.element_close(element, child, out)?;
                    } else {
                        self.nodes(children, out)?;
                        self.element_close(element, None, out)?;
                    }
                }
                Node::Special { text, id } => match id.get() {
                    Some(i) => out.tok(i),
                    None => return err(format!("special token {:?} not bound", text)),
                },
                Node::DynSpecial(e) => {
                    let v = self.eval(e)?;
                    let s = v.as_str().ok_or("<sp t={..}> needs a string")?;
                    match self.specials.get(s).cloned() {
                        Some(Value::Int(i)) => out.tok(i as u32),
                        _ => {
                            return err(format!("{:?} is not a special token of this tokenizer", s));
                        }
                    }
                }
                Node::TokId(e) => match self.eval(e)? {
                    Value::Int(i) => out.tok(i as u32),
                    _ => return err("<tok id> needs an int"),
                },
            }
        }
        Ok(Flow::Normal)
    }

    /// Open an element before evaluating children. Both direct rendering and suspended
    /// markup use this path, including visibility, attributes and affordance registration.
    pub(super) fn element_open(&mut self, node: &Node, out: &mut dyn Out) -> RResult<Option<Element>> {
        let mark = self.document_mark();
        let result = self.element_open_inner(node, out, mark);
        if result.is_err() {
            self.abort_document(mark);
        }
        result
    }
    fn element_open_inner(&mut self, node: &Node, out: &mut dyn Out, mark: DocumentMark) -> RResult<Option<Element>> {
        match node {
            Node::Call { name, attrs, .. } => {
                let mut path = name.split('.');
                let mut f = self.lookup(path.next().unwrap());
                for property in path {
                    f = self.get_prop(&f, property)?;
                }
                if !f.is_callable() {
                    return err(format!(
                        "unknown component <{}>: expected a function, got {}",
                        name,
                        f.type_name()
                    ));
                }
                let attrs = self.attr_vals(attrs)?;
                let style = crate::style::compute(&self.prog.styles, name, &attrs, false, &self.ancestry)?;
                self.enter_element(name, &attrs)?;
                if style.hidden {
                    out.open(Rc::new(SpanSpec {
                        label: Rc::from(""),
                        kind: SpanKind::Hidden,
                        meta: serde_json::Value::Null,
                    }));
                }
                let closes = usize::from(style.hidden) + self.open_style_box(name, &attrs, &style, false, out)?;
                Ok(Some(Element {
                    closes,
                    component: Some(ComponentCall {
                        function: f,
                        props: attrs,
                    }),
                    mark,
                }))
            }
            Node::Builtin { tag, attrs, children } => {
                let mut attrs = self.attr_vals(attrs)?;
                if self.initializing > 0
                    && attrs.iter().any(|(name, value)| {
                        name.len() > 2
                            && name.starts_with("on")
                            && name.as_bytes()[2].is_ascii_uppercase()
                            && value.is_callable()
                    })
                {
                    return err("interactive JSX belongs in component functions, not module initializers");
                }
                let style = crate::style::compute(&self.prog.styles, tag, &attrs, true, &self.ancestry)?;
                self.enter_element(tag, &attrs)?;
                if style.hidden {
                    out.open(Rc::new(SpanSpec {
                        label: Rc::from(""),
                        kind: SpanKind::Hidden,
                        meta: serde_json::Value::Null,
                    }));
                }
                let hidden = usize::from(style.hidden);
                // <Mount key app props model machine(s) onX...>: a child instance. Zero tokens: a
                // `mount` span declares it; each onX callback is a harness callback
                // `mount:<key>:onX` the child invokes as props.onX(...).
                if &**tag == "mount" {
                    if self.initializing > 0 {
                        return err("mounts belong in component functions, not module initializers");
                    }
                    let get = |k: &str| attrs.iter().find(|(n, _)| &**n == k).map(|(_, v)| v.clone());
                    let key = get("key")
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .ok_or("<Mount> needs key (a string naming the child)")?;
                    // The child: exactly one component element, recorded (not rendered here).
                    let mut elements = children
                        .iter()
                        .filter(|c| !matches!(c, Node::Text(t) if t.trim().is_empty()));
                    let (name, cattrs, cchildren) = match (elements.next(), elements.next()) {
                        (Some(Node::Call { name, attrs, children }), None) => (name, attrs, children),
                        _ => {
                            return err(format!(
                                "<Mount key={key:?}> takes exactly one component element: <Mount ...><App .../></Mount>"
                            ));
                        }
                    };
                    if cchildren
                        .iter()
                        .any(|c| !matches!(c, Node::Text(t) if t.trim().is_empty()))
                    {
                        return err(format!(
                            "<Mount key={key:?}>: <{name}> runs as its own instance, so it takes no children (pass data as props)"
                        ));
                    }
                    let f = self
                        .module_function(name)
                        .ok_or_else(|| format!("<Mount key={key:?}>: unknown component <{name}> (import its file)"))?;
                    if !std::path::Path::new(f.loc.file.as_ref()).is_absolute() {
                        return err("<Mount> needs a file-backed entry; render source-loaded components directly");
                    }
                    let machines: Vec<serde_json::Value> = match (get("machine"), get("machines")) {
                        (Some(Value::Str(m)), _) => vec![serde_json::json!(&*m)],
                        (_, Some(Value::List(l))) => l.read().iter().map(|v| v.to_json()).collect(),
                        _ => vec![],
                    };
                    // Props cross the instance boundary as JSON; a function prop named onX is a
                    // callback: calling it in the child is an event here, which runs it.
                    let mut props = serde_json::Map::new();
                    let mut callbacks = vec![];
                    for (n, v) in self.attr_vals(cattrs)? {
                        if n.len() > 2 && n.starts_with("on") && n.as_bytes()[2].is_ascii_uppercase() && v.is_callable()
                        {
                            let handler = self.handlers.len();
                            self.handlers.push(v);
                            let label = format!("mount:{key}:{n}");
                            self.callback_decls.push(Rc::new(SpanSpec { label: Rc::from(label.as_str()), kind: SpanKind::Affordance,
                                    meta: serde_json::json!({"name": label, "target": label, "params": [], "required_params": [], "object_param": false, "handler": handler, "props": {}}) }));
                            callbacks.push(n.to_string());
                        } else if let Some(kind) = not_json(&v) {
                            return err(format!(
                                "<Mount key={key:?}>: prop {n} of <{name}> is {kind}; props cross to the child instance as JSON, and only onX functions (callbacks) are allowed"
                            ));
                        } else {
                            props.insert(n.to_string(), v.to_wire());
                        }
                    }
                    let meta = serde_json::json!({"key": key, "app": &*f.loc.file, "entry": &*f.name, "props": props, "model": get("model").map(|v| v.to_json()), "machines": machines, "callbacks": callbacks});
                    self.mount_decls.push(meta.clone());
                    out.open(Rc::new(SpanSpec {
                        label: Rc::from("mount"),
                        kind: SpanKind::Label,
                        meta,
                    }));
                    out.close()?;
                    for _ in 0..hidden {
                        out.close()?;
                    }
                    self.leave_document(mark);
                    return Ok(None);
                }
                // TSX: onX={handler} props become affordances "x" on this element's tokens; the
                // Destructured fields give parameter names, not JSON primitive types.
                let mut actions: Vec<(String, Value)> = vec![];
                attrs.retain(|(n, v)| {
                    if n.len() > 2 && n.starts_with("on") && n.as_bytes()[2].is_ascii_uppercase() && v.is_callable() {
                        if &**n != "onScroll" || !style.scrolls() {
                            actions.push((n[2..].to_lowercase(), v.clone()));
                        }
                        &**n == "onScroll" && style.scrolls()
                    } else {
                        true
                    }
                });
                if !actions.is_empty() && !matches!(&**tag, "box" | "span" | "aff") {
                    return err(format!(
                        "on* actions are supported on <box>, <span>, <aff>, not <{}>",
                        tag
                    ));
                }
                let mut n_actions = 0;
                if !actions.is_empty() {
                    let target = attrs
                        .iter()
                        .find(|(n, _)| &**n == "label")
                        .map(|(_, v)| crate::jsvalue::to_string(v))
                        .unwrap_or_default();
                    // scalar props travel with the affordance (harness callbacks read after=, every=, event=)
                    let props: serde_json::Map<String, serde_json::Value> = attrs
                        .iter()
                        .filter(|(n, v)| {
                            matches!(v, Value::Int(_) | Value::Float(_) | Value::Str(_) | Value::Bool(_))
                                || (&**n == "default" && matches!(v, Value::Map(_)))
                        })
                        .map(|(n, v)| (n.to_string(), v.to_json()))
                        .collect();
                    for (name, h) in &actions {
                        // An arrow or a declared function (`onRun={lookup}`): its parameters name the arguments.
                        let (pprops, pparams) = match h {
                            Value::Lambda(l, _) => (l.props.as_ref(), &l.params[..]),
                            Value::Func(f) => (f.props.as_ref(), &f.params[..]),
                            _ => (None, &[][..]),
                        };
                        let params: Vec<serde_json::Value> = match pprops {
                            Some(pp) => pp.iter().map(|p| serde_json::Value::from(p.name.to_string())).collect(),
                            None => pparams
                                .iter()
                                .map(|p| serde_json::Value::from(p.name.to_string()))
                                .collect(),
                        };
                        let required: Vec<serde_json::Value> = pprops
                            .into_iter()
                            .flatten()
                            .filter(|p| p.default.is_none())
                            .map(|p| serde_json::Value::from(p.name.to_string()))
                            .collect();
                        let object_param = pprops.is_none() && !pparams.is_empty();
                        let handler = self.handlers.len();
                        self.handlers.push(h.clone());
                        let mut meta = serde_json::json!({"name": name, "target": target, "params": params, "required_params": required, "object_param": object_param, "handler": handler, "props": props});
                        if let Some((_, schema)) = attrs.iter().find(|(n, _)| &**n == "args_schema") {
                            if !matches!(schema, Value::Map(_)) {
                                return err("args_schema must be a JSON schema object");
                            }
                            meta["args_schema"] = schema.to_json();
                        }
                        let spec = Rc::new(SpanSpec {
                            label: Rc::from(name.as_str()),
                            kind: SpanKind::Affordance,
                            meta,
                        });
                        if crate::is_harness_callback(name) {
                            self.callback_decls.push(spec);
                        } else {
                            out.open(spec);
                            n_actions += 1;
                        }
                    }
                }
                // A protocol turn a template writes, and the run of history turns:
                // transparent label spans (no encoding
                // boundary) that sheets select, cap and drop; props travel as meta. The label
                // is outside the element's style box, so a drop box's items are the element's
                // own children: `history { max-tokens: 80win; overflow: drop-start }` drops turns.
                if matches!(&**tag, "turn" | "history") {
                    let mut meta: serde_json::Map<String, serde_json::Value> = attrs
                        .iter()
                        .filter(|(_, v)| !v.is_callable() && !matches!(v, Value::Frag(_) | Value::Promise(_)))
                        .map(|(n, v)| (n.to_string(), v.to_json()))
                        .collect();
                    // Under positional result rules, layout counts this history's results.
                    if &**tag == "history" && self.prog.styles.iter().any(|r| r.sel.nth_last.is_some()) {
                        meta.insert("graded".into(), true.into());
                    }
                    out.open(Rc::new(SpanSpec {
                        label: Rc::from(&**tag),
                        kind: SpanKind::Label,
                        meta: if meta.is_empty() {
                            serde_json::Value::Null
                        } else {
                            serde_json::Value::Object(meta)
                        },
                    }));
                    let styled = self.open_style_box(tag, &attrs, &style, false, out)?;
                    return Ok(Some(Element {
                        closes: 1 + hidden + styled + n_actions,
                        component: None,
                        mark,
                    }));
                }
                let get = |k: &str| attrs.iter().find(|(n, _)| &**n == k).map(|(_, v)| v.clone());
                // A picture or a clip laid out by its model's size rule: `max-tokens` is the
                // size it is given, not a box that clips its placeholders.
                let sized = matches!(&**tag, "img" | "audio" | "video") && style.tok_size.is_none();
                let styled = if sized {
                    0
                } else {
                    self.open_style_box(tag, &attrs, &style, &**tag == "box", out)?
                };
                let label: Rc<str> = get("label")
                    .or_else(|| get("name"))
                    .map(|v| Rc::from(crate::jsvalue::to_string(&v)))
                    .unwrap_or_else(|| Rc::from(""));
                let spec = match &**tag {
                    "text" => {
                        // <text>..</text>: plain grouping, no span
                        return Ok(Some(Element {
                            closes: hidden + styled + n_actions,
                            component: None,
                            mark,
                        }));
                    }
                    // <img tok_size=N t="<|image_pad|>" src=.../>: N placeholder tokens, and the
                    // image the model request sends for them. <audio> and <video> are the same
                    // for a clip: the placeholder is the model's, the source says what it is.
                    "img" | "audio" | "video" => {
                        let t = get("t").ok_or_else(|| format!("<{tag}> needs t= (placeholder token)"))?;
                        let id = self.special_id(t.as_str().unwrap_or(""))?;
                        let src = get("src").and_then(|v| v.as_str().map(String::from));
                        let mut meta = serde_json::json!({"img": true});
                        if &**tag != "img" {
                            meta["media"] = serde_json::json!(&**tag);
                        }
                        let cap = style.max_tokens.and_then(|size| match size {
                            crate::emit::Size::Tokens(n) => Some(n),
                            crate::emit::Size::Window(share) => self
                                .window
                                .map(|w| (w.tokens() as u64 * share as u64 / 10_000) as usize),
                        });
                        // The tokens it takes. The author may state them (`tok_size`: that many
                        // placeholders, the file as it is). Otherwise the model's size rule for
                        // this kind of file gives them, at the size the style allows: the rule
                        // the element is given (`metrics`), or the tokenizer's (`Encoder.media`).
                        // With no rule, or a file the rule cannot measure, the element is not
                        // laid out at a guess: `Err` says why, and its alt text stands in.
                        // A video laid out by its frames is not a run of one token: its ids.
                        let mut video_ids: Option<Vec<u32>> = None;
                        // A bug in a size rule or its tokens (not a file that is missing or too big
                        // for its max-tokens) fails the render: alt text would hide it.
                        let mut broken: Option<String> = None;
                        let placed: Result<usize, String> = if let Some(n) = style.tok_size {
                            Ok(n)
                        } else {
                            let rule = match get("metrics").filter(|m| !matches!(m, Value::Undef | Value::None)) {
                                Some(given) => Some(crate::media::Metrics::parse(&given.to_json())?),
                                None => self.encoder.and_then(|enc| enc.media.of(tag)),
                            };
                            let what = match &**tag {
                                "img" => "a picture",
                                "audio" => "a sound clip",
                                _ => "a video",
                            };
                            match (rule, src.as_deref()) {
                                (None, _) => Err(format!("this model's tokenizer has no size rule for {what}")),
                                (_, None) => Err(format!("<{tag}> has no src")),
                                (
                                    Some(
                                        rule @ (crate::media::Metrics::Grid { .. }
                                        | crate::media::Metrics::Fixed { .. }
                                        | crate::media::Metrics::Shape { .. }
                                        | crate::media::Metrics::Patches { .. }
                                        | crate::media::Metrics::Lines { .. }
                                        | crate::media::Metrics::OvisGrid { .. }),
                                    ),
                                    Some(src),
                                )
                                    if &**tag == "img" =>
                                {
                                    crate::media::natural_size(src).and_then(|natural| {
                                        let placed = crate::media::fit(&rule, natural, cap, style.min_tokens, style.object_fit)
                                            .ok_or_else(|| "it does not fit its max-tokens".to_string())?;
                                        meta["natural"] = serde_json::json!([natural.0, natural.1]);
                                        if let Some((w, h)) = placed.size {
                                            meta["size"] = serde_json::json!([w, h]);
                                        }
                                        Ok(placed.tokens)
                                    })
                                }
                                (Some(rule @ crate::media::Metrics::Rate { .. }), Some(src)) if &**tag == "audio" => {
                                    crate::media::clip_length_ms(src).and_then(|length| {
                                        let placed = crate::media::fit_clip(&rule, length, cap)
                                            .ok_or_else(|| "it does not fit its max-tokens".to_string())?;
                                        if let Some(keep) = placed.keep_ms {
                                            meta["clip"] = serde_json::json!({"ms": keep, "from_end": style.overflow == crate::style::Overflow::ClipStart});
                                        }
                                        Ok(placed.tokens)
                                    })
                                }
                                // A video as timed frames: nothing to resize (the file goes as it
                                // is), so the count is the processor's and a cap under it means
                                // the video is not shown.
                                (Some(rule @ crate::media::Metrics::Frames { .. }), Some(src)) if &**tag == "video" => {
                                    crate::media::video_info(src).and_then(|video| {
                                        let placed = crate::media::fit_video(&rule, video)
                                            .ok_or_else(|| "its processor would not take this video".to_string())?;
                                        let crate::media::Metrics::Frames { open, close, .. } = &rule else { unreachable!() };
                                        let (open, close) = (self.special_id(open)?, self.special_id(close)?);
                                        let enc = self.encoder.ok_or("a video is laid out with its model's tokenizer")?;
                                        let mut ids = vec![];
                                        for stamp in &placed.stamps {
                                            let stamp = enc.tok.encode(stamp.as_str(), false).map_err(|e| e.to_string())?;
                                            ids.extend_from_slice(stamp.get_ids());
                                            ids.push(open);
                                            ids.extend(std::iter::repeat_n(id, placed.cells));
                                            ids.push(close);
                                        }
                                        if cap.is_some_and(|cap| ids.len() > cap) {
                                            return Err(format!("it takes {} tokens, over its max-tokens (show it as frames to size it)", ids.len()));
                                        }
                                        video_ids = Some(ids);
                                        Ok(0)
                                    })
                                }
                                // A tiled picture: markers and runs of cells, the file sent at the
                                // size laid out.
                                (
                                    Some(
                                        rule @ (crate::media::Metrics::Tiles { .. }
                                        | crate::media::Metrics::Rows { .. }
                                        | crate::media::Metrics::DynTiles { .. }
                                        | crate::media::Metrics::Canvas { .. }),
                                    ),
                                    Some(src),
                                )
                                    if &**tag == "img" =>
                                {
                                    crate::media::natural_size(src).and_then(|natural| {
                                        let laid = match &rule {
                                            crate::media::Metrics::Rows { .. } => crate::media::rows_layout(&rule, natural, cap),
                                            crate::media::Metrics::DynTiles { .. } => crate::media::dyntiles_layout(&rule, natural, cap),
                                            crate::media::Metrics::Canvas { .. } => crate::media::canvas_layout(&rule, natural, cap),
                                            _ => crate::media::tiles_layout(&rule, natural, cap),
                                        };
                                        let (pieces, size) = laid
                                            .ok_or_else(|| "it does not fit its max-tokens".to_string())?;
                                        let mut ids = vec![];
                                        for piece in &pieces {
                                            match piece {
                                                crate::media::Piece::Marker(name) => ids.push(self.special_id(name)?),
                                                crate::media::Piece::Cells(n) => ids.extend(std::iter::repeat_n(id, *n)),
                                                crate::media::Piece::Run(name, n) => {
                                                    let run = self.special_id(name)?;
                                                    ids.extend(std::iter::repeat_n(run, *n))
                                                }
                                            }
                                        }
                                        meta["natural"] = serde_json::json!([natural.0, natural.1]);
                                        if let Some((w, h)) = size {
                                            meta["size"] = serde_json::json!([w, h]);
                                        }
                                        video_ids = Some(ids);
                                        Ok(0)
                                    })
                                }
                                (Some(rule @ crate::media::Metrics::Named { .. }), Some(src)) if &**tag == "img" => {
                                    crate::media::natural_size(src).and_then(|natural| {
                                        let mut fail = |why: String| {
                                            broken = Some(why.clone());
                                            why
                                        };
                                        let pieces = crate::media::named_layout(&rule, natural, cap)
                                            .map_err(&mut fail)?
                                            .ok_or_else(|| "it does not fit its max-tokens".to_string())?;
                                        let enc = self
                                            .encoder
                                            .ok_or_else(|| fail("a picture is laid out with its model's tokenizer".into()))?;
                                        let mut ids = vec![];
                                        for piece in &pieces {
                                            match piece {
                                                crate::media::Piece::Cells(n) => ids.extend(std::iter::repeat_n(id, *n)),
                                                crate::media::Piece::Marker(name) | crate::media::Piece::Run(name, _) => {
                                                    let n = crate::media::piece_tokens(piece);
                                                    if let Some(text) = name.strip_prefix("text:") {
                                                        for _ in 0..n {
                                                            let encoded = enc.tok.encode(text, false).map_err(|e| fail(e.to_string()))?;
                                                            ids.extend_from_slice(encoded.get_ids());
                                                        }
                                                    } else {
                                                        // An added token, or one of the vocabulary's own (a line end, `Ċ`).
                                                        let token = self.special_id(name).ok().or_else(|| enc.token_id(name)).ok_or_else(|| {
                                                            fail(format!("the picture's marker {name:?} is not a token of this model"))
                                                        })?;
                                                        ids.extend(std::iter::repeat_n(token, n));
                                                    }
                                                }
                                            }
                                        }
                                        meta["natural"] = serde_json::json!([natural.0, natural.1]);
                                        video_ids = Some(ids);
                                        Ok(0)
                                    })
                                }
                                (Some(_), Some(_)) => Err(format!("this model's size rule is not one for {what}")),
                            }
                        };
                        if let Some(why) = broken {
                            return err(format!("<{tag}>: {why}"));
                        }
                        match placed {
                            Ok(n) => {
                                let ids = video_ids.unwrap_or_else(|| vec![id; n]);
                                meta["tok_size"] = serde_json::json!(ids.len());
                                // The token a server that expands the file itself is sent.
                                meta["placeholder"] = serde_json::json!(id);
                                if let Some(src) = src {
                                    meta["src"] = serde_json::Value::String(src);
                                }
                                out.open(Rc::new(SpanSpec {
                                    label,
                                    kind: SpanKind::Label,
                                    meta,
                                }));
                                for id in ids {
                                    out.tok(id);
                                }
                                out.close()?;
                            }
                            // Not shown: its alt text stands in, as on a page, and the span says
                            // why (a run reports it). Its file is not sent.
                            Err(why) => {
                                let alt = get("alt")
                                    .and_then(|v| v.as_str().map(String::from))
                                    .unwrap_or_else(|| {
                                        match &**tag {
                                            "img" => "[picture not shown]",
                                            "audio" => "[sound clip not played]",
                                            _ => "[video not shown]",
                                        }
                                        .to_string()
                                    });
                                out.open(Rc::new(SpanSpec {
                                    label,
                                    kind: SpanKind::Label,
                                    meta: serde_json::json!({"not_shown": why, "media": &**tag}),
                                }));
                                out.text(&alt);
                                out.close()?;
                            }
                        }
                        for _ in 0..hidden + styled + n_actions {
                            out.close()?;
                        }
                        self.leave_document(mark);
                        return Ok(None);
                    }
                    // Props other than label/name travel as meta (a template adapter's `protocol` span).
                    "span" => {
                        let extra: serde_json::Map<String, serde_json::Value> = attrs
                            .iter()
                            .filter(|(n, v)| {
                                !matches!(&**n, "label" | "name")
                                    && !v.is_callable()
                                    && !matches!(v, Value::Frag(_) | Value::Promise(_))
                            })
                            .map(|(n, v)| (n.to_string(), v.to_json()))
                            .collect();
                        SpanSpec {
                            label,
                            kind: SpanKind::Label,
                            meta: if extra.is_empty() {
                                serde_json::Value::Null
                            } else {
                                serde_json::Value::Object(extra)
                            },
                        }
                    }
                    "aff" => {
                        let mut meta = serde_json::Map::new();
                        for (k, v) in &attrs {
                            // Keys the runtime writes (a handler's index, an image, a scroll's layout).
                            if matches!(&**k, "handler" | "img" | "scroll_max" | "projected_tool") {
                                return err(format!(
                                    "<aff {k}=>: `{k}` is set by the runtime, not by an attribute (a handler is an on* prop)"
                                ));
                            }
                            meta.insert(k.to_string(), v.to_json());
                        }
                        SpanSpec {
                            label,
                            kind: SpanKind::Affordance,
                            meta: serde_json::Value::Object(meta),
                        }
                    }
                    "box" => {
                        return Ok(Some(Element {
                            closes: hidden + styled + n_actions,
                            component: None,
                            mark,
                        }));
                    }
                    other => return err(format!("unknown builtin <{}>", other)),
                };
                out.open(Rc::new(spec));
                Ok(Some(Element {
                    closes: 1 + hidden + styled + n_actions,
                    component: None,
                    mark,
                }))
            }
            _ => unreachable!("element header"),
        }
    }
    pub(super) fn element_close(
        &mut self,
        element: Element,
        children: Option<Value>,
        out: &mut dyn Out,
    ) -> RResult<()> {
        let mark = element.mark;
        let result = self.element_close_inner(element, children, out);
        if result.is_err() {
            self.abort_document(mark);
        }
        result
    }
    fn element_close_inner(&mut self, element: Element, children: Option<Value>, out: &mut dyn Out) -> RResult<()> {
        if let Some(ComponentCall {
            function: f,
            props: attrs,
        }) = element.component
        {
            let mut props: Map = attrs.into_iter().collect();
            if let Some(children) = children {
                props.insert(Rc::from("children"), children);
            }
            self.template_actions.push(vec![]);
            let value = self.call_value(&f, vec![Value::map(props)]);
            let actions = self.template_actions.pop().unwrap_or_default();
            let value = value?;
            match &value {
                Value::List(l) if element.closes > 0 => {
                    for x in l.read().iter() {
                        out.segment();
                        emit_child(x, out)?;
                    }
                }
                _ => emit_child(&value, out)?,
            }
            // A template app offers its screen's actions however it wrote the screen: those its
            // output does not carry follow it, zero-width (`turns::screen_actions`).
            let mut carried = std::collections::HashSet::new();
            crate::turns::handlers_in(&value, &mut carried);
            let missing: Vec<_> = actions
                .into_iter()
                .filter(|s| carried.insert(s.meta["handler"].as_u64().unwrap_or(u64::MAX)))
                .collect();
            if !missing.is_empty() {
                let marks = Value::Frag(Rc::new(vec![Item::Tokens(
                    Rc::from(Vec::<u32>::new()),
                    Rc::from(missing),
                )]));
                emit_child(&marks, out)?;
            }
        } else if let Some(children) = children {
            emit_child(&children, out)?;
        }
        for _ in 0..element.closes {
            out.close()?;
        }
        self.leave_document(element.mark);
        Ok(())
    }

    fn special_id(&self, s: &str) -> RResult<u32> {
        match self.specials.get(s) {
            Some(Value::Int(i)) => Ok(*i as u32),
            _ => err(format!("{:?} is not an added token", s)),
        }
    }

    pub(super) fn scroll(&mut self, a: Vec<Value>) -> RResult<Value> {
        let setter = a.first().ok_or("missing scroll state")?;
        let Value::Setter(key) = setter else {
            return err("invalid scroll state");
        };
        let max = match a.get(2) {
            Some(Value::Int(n)) if *n >= 0 => *n,
            _ => return err("missing captured scroll bounds"),
        };
        let by = match a.get(3).map(|v| get_attr(v, "by")).transpose()?.unwrap_or(Value::Undef) {
            Value::Int(n) => n,
            Value::Float(n) if n.is_finite() && n.fract() == 0.0 && n >= i64::MIN as f64 && n < i64::MAX as f64 => {
                n as i64
            }
            _ => return err("scroll expects {by: integer}"),
        };
        if self.live_ui.as_ref().is_some_and(|ui| !ui.borrow().contains_key(key)) {
            return Ok(Value::Undef);
        }
        let current = self
            .live_ui
            .as_ref()
            .and_then(|ui| ui.borrow().get(key).cloned())
            .or_else(|| self.ui_updates.get(key).cloned())
            .or_else(|| self.ui_seen.get(key).cloned())
            .or_else(|| self.ui_value(key));
        let old = match current {
            Some(Value::Int(n)) => n.clamp(0, max),
            _ => 0,
        };
        let offset = (old as i128 + by as i128).clamp(0, max as i128) as i64;
        self.call_value(setter, vec![Value::Int(offset)])?;
        if let Some(callback) = a.get(1).filter(|f| f.is_callable()) {
            self.call_value(
                callback,
                vec![Value::from_json(&serde_json::json!({"by":offset-old,"offset":offset}))],
            )?;
        }
        Ok(Value::Int(offset))
    }
}
