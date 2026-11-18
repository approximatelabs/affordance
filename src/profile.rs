//! Profiling for benchmarks. A per-function profile of TSX runs in any build (`tsx_start`,
//! `tsx_take`). With the feature `profile`, `AFFORDANCE_FLAMEGRAPH=<path>` also samples the process
//! and writes a flame graph there: the one place pprof is called, the callsite docs/SECURITY.md
//! reviews. It writes SVG from its own samples and parses nothing. Callers summarize the samples
//! as plain names.

/// Start profiling named TSX functions on this thread: calls and inclusive time.
pub fn tsx_start() {
    crate::interp::profile_start()
}

/// Stop profiling TSX functions: `(function, calls, inclusive ms)`, the slowest first.
pub fn tsx_take() -> Vec<(String, u64, f64)> {
    crate::interp::profile_take()
}

#[cfg(feature = "profile")]
mod sampled {

    /// A running profile.
    pub struct Profile {
        path: String,
        guard: pprof::ProfilerGuard<'static>,
    }

    /// Sample at `hz` if `AFFORDANCE_FLAMEGRAPH` names an output file.
    pub fn start(hz: i32) -> Option<Profile> {
        let path = std::env::var("AFFORDANCE_FLAMEGRAPH").ok()?;
        let guard = pprof::ProfilerGuardBuilder::default()
            .frequency(hz)
            .blocklist(&["libc", "libgcc", "pthread", "vdso"])
            .build()
            .ok()?;
        Some(Profile { path, guard })
    }

    impl Profile {
        /// Write the flame graph and return the samples: each stack's frame names (leaf first) and
        /// its count.
        pub fn finish(self) -> Vec<(Vec<String>, usize)> {
            let Ok(report) = self.guard.report().build() else {
                return vec![];
            };
            match std::fs::File::create(&self.path) {
                Ok(file) => {
                    if let Err(e) = report.flamegraph(file) {
                        eprintln!("profile {}: {e}", self.path)
                    }
                }
                Err(e) => eprintln!("profile {}: {e}", self.path),
            }
            report
                .data
                .iter()
                .map(|(frames, n)| (frames.frames.iter().flatten().map(|f| f.name()).collect(), *n as usize))
                .collect()
        }
    }
}
#[cfg(feature = "profile")]
pub use sampled::*;
