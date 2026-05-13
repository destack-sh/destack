use std::time::Instant;

/// Return the current process-relative monotonic time in nanoseconds.
pub(crate) fn monotonic_now_ns() -> u64 {
    // use the native Apple host-time domain
    #[cfg(target_os = "macos")]
    {
        match super::apple_process_monotonic_nanos() {
            Some(nanos) => nanos,
            None => fallback_process_monotonic_nanos(),
        }
    }

    // use process-relative CLOCK_MONOTONIC on non-Apple Unix hosts
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        match super::unix_process_monotonic_nanos() {
            Some(nanos) => nanos,
            None => fallback_process_monotonic_nanos(),
        }
    }

    // use process-relative QPC on Windows hosts
    #[cfg(windows)]
    {
        match super::qpc_process_monotonic_nanos() {
            Some(nanos) => nanos,
            None => fallback_process_monotonic_nanos(),
        }
    }

    // fall back to one process-relative instant clock everywhere else
    // this only needs monotonic process-local ordering, not a host-global epoch
    #[cfg(not(any(unix, windows)))]
    {
        fallback_process_monotonic_nanos()
    }
}

/// Return one monotonic timeout deadline when it fits in `u64`.
#[cfg(unix)]
pub(crate) fn timeout_deadline(timeout_ns: u64) -> Option<Instant> {
    Instant::now().checked_add(std::time::Duration::from_nanos(timeout_ns))
}

/// Return one process-relative monotonic timestamp from Rust's fallback clock.
fn fallback_process_monotonic_nanos() -> u64 {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

    let start = START.get_or_init(Instant::now);
    let elapsed = start.elapsed().as_nanos();

    elapsed.min(u128::from(u64::MAX)) as u64
}
