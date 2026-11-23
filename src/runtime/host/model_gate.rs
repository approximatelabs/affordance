//! Model admission: how many calls each binding has in flight, queued fairly per lane.
use super::*;

/// Admission to the model pool: at most `limit` calls in flight, split evenly across the model's
/// data-parallel ranks (one lane per rank, FIFO within a lane), the rest queued. With data-parallel
/// attention the ranks step in lockstep, so the busiest rank sets everyone's pace: balancing the
/// lanes is worth more than a global count. The pool stays as wide as `model_threads`; the limit
/// is what moves at runtime.
pub(super) struct ModelGate {
    lanes: Mutex<Vec<Lane>>,
    limit: AtomicUsize,
    next_lane: AtomicUsize,
}

#[derive(Default)]
pub(super) struct Lane {
    queue: VecDeque<QueuedModel>,
    inflight: usize,
}
pub(super) struct QueuedModel {
    agent: String,
    incarnation: u64,
    task: u64,
    job: Box<dyn FnOnce() + Send>,
}

impl ModelGate {
    pub(super) fn new(ranks: usize, limit: usize) -> ModelGate {
        ModelGate {
            lanes: Mutex::new((0..ranks.max(1)).map(|_| Lane::default()).collect()),
            limit: AtomicUsize::new(limit),
            next_lane: AtomicUsize::new(0),
        }
    }
    pub(super) fn set_limit(&self, n: usize) -> bool {
        // Serialize changes with admission. A decrease drains existing calls;
        // zero holds queued calls until the controller assigns capacity again.
        let _lanes = self.lanes.lock().unwrap();
        self.limit.swap(n, Ordering::SeqCst) != n
    }
    pub(super) fn take_next(&self) -> Option<(usize, QueuedModel)> {
        let mut lanes = self.lanes.lock().unwrap();
        let limit = self.limit.load(Ordering::SeqCst);
        if lanes.iter().map(|l| l.inflight).sum::<usize>() >= limit {
            return None;
        }
        let per_lane = limit.div_ceil(lanes.len());
        let start = self.next_lane.load(Ordering::SeqCst);
        // Preserve balanced per-rank shares and FIFO within each rank. Rotate
        // equal-load ties so a budget smaller than the rank count cannot leave
        // later ranks waiting behind a continually replenished early rank.
        let i = (0..lanes.len())
            .map(|offset| (start + offset) % lanes.len())
            .filter(|&i| lanes[i].inflight < per_lane && !lanes[i].queue.is_empty())
            .min_by_key(|&i| lanes[i].inflight)?;
        lanes[i].inflight += 1;
        let job = lanes[i].queue.pop_front().unwrap();
        self.next_lane.store((i + 1) % lanes.len(), Ordering::SeqCst);
        Some((i, job))
    }
    pub(super) fn release(&self, lane: usize) {
        self.lanes.lock().unwrap()[lane].inflight -= 1;
    }
    /// (in flight, queued) over all lanes.
    pub(super) fn counts(&self) -> (usize, usize) {
        let lanes = self.lanes.lock().unwrap();
        (
            lanes.iter().map(|l| l.inflight).sum(),
            lanes.iter().map(|l| l.queue.len()).sum(),
        )
    }
}

pub(super) fn submit_model(
    shared: &Arc<Shared>,
    lane: usize,
    agent: String,
    incarnation: u64,
    task: u64,
    priority: bool,
    job: Box<dyn FnOnce() + Send>,
) {
    {
        let mut lanes = shared.model_gate.lanes.lock().unwrap();
        // Checked under the lanes' lock, which closing's `suspend_queued_models` also takes: a call
        // queued after the run began closing would sit in a queue nothing drains any more.
        if shared.closing.load(Ordering::SeqCst) {
            drop(lanes);
            shared.done(&agent, incarnation, task);
            shared.inflight.fetch_sub(1, Ordering::SeqCst);
            return;
        }
        let n = lanes.len();
        let queued = QueuedModel {
            agent,
            incarnation,
            task,
            job,
        };
        if priority {
            lanes[lane % n].queue.push_front(queued)
        } else {
            lanes[lane % n].queue.push_back(queued)
        }
    }
    pump_model(shared);
}

