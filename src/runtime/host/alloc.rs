//! The process's allocator, tuned for a long run of many agents (Linux glibc only).

/// Give the allocator's free memory back to the system. A long run frees as much as it
/// allocates (renders, replies, handler states), but glibc keeps freed pages in its arenas, so
/// resident memory would only grow.
pub(super) fn trim_heap() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        unsafe extern "C" {
            fn malloc_trim(pad: usize) -> i32;
        }
        unsafe {
            malloc_trim(0);
        }
    }
}

/// Tune glibc's allocator for a long-running process of many agents (`workers` render workers).
/// It is process-wide, so the library never does it on its own: the `affordance` binary calls it
/// before it runs, and an embedder may. A setting the environment already makes
/// (`MALLOC_ARENA_MAX`, `MALLOC_TOP_PAD_`) is left alone. No-op outside Linux glibc.
pub fn tune_allocator(workers: usize) {
    if std::env::var_os("MALLOC_ARENA_MAX").is_none() {
        limit_arenas(8.max(2 * workers.min(1 << 20) as i32));
    }
    if std::env::var_os("MALLOC_TOP_PAD_").is_none() {
        pad_heaps();
    }
}

/// Grow a thread's heap 64 MiB at a time. A thread's malloc arena grows and shrinks its heap with
/// `mprotect`, which takes the process's memory-map lock, so threads allocating at once (workers
/// booting agents) queued behind one another: about 95 `mprotect` calls an agent booted, and 32
/// workers booted 4,096 agents in 1.5 s, 11 ms an agent where one thread takes 1. With the pad:
/// 0.26 s. The pad is address space; only pages touched count as memory.
pub fn pad_heaps() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        const M_TOP_PAD: i32 = -2;
        unsafe extern "C" {
            fn mallopt(param: i32, value: i32) -> i32;
        }
        unsafe {
            mallopt(M_TOP_PAD, 64 << 20);
        }
    }
}

/// At most `n` malloc arenas. glibc gives threads their own arenas (up to eight per core); with
/// hundreds of model and exec threads, memory freed in one arena is not reused by another and a
/// long run's heap fragments. Measured (runtime_bench, 8 agents x 800 turns): 6 MiB an agent of
/// growth with the default, none with 8 arenas, at the same speed (one arena is 4x slower). Render
/// workers need two each: replaying 256 agents with 16 workers took 9:56 with 8 arenas and 3:08
/// with 32, at about the same memory.
pub(super) fn limit_arenas(n: i32) {
    #[cfg(not(all(target_os = "linux", target_env = "gnu")))]
    let _ = n;
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        const M_ARENA_MAX: i32 = -8;
        unsafe extern "C" {
            fn mallopt(param: i32, value: i32) -> i32;
        }
        unsafe {
            mallopt(M_ARENA_MAX, n);
        }
    }
}
