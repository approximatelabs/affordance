//! Model calls (`model()`, `model(<View/>)`) and dispatching an output's calls.
use super::*;

impl Agent {
    /// `model(opts)`: the page's own view, rendered and laid out once, now, from live state, goes
    /// to the instance's model (the browser's `fetch`, with the page as its request). Its handlers
    /// are kept for `dispatch(output)`.
    pub(super) fn issue_model(
        &mut self,
        view: &View,
        enc: &Encoder,
        owner: (u64, usize),
        opts: &J,
    ) -> Result<(Option<Outcome>, Vec<Command>), String> {
        let mut out = vec![];
        if self.cfg.model.is_none() {
            return Ok((
                Some(Outcome::Error("model(): this instance has no model binding".into())),
                out,
            ));
        }
        if self.cfg.max_turns.is_some_and(|m| self.metrics.turns >= m) {
            self.stopped = Some(json!({"reason": "max_turns"}));
            self.note(json!({"t": "stop", "reason": "max_turns"}));
            return Ok((None, out));
        }
        let started = std::time::Instant::now();
        // A page that fails to render, or renders a prompt that cannot be read, rejects the call
        // (as `fetch` rejects): the app catches it and the instance goes on.
        let (r, prepared) = match self.render_prepared(view, enc) {
            Ok(r) => r,
            Err(e) => return Ok((Some(Outcome::Error(e)), out)),
        };
        let render_ms = started.elapsed().as_secs_f64() * 1000.0;
        let Some(protocol) = r.spans.iter().find(|s| s.label == "protocol" && s.kind == "label") else {
            return Ok((
                Some(Outcome::Error(
                    "model(): the view renders no prompt (no template app's protocol span)".into(),
                )),
                out,
            ));
        };
        let read = || {
            Ok::<_, String>((
                Syntax::parse(protocol.meta["syntax"].as_str().unwrap_or(""))?,
                image_sources(&r.spans)?,
                ActionSet::from_spans(&r.spans)?.with_protocol(&protocol.meta)?,
            ))
        };
        let (syntax, images, actions) = match read() {
            Ok(read) => read,
            Err(e) => return Ok((Some(Outcome::Error(format!("model(): {e}"))), out)),
        };
        let call_max_new = opts["max_new"].as_u64().map_or(self.cfg.max_new, |n| n as usize);
        if let Some(e) = self.over_context(r.ids.len(), call_max_new, &r.spans) {
            return Ok((Some(Outcome::Error(e)), out));
        }
        // `layout`: what the prompt's boxes did, after layout, as ResizeObserver reports sizes. An
        // app reads it to schedule a fold or a summary; it never measures during render.
        if r.spans
            .iter()
            .any(|s| s.kind == "affordance" && s.label == "callback" && s.meta["props"]["event"] == "layout")
        {
            let boxes: Vec<J> = r.spans.iter().filter(|s| s.kind == "box").map(|s| json!({
                "label": s.label, "tokens": s.end - s.start, "content_tokens": s.meta["content_tokens"],
                "hidden_tokens": s.meta["hidden_tokens"], "dropped_items": s.meta.get("dropped_items").cloned().unwrap_or(json!(0))})).collect();
            let window = self.cfg.context.map(|c| c.saturating_sub(call_max_new));
            self.events.push_back(("layout".into(), json!({"prompt_tokens": sent_tokens(r.ids.len(), &r.spans), "window": window, "context": self.cfg.context, "max_new": call_max_new, "boxes": boxes}), Some(prepared.clone())));
        }
        if let Some(m) = self.cfg.max_prompt.filter(|m| r.ids.len() > *m) {
            self.stopped = Some(json!({"reason": "prompt_over_limit", "tokens": r.ids.len(), "limit": m}));
            self.note(json!({"t": "stop", "reason": "prompt_over_limit", "tokens": r.ids.len()}));
            return Ok((None, out));
        }
        let stop: Vec<u32> = protocol.meta["stop"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_u64().map(|x| x as u32))
            .collect();
        let in_think = protocol.meta["in_think"].as_bool().unwrap_or(false);
        // The screen as the model saw it: a hosted prompt's is the content it sent (its `hosted`
        // span carries it), a native prompt's the current turn's ids and images.
        let hosted = r.spans.iter().find(|s| s.label == "hosted" && s.kind == "label");
        let screen = match (hosted, r.current_turn_ids(), r.current_turn()) {
            (Some(h), ..) => h.meta.get("screen").map(|content| json!({"content": content})),
            (None, Some(ids), Some(range)) => {
                let imgs: Vec<J> = r
                    .spans
                    .iter()
                    .filter(|s| s.meta.get("img").is_some() && s.start >= range.start && s.end <= range.end)
                    .map(|s| json!({"at": s.start - range.start, "n": s.end - s.start, "src": s.meta["src"]}))
                    .collect();
                let mut screen = json!({"ids": ids});
                if !imgs.is_empty() {
                    screen["images"] = json!(imgs);
                }
                Some(screen)
            }
            _ => None,
        };
        // Exact append: this prompt starts with the previous page call's prompt plus its reply.
        let last_len = self.last_full.as_ref().map_or(0, |(p, reply)| p.len() + reply.len());
        let common = self.last_full.as_ref().map_or(0, |(p, reply)| {
            p.iter().chain(reply).zip(&r.ids).take_while(|(a, b)| a == b).count()
        });
        let exact = last_len > 0 && common == last_len;
        if last_len > 0 {
            if exact {
                self.metrics.exact_appends += 1
            } else {
                self.metrics.rewrites += 1
            }
        }
        self.metrics.turns += 1;
        self.metrics.prompt_tokens += r.ids.len() as u64;
        self.metrics.uncached_tokens += (r.ids.len() - common) as u64;
        self.metrics.max_prompt = self.metrics.max_prompt.max(r.ids.len());
        self.last_prompt = r.ids.len();
        let mut render = json!({"t": "render", "turn": self.metrics.turns, "prompt_tokens": r.ids.len(), "screen_tokens": screen.as_ref().and_then(|s| s["ids"].as_array()).map_or(0, Vec::len),
            "append": if last_len == 0 { J::Null } else if exact { json!("exact") } else { json!({"rewrite_at": common, "of": last_len}) }, "render_ms": render_ms});
        // The prefix a prefix-caching server can reuse: what this prompt shares with the previous
        // page prompt and its reply.
        if last_len > 0 {
            render["shared"] = json!(common);
        }
        // The prompt's hash: over what is new when it is an exact append, else over all of it.
        let (mut hasher, from) = match self.last_hasher.take() {
            Some(h) if exact => (h, last_len),
            _ => (sha2::Sha256::default(), 0),
        };
        sha_ids(&mut hasher, &r.ids[from..]);
        let hash = hex_digest(&hasher);
        let usage = json!({"prompt_tokens": r.ids.len(), "cached_tokens": common, "exact": exact});
        let ids: Arc<[u32]> = r.ids[..].into();
        // The call's own retry policy, if it states one, over its binding's.
        if !opts["retry"].is_null() {
            let policy = self
                .cfg
                .retry
                .with(&opts["retry"])
                .map_err(|e| format!("model(): {e}"))?;
            self.open.entry(owner).or_default().policy = Some(policy);
        }
        let max_new = opts["max_new"].as_u64().map_or(self.cfg.max_new, |n| n as usize);
        let gen_opts = GenOptions {
            sampling: opts["sampling"].clone(),
            tag: opts["tag"].as_str().map(str::to_owned),
            priority: opts["priority"].as_bool().unwrap_or(false),
            abortable: self.open.get(&owner).is_some_and(|o| o.signal.is_some()),
            pipe: (!opts["pipe"].is_null()).then(|| Box::new(opts["pipe"].clone())),
        };
        let cmd = self.new_task(Some(owner), |task| Command::Generate {
            task,
            ids: ids.clone(),
            stop: stop.clone(),
            max_new,
            opts: gen_opts,
            prompt_hash: Some(hash.clone()),
            images,
            media: media_runs(&r.spans),
        });
        let no = self.metrics.turns;
        // The model call this turn issued (its `generate` record's `task`).
        if let Command::Generate { task, .. } = &cmd {
            render["task"] = json!(task);
        }
        self.note(render);
        self.open.entry(owner).or_default().model = Some(ModelCall {
            page: true,
            no,
            hash,
            hasher,
            prompt: ids,
            actions,
            syntax,
            in_think,
            stop,
            screen,
            usage,
        });
        let (state, ctx, clock_ms) = (self.state.clone(), self.context(), self.clock_ms);
        self.kept.insert(
            no,
            Kept {
                view: Some((prepared, state, ctx, clock_ms, self.journaled as u64)),
                calls: None,
                started: BTreeSet::new(),
                results: vec![],
            },
        );
        // Old outputs go, and old undispatched views (a dispatch that is running keeps its own).
        let running: BTreeSet<u64> = self.dispatches.values().map(|d| d.call).collect();
        while self.kept.len() > KEPT_OUTPUTS {
            let Some(old) = self.kept.keys().find(|n| !running.contains(n)).copied() else {
                break;
            };
            self.kept.remove(&old);
        }
        let waiting: Vec<u64> = self
            .kept
            .iter()
            .filter(|(n, k)| k.view.is_some() && !running.contains(n) && k.started.is_empty())
            .map(|(n, _)| *n)
            .collect();
        for n in waiting.iter().take(waiting.len().saturating_sub(KEPT_VIEWS)) {
            if let Some(k) = self.kept.get_mut(n) {
                k.view = None;
            }
        }
        out.push(cmd);
        Ok((None, out))
    }

