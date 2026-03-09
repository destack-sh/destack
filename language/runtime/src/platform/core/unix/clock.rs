use std::mem::MaybeUninit;
use std::sync::OnceLock;

/// Process-relative epoch for Unix monotonic clock normalization.
static UNIX_MONOTONIC_EPOCH_NS: OnceLock<u64> = OnceLock::new();

/// Convert one `timespec` value into one saturated nanosecond count.
fn timespec_to_nanos(spec: libc::timespec) -> u64 {
    let seconds = u64::try_from(spec.tv_sec)
        .unwrap_or_else(|_| panic!("clock_gettime returned negative seconds: {}", spec.tv_sec));
    let nanos = u64::try_from(spec.tv_nsec).unwrap_or_else(|_| {
        panic!(
            "clock_gettime returned negative nanoseconds: {}",
            spec.tv_nsec
        )
    });

    let seconds_nanos = seconds
        .checked_mul(1_000_000_000)
        .unwrap_or_else(|| panic!("clock_gettime seconds overflow nanoseconds: {seconds}"));

    seconds_nanos.saturating_add(nanos)
}

/// Sample one Unix monotonic clock value in nanoseconds.
fn unix_monotonic_clock_now_ns() -> u64 {
    let mut spec = MaybeUninit::<libc::timespec>::uninit();

    // sample one host monotonic timespec
    let status = unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, spec.as_mut_ptr()) };
    if status != 0 {
        panic!("clock_gettime(CLOCK_MONOTONIC) failed");
    }

    let spec = unsafe { spec.assume_init() };
    timespec_to_nanos(spec)
}

/// Return one process-relative monotonic timestamp in nanoseconds.
pub(crate) fn unix_process_monotonic_nanos() -> u64 {
    let epoch_ns = *UNIX_MONOTONIC_EPOCH_NS.get_or_init(unix_monotonic_clock_now_ns);
    let now_ns = unix_monotonic_clock_now_ns();

    now_ns.saturating_sub(epoch_ns)
}
