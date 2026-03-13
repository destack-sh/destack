use std::time::{Duration, Instant};

/// Return one process-monotonic timestamp in nanoseconds.
#[cfg(any(unix, windows))]
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
}

/// Build one safe poll deadline for one timeout duration.
pub(crate) fn timeout_deadline(timeout_ns: u64) -> Option<Instant> {
    let timeout = Duration::from_nanos(timeout_ns);

    Instant::now().checked_add(timeout)
}

#[cfg(test)]
mod tests {
    use super::timeout_deadline;

    /// Return no deadline when the timeout cannot fit in the local instant domain.
    #[test]
    fn test_timeout_deadline_returns_none_for_instant_overflow() {
        let deadline = timeout_deadline(u64::MAX);

        assert!(deadline.is_none());
    }
}
