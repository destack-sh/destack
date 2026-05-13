use std::mem::MaybeUninit;
use std::sync::OnceLock;

/// Process-relative epoch for Unix monotonic clock normalization.
static UNIX_MONOTONIC_EPOCH_NS: OnceLock<u64> = OnceLock::new();

/// Convert one `timespec` value into one saturated nanosecond count.
fn timespec_to_nanos(spec: libc::timespec) -> Option<u64> {
    let seconds = u64::try_from(spec.tv_sec).ok()?;
    let nanos = u64::try_from(spec.tv_nsec).ok()?;
    let seconds_nanos = seconds.checked_mul(1_000_000_000)?;

    Some(seconds_nanos.saturating_add(nanos))
}

/// Sample one Unix monotonic clock value in nanoseconds.
fn unix_monotonic_clock_now_ns() -> Option<u64> {
    let mut spec = MaybeUninit::<libc::timespec>::uninit();

    // sample one host monotonic timespec
    let status = unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, spec.as_mut_ptr()) };
    if status != 0 {
        return None;
    }

    let spec = unsafe { spec.assume_init() };
    timespec_to_nanos(spec)
}

/// Return one process-relative monotonic timestamp in nanoseconds.
pub(crate) fn unix_process_monotonic_nanos() -> Option<u64> {
    let now_ns = unix_monotonic_clock_now_ns()?;
    let epoch_ns = *UNIX_MONOTONIC_EPOCH_NS.get_or_init(|| now_ns);

    Some(now_ns.saturating_sub(epoch_ns))
}