/// Start queued model calls in every lane that has room under its share of the limit.
pub(super) fn pump_model(shared: &Arc<Shared>) {
    loop {
        if shared.closing.load(Ordering::SeqCst) {
            return;
        }
        let Some((lane, job)) = shared.model_gate.take_next() else {
            return;
        };
        // The slot is released however the job ends, a panic included.
        struct Lease(Arc<Shared>, usize);
        impl Drop for Lease {
            fn drop(&mut self) {
                self.0.model_gate.release(self.1);
                pump_model(&self.0);
            }
        }
        let done = Lease(shared.clone(), lane);
        shared.model_pool.submit(Box::new(move || {
            let _done = done;
            (job.job)();
        }));
    }
}

pub(super) fn suspend_queued_models(shared: &Shared) {
    let queued: Vec<_> = shared
        .model_gate
        .lanes
        .lock()
        .unwrap()
        .iter_mut()
        .flat_map(|lane| lane.queue.drain(..))
        .collect();
    for job in queued {
        // The journal still contains the outstanding Generate. Resuming submits it
        // again without inventing an error or starting a new decode while the run closes.
        shared.done(&job.agent, job.incarnation, job.task);
        shared.inflight.fetch_sub(1, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod model_gate_tests {
    use super::*;

    pub(super) fn enqueue(gate: &ModelGate, rank: usize, task: u64) {
        gate.lanes.lock().unwrap()[rank].queue.push_back(QueuedModel {
            agent: format!("rank-{rank}"),
            incarnation: 0,
            task,
            job: Box::new(|| {}),
        });
    }

    pub(super) fn populated(limit: usize) -> ModelGate {
        let gate = ModelGate::new(8, limit);
        for rank in 0..8 {
            for task in 0..300 {
                enqueue(&gate, rank, task);
            }
        }
        gate
    }

    #[test]
    pub(super) fn model_gate_exact_global_limits_and_balanced_ranks() {
        for limit in [1, 7, 9, 256, 896, 2048] {
            let gate = populated(limit);
            let mut by_rank = [0; 8];
            for _ in 0..limit {
                let (rank, _) = gate.take_next().expect("remaining global capacity");
                by_rank[rank] += 1;
                assert!(gate.counts().0 <= limit);
            }
            assert!(gate.take_next().is_none(), "cap {limit}");
            assert_eq!(gate.counts().0, limit);
            assert!(by_rank.iter().max().unwrap() - by_rank.iter().min().unwrap() <= 1);
            for (rank, &count) in by_rank.iter().enumerate() {
                for _ in 0..count {
                    gate.release(rank);
                }
            }
            assert_eq!(gate.counts().0, 0);
        }
    }

    #[test]
    pub(super) fn model_gate_waiting_ranks_make_progress_despite_continual_refill() {
        let gate = ModelGate::new(8, 1);
        for rank in 0..8 {
            enqueue(&gate, rank, 0);
        }
        let mut ranks = Vec::new();
        for _ in 0..24 {
            let (rank, job) = gate.take_next().unwrap();
            ranks.push(rank);
            enqueue(&gate, rank, job.task + 1);
            gate.release(rank);
        }
        assert_eq!(ranks, (0..24).map(|i| i % 8).collect::<Vec<_>>());
    }

    #[test]
    pub(super) fn model_gate_concurrent_pumps_cannot_exceed_allocation() {
        let gate = Arc::new(populated(9));
        let barrier = Arc::new(std::sync::Barrier::new(16));
        let handles: Vec<_> = (0..16)
            .map(|_| {
                let gate = gate.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    let mut admitted = 0;
                    while gate.take_next().is_some() {
                        admitted += 1;
                    }
                    admitted
                })
            })
            .collect();
        let admitted: usize = handles.into_iter().map(|handle| handle.join().unwrap()).sum();
        assert_eq!(admitted, 9);
        assert_eq!(gate.counts().0, 9);
    }

    #[test]
    pub(super) fn model_gate_uneven_demand_retains_rank_share_and_fifo() {
        let gate = ModelGate::new(8, 9);
        for task in 0..6 {
            enqueue(&gate, 0, task);
        }
        for rank in 1..4 {
            enqueue(&gate, rank, 100 + rank as u64);
        }
        let mut rank_zero = Vec::new();
        while let Some((rank, job)) = gate.take_next() {
            if rank == 0 {
                rank_zero.push(job.task);
            }
        }
        assert_eq!(rank_zero, [0, 1]);
        assert_eq!(gate.counts(), (5, 4));
        gate.release(0);
        let (rank, job) = gate.take_next().unwrap();
        assert_eq!((rank, job.task), (0, 2));
        assert_eq!(gate.counts(), (5, 3));
    }

    #[test]
    pub(super) fn model_gate_decrease_drains_without_cancelling_and_zero_holds() {
        let gate = populated(9);
        let active: Vec<_> = (0..9).map(|_| gate.take_next().unwrap().0).collect();
        assert!(gate.set_limit(1));
        assert_eq!(gate.counts().0, 9);
        for &rank in &active[..8] {
            assert!(gate.take_next().is_none());
            gate.release(rank);
        }
        assert!(gate.take_next().is_none());
        gate.release(active[8]);
        assert!(gate.take_next().is_some());
        assert!(gate.set_limit(0));
        assert!(gate.take_next().is_none());
        let rank = gate
            .lanes
            .lock()
            .unwrap()
            .iter()
            .position(|lane| lane.inflight == 1)
            .unwrap();
        gate.release(rank);
        assert_eq!(gate.counts().0, 0);
        assert!(gate.take_next().is_none());
        assert!(gate.set_limit(7));
        for _ in 0..7 {
            assert!(gate.take_next().is_some());
        }
        assert!(gate.take_next().is_none());
    }

    #[test]
    pub(super) fn model_gate_pump_releases_cancelled_returned_and_panicked_jobs_and_wakes_next() {
        let shared = Arc::new(Shared {
            cfg: RuntimeConfig::new("unused"),
            models: BTreeMap::new(),
            machines: BTreeMap::new(),
            store: Store::new(None),
            route: Default::default(),
            workers: Vec::new(),
            inflight: AtomicI64::new(0),
            status: Mutex::new(HashMap::new()),
            cancels: Mutex::new(HashMap::new()),
            model_pool: Pool::new(1, "gate-test"),
            exec_pool: Pool::new(1, "gate-test-exec"),
            model_gate: ModelGate::new(8, 0),
            closing: AtomicBool::new(false),
            started: Instant::now(),
            clock: Clock::new(0),
            diagnostics: Default::default(),
            retired: Default::default(),
            booting: Default::default(),
            durable: Default::default(),
            restoring: Permits::new(1),
        });
        let (tx, rx) = channel();
        let token = CancelToken::new();
        let queued_token = token.clone();
        let first_tx = tx.clone();
        // A job that panics releases its slot, and the pool's one thread runs the next.
        submit_model(
            &shared,
            0,
            "panics".into(),
            0,
            2,
            false,
            Box::new(|| panic!("a model job panicked")),
        );
        submit_model(
            &shared,
            0,
            "cancelled".into(),
            0,
            0,
            false,
            Box::new(move || {
                first_tx.send((0, queued_token.check().is_err())).unwrap();
            }),
        );
        submit_model(
            &shared,
            0,
            "next-in-lane".into(),
            0,
            1,
            false,
            Box::new(move || {
                tx.send((1, false)).unwrap();
            }),
        );
        token.cancel("queued cancellation");
        assert_eq!(shared.model_gate.counts(), (0, 3));
        shared.model_gate.set_limit(1);
        pump_model(&shared);
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), (0, true));
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), (1, false));
        let deadline = Instant::now() + Duration::from_secs(2);
        while shared.model_gate.counts() != (0, 0) {
            assert!(Instant::now() < deadline, "slots should release after returns");
            std::thread::yield_now();
        }
    }
}
