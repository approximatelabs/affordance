//! Cancellation (the browser's `AbortController`). Every executor and model call carries a token.
//! Cancelling it runs the cleanups the work registered (kill a process group, shut a socket), so
//! abandoned work actually stops instead of running on in a leaked thread.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

type Cleanup = Box<dyn FnOnce() + Send>;

/// A cancellation signal shared by the caller and the work (the browser's `AbortSignal`). Clones
/// share it.
#[derive(Clone, Default)]
pub struct CancelToken(Arc<Inner>);

#[derive(Default)]
struct Inner {
    cancelled: AtomicBool,
    /// Cancelled by the host (a controller pause, the run's shutdown), not by the work's owner.
    by_host: AtomicBool,
    cleanups: Mutex<Vec<Cleanup>>,
    reason: Mutex<String>,
}

impl std::fmt::Debug for CancelToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CancelToken({})", self.is_cancelled())
    }
}

impl CancelToken {
    /// A token not yet cancelled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether it has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::SeqCst)
    }

    /// The first cancel's reason (empty if not cancelled).
    pub fn reason(&self) -> String {
        self.0.reason.lock().unwrap().clone()
    }

    /// Cancel once: run every registered cleanup (later registrations run at once).
    pub fn cancel(&self, reason: &str) {
        self.cancel_as(reason, false)
    }

    /// Cancel on the host's behalf (a pause, a shutdown): the work did not fail and its owner did
    /// not abandon it, so its failure is not the owner's to record.
    pub fn cancel_by_host(&self, reason: &str) {
        self.cancel_as(reason, true)
    }

    /// Whether the first cancel was the host's.
    pub fn cancelled_by_host(&self) -> bool {
        self.is_cancelled() && self.0.by_host.load(Ordering::SeqCst)
    }

    fn cancel_as(&self, reason: &str, by_host: bool) {
        {
            let mut first = self.0.reason.lock().unwrap();
            if self.is_cancelled() {
                return;
            }
            *first = reason.to_string();
            self.0.by_host.store(by_host, Ordering::SeqCst);
            self.0.cancelled.store(true, Ordering::SeqCst);
        }
        let cleanups = std::mem::take(&mut *self.0.cleanups.lock().unwrap());
        for c in cleanups {
            c();
        }
    }

    /// Run `f` when the token is cancelled (now, if it already is).
    pub fn on_cancel(&self, f: impl FnOnce() + Send + 'static) {
        let mut cleanups = self.0.cleanups.lock().unwrap();
        if self.is_cancelled() {
            drop(cleanups);
            f();
        } else {
            cleanups.push(Box::new(f));
        }
    }

    /// `Err` with the reason if cancelled (for executors that check between steps).
    pub fn check(&self) -> Result<(), String> {
        if self.is_cancelled() {
            Err(format!("cancelled: {}", self.reason()))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_publishes_its_first_reason_before_observers_and_reentrant_cleanups() {
        let (send, receive) = std::sync::mpsc::sync_channel::<CancelToken>(0);
        let (answer, answers) = std::sync::mpsc::sync_channel(0);
        let observer = std::thread::spawn(move || {
            for token in receive {
                while !token.is_cancelled() {
                    std::hint::spin_loop();
                }
                answer.send(token.reason()).unwrap();
            }
        });
        for _ in 0..10_000 {
            let token = CancelToken::new();
            let called = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let (cleanup_token, count) = (token.clone(), called.clone());
            token.on_cancel(move || {
                assert_eq!(cleanup_token.reason(), "controller pause");
                cleanup_token.on_cancel(move || {
                    count.fetch_add(1, Ordering::SeqCst);
                });
            });
            send.send(token.clone()).unwrap();
            token.cancel("controller pause");
            token.cancel("a later reason");
            assert_eq!(answers.recv().unwrap(), "controller pause");
            assert_eq!(token.check(), Err("cancelled: controller pause".into()));
            assert_eq!(called.load(Ordering::SeqCst), 1);
        }
        drop(send);
        observer.join().unwrap();
    }
}
