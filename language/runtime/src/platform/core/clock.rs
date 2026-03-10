/// Return one process-monotonic timestamp in nanoseconds.
#[cfg(any(unix, windows))]
#[allow(dead_code)]
pub(crate) fn monotonic_now_ns() -> u64 {
    // use the native Apple host-time domain
    #[cfg(target_vendor = "apple")]
    {
        return super::apple_process_monotonic_nanos();
    }

    // use process-relative CLOCK_MONOTONIC on non-Apple Unix hosts
    #[cfg(all(unix, not(target_vendor = "apple")))]
    {
        super::unix_process_monotonic_nanos()
    }

    // use process-relative QPC on Windows hosts
    #[cfg(windows)]
    {
        return super::qpc_process_monotonic_nanos()
            .unwrap_or_else(|| panic!("QueryPerformanceCounter monotonic clock unavailable"));
    }
}