    /// A page call's reply as the `Output` its `model()` resolves with: text, reasoning and calls
    /// parsed against the view's affordances, the generated ids, the screen, usage.
    pub(super) fn model_output(&mut self, enc: &Encoder, call: ModelCall, ids: &[u32], cut_off: bool) -> Outcome {
        let mut hasher = call.hasher.clone();
        sha_ids(&mut hasher, ids);
        self.last_full = Some((call.prompt.clone(), ids.to_vec()));
        self.last_hasher = Some(hasher);
        let body = match ids.last() {
            Some(last) if call.stop.contains(last) => &ids[..ids.len() - 1],
            _ => ids,
        };
        let text = match enc.tok.decode(body, false) {
            Ok(t) => t,
            Err(e) => return Outcome::Error(e.to_string()),
        };
        let reply = parse_reply_typed(&text, call.syntax, &call.actions, call.in_think, cut_off);
        let n = call.no;
        // The model's own call id when it writes one (its tool result must cite it), else ours. A
        // call the view does not offer, or with arguments outside its schema, carries the problem
        // it is answered with when dispatched, so the model sees what went wrong.
        let calls: Vec<(String, J, String, Option<String>)> = if cut_off {
            vec![]
        } else {
            reply
                .calls
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let name = c.name.clone();
                    // This call's own problem (`reply.problems` is the whole reply's, by name).
                    let problem = match crate::reply::call_problem(&call.actions, &name, &c.args) {
                        Some(Problem::UnknownAction(_)) => {
                            Some(format!("error: there is no tool named `{name}` on this screen"))
                        }
                        Some(Problem::BadArgs(e)) => Some(format!("error: bad arguments for {e}")),
                        _ => None,
                    };
                    (
                        name,
                        c.args.clone(),
                        c.id.clone().unwrap_or_else(|| format!("call_{n}_{i}")),
                        problem,
                    )
                })
                .collect()
        };
        let mut record = json!({"t": "reply", "turn": n, "tokens": ids.len(), "calls": calls.len(), "problems": reply.problem_names(), "cut_off": cut_off});
        // A reply with no call whose text holds a tool call's markup: the server did not parse it
        // (it runs without its tool-call parser). The trace says so; nothing else changes.
        if calls.is_empty()
            && [
                "<tool_call>",
                "<|tool_call",
                "<function=",
                "[TOOL_CALLS]",
                "<|python_tag|>",
            ]
            .iter()
            .any(|mark| reply.prose.contains(mark))
        {
            record["text_call"] = json!(true);
        }
        self.note(record);
        let mut usage = call.usage.clone();
        usage["generated_tokens"] = json!(ids.len());
        let mut value = json!({"text": reply.prose, "reasoning": reply.think, "cut_off": cut_off, "problems": reply.problem_names(),
            "calls": calls.iter().map(|(name, args, id, problem)| { let mut c = json!({"name": name, "args": args, "id": id}); if let Some(p) = problem { c["problem"] = json!(p); } c }).collect::<Vec<_>>(),
            "ids": ids, "screen": call.screen,
            "usage": usage, "__call": n});
        if let Some(state) = reply.provider_state {
            value["provider_state"] = json!(state);
        }
        if let Some(k) = self.kept.get_mut(&n) {
            // A reply with no calls has nothing to dispatch: its view is released now, and a
            // dispatch of it answers with no results. Kept, the view would stand in the way of
            // a recovery image for as long as the app did not dispatch (a chat bot never does).
            if calls.is_empty() {
                k.view = None;
            }
            k.results = vec![None; calls.len()];
            k.calls = Some(calls);
        }
        Outcome::Ok(value)
    }

    /// `dispatch(output, {calls})`: run a page call's calls (all, or those at the indices given,
    /// in that order) on the handlers of the view the model saw, and answer with their results.
    /// Each call runs once: a call already started (by this dispatch or an earlier one) is waited
    /// for, and one already answered gives its result again. So an app runs a reply's calls one
    /// after another, all at once, or any way between, with plain promises.
    pub(super) fn dispatch(
        &mut self,
        view: &View,
        enc: &Encoder,
        waiter: (u64, usize),
        call: Option<u64>,
        only: Option<Vec<Option<u64>>>,
    ) -> Result<(Option<Outcome>, Vec<Command>), String> {
        let mut out = vec![];
        let Some(n) = call else {
            return Ok((
                Some(Outcome::Error(
                    "InvalidStateError: this output cannot be dispatched (an ad-hoc view's handlers are not kept)"
                        .into(),
                )),
                out,
            ));
        };
        let gone = || {
            Outcome::Error(format!(
                "InvalidStateError: this output's view is no longer kept (dispatch it before {KEPT_VIEWS} newer page calls)"
            ))
        };
        let Some(k) = self.kept.get_mut(&n) else {
            return Ok((Some(gone()), out));
        };
        let Some(calls) = k.calls.clone() else {
            return Ok((
                Some(Outcome::Error("InvalidStateError: dispatch before the reply".into())),
                out,
            ));
        };
        let indices: Vec<usize> = match only {
            None => (0..calls.len()).collect(),
            Some(asked) => {
                let mut indices = vec![];
                for i in asked {
                    match i.map(|i| i as usize).filter(|i| *i < calls.len()) {
                        Some(i) if !indices.contains(&i) => indices.push(i),
                        Some(_) => {}
                        None => {
                            return Ok((
                                Some(Outcome::Error(format!(
                                    "RangeError: dispatch: calls are indices into the output's {} calls",
                                    calls.len()
                                ))),
                                out,
                            ));
                        }
                    }
                }
                indices
            }
        };
        let answered = |k: &Kept| {
            J::Array(
                indices
                    .iter()
                    .map(|&i| k.results[i].clone().unwrap_or(J::Null))
                    .collect(),
            )
        };
        if indices.iter().all(|&i| k.results[i].is_some()) {
            return Ok((Some(Outcome::Ok(answered(k))), out));
        }
        let to_start: Vec<usize> = indices.iter().copied().filter(|i| !k.started.contains(i)).collect();
        // The tools run from the view; once every call has been started, only the results stay.
        let view_kept = match (&k.view, to_start.is_empty()) {
            (_, true) => None,
            (Some(v), false) => Some(v.clone()),
            (None, false) => return Ok((Some(gone()), out)),
        };
        let d = self.next_dispatch;
        self.next_dispatch += 1;
        self.dispatches.insert(
            d,
            Dispatch {
                call: n,
                waiters: vec![waiter],
                indices,
                starting: true,
            },
        );
        if let Some((prepared, state, ctx, clock, issued)) = view_kept {
            for index in to_start {
                let (name, args, call_id, problem) = calls[index].clone();
                let k = self.kept.get_mut(&n).unwrap();
                k.started.insert(index);
                // Every call started: the view and the calls' arguments are no longer needed (only
                // the results stay, and recovery images do not carry what was consumed).
                if k.started.len() == calls.len() {
                    k.view = None;
                    for c in k.calls.iter_mut().flatten() {
                        c.1 = J::Null;
                    }
                }
                match problem {
                    Some(error) => {
                        k.results[index] = Some(json!({"name": name, "id": call_id, "result": error, "error": true}));
                    }
                    None => {
                        let origin = Origin::Tool {
                            name: name.clone(),
                            call_id,
                            dispatch: d,
                            index,
                            started_ms: self.clock_ms,
                        };
                        out.extend(self.start_handler_at(
                            view,
                            enc,
                            origin,
                            &name,
                            0,
                            args,
                            state.clone(),
                            ctx.clone(),
                            clock,
                            Some(prepared.clone()),
                            Some(issued),
                        )?);
                    }
                }
            }
        }
        if let Some(d) = self.dispatches.get_mut(&d) {
            d.starting = false;
        }
        self.finish_dispatches();
        Ok((None, out))
    }

    /// Resolve the dispatches each of whose calls has answered, with their results in the order
    /// asked.
    pub(super) fn finish_dispatches(&mut self) {
        let done: Vec<u64> = self
            .dispatches
            .iter()
            .filter(|(_, d)| {
                !d.starting
                    && self
                        .kept
                        .get(&d.call)
                        .is_none_or(|k| d.indices.iter().all(|&i| k.results.get(i).is_some_and(Option::is_some)))
            })
            .map(|(k, _)| *k)
            .collect();
        for d in done {
            let Some(d) = self.dispatches.remove(&d) else {
                continue;
            };
            let results = match self.kept.get(&d.call) {
                Some(k) => J::Array(
                    d.indices
                        .iter()
                        .map(|&i| k.results[i].clone().unwrap_or(J::Null))
                        .collect(),
                ),
                None => J::Array(vec![]),
            };
            for (h, i) in d.waiters {
                self.resolve(h, i, Outcome::Ok(results.clone()));
            }
        }
    }

    /// `prepared`: the render the handler comes from, when the agent has it (a tool call's turn).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn start_handler_at(
        &mut self,
        view: &View,
        enc: &Encoder,
        origin: Origin,
        action: &str,
        which: usize,
        args: J,
        state: Rc<Map>,
        ctx: Value,
        clock_ms: i64,
        prepared: Option<Rc<crate::Prepared>>,
        at: Option<u64>,
    ) -> Result<Vec<Command>, String> {
        let id = self.next_handler;
        self.next_handler += 1;
        let at = at.unwrap_or(self.journaled as u64);
        self.handlers.insert(
            id,
            Handler {
                origin,
                action: action.into(),
                which,
                args,
                state,
                ctx,
                clock_ms,
                effects: BTreeMap::new(),
                next_effect: 0,
                prepared,
                closure: None,
                at,
                consumed: at,
                done: None,
            },
        );
        self.run_handler(view, enc, id)
    }

    /// The outcome of `model(<View/>)`, an ad-hoc call: the reply read by the view's protocol.
    pub(super) fn render_result(&self, enc: &Encoder, h: u64, i: usize, ids: &[u32], cut_off: bool) -> Outcome {
        let Some(Request::Render { ids: prompt, spans, .. }) = self.handlers.get(&h).and_then(|h| h.effects.get(&i))
        else {
            return Outcome::Error("not a render".into());
        };
        let read = || -> Result<J, String> {
            let protocol = spans
                .iter()
                .find(|s| s.label == "protocol" && s.kind == "label")
                .ok_or("no protocol span")?;
            let syntax = Syntax::parse(protocol.meta["syntax"].as_str().unwrap_or(""))?;
            let in_think = protocol.meta["in_think"].as_bool().unwrap_or(false);
            let stop = protocol_stop(spans);
            let body = match ids.last() {
                Some(last) if stop.contains(last) => &ids[..ids.len() - 1],
                _ => ids,
            };
            let text = enc.tok.decode(body, false).map_err(|e| e.to_string())?;
            let reply = parse_reply_typed(
                &text,
                syntax,
                &ActionSet::from_spans(spans)?.with_protocol(&protocol.meta)?,
                in_think,
                cut_off,
            );
            let calls: Vec<J> = reply
                .calls
                .iter()
                .map(|c| json!({"name": c.name, "args": c.args, "id": c.id}))
                .collect();
            let mut out = json!({"text": reply.prose, "reasoning": reply.think, "calls": calls, "cut_off": cut_off, "problems": reply.problem_names(),
                "ids": ids, "screen": null, "__call": null,
                // Not on the page's append chain: no prefix of it is known to be cached.
                "usage": {"prompt_tokens": prompt.len(), "cached_tokens": 0, "exact": false, "generated_tokens": ids.len()}});
            if let Some(state) = reply.provider_state {
                out["provider_state"] = json!(state);
            }
            Ok(out)
        };
        read().map_or_else(Outcome::Error, Outcome::Ok)
    }
}
