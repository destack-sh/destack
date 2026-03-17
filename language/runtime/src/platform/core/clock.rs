use std::time::Instant;

/// Return the current process-relative monotonic time in nanoseconds.
pub(crate) fn monotonic_now_ns() -> u64 {
    // use the native Apple host-time domain
    #[cfg(target_vendor = "apple")]
    {
        super::apple_process_monotonic_nanos()
    }

    // use process-relative CLOCK_MONOTONIC on non-Apple Unix hosts
    #[cfg(all(unix, not(target_vendor = "apple")))]
    {
        super::unix_process_monotonic_nanos()
    }

    // use process-relative QPC on Windows hosts
    #[cfg(windows)]
    {
        super::qpc_process_monotonic_nanos()
            .unwrap_or_else(|| panic!("QueryPerformanceCounter monotonic clock unavailable"))
    }

    // fall back to one process-relative instant clock everywhere else
    // this only needs monotonic process-local ordering, not a host-global epoch
    #[cfg(not(any(unix, windows)))]
    {
        static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

        let start = START.get_or_init(Instant::now);
        let elapsed = start.elapsed();
        let elapsed = elapsed.as_nanos();

        u64::try_from(elapsed).unwrap_or(u64::MAX)
    }
}

/// Return one monotonic timeout deadline when it fits in `u64`.
pub(crate) fn timeout_deadline(timeout_ns: u64) -> Option<Instant> {
    Instant::now().checked_add(std::time::Duration::from_nanos(timeout_ns))
}
