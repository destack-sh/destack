use std::sync::OnceLock;

/// Mach host-time status code for success.
const KERN_SUCCESS: libc::c_int = 0;

/// Process-relative epoch for Apple host-time normalization.
static APPLE_HOST_TIME_EPOCH: OnceLock<u64> = OnceLock::new();
/// Cached Mach host-time conversion ratio.
static APPLE_TIMEBASE_INFO: OnceLock<MachTimebaseInfo> = OnceLock::new();

/// Mach timebase conversion payload.
#[repr(C)]
#[derive(Clone, Copy)]
struct MachTimebaseInfo {
    /// Numerator for tick to nanosecond conversion.
    numer: u32,
    /// Denominator for tick to nanosecond conversion.
    denom: u32,
}

unsafe extern "C" {
    /// Return the current Mach absolute-time tick sample.
    fn mach_absolute_time() -> u64;

    /// Resolve the Mach absolute-time conversion ratio.
    fn mach_timebase_info(info: *mut MachTimebaseInfo) -> libc::c_int;
}

/// Return the cached Mach timebase conversion payload.
fn apple_timebase_info() -> &'static MachTimebaseInfo {
    APPLE_TIMEBASE_INFO.get_or_init(|| {
        let mut info = MachTimebaseInfo { numer: 0, denom: 0 };

        // resolve the host-time conversion ratio once for the process
        let status = unsafe { mach_timebase_info(&mut info) };
        if status != KERN_SUCCESS || info.denom == 0 {
            panic!(
                "mach_timebase_info failed: status={status}, denom={}",
                info.denom
            );
        }

        info
    })
}

/// Return the current Apple host-time tick sample.
///
/// CoreAudio host time and `mach_absolute_time` share the same underlying clock domain.
pub(crate) fn apple_host_time_now() -> u64 {
    unsafe { mach_absolute_time() }
}

/// Convert an Apple host-time tick value into nanoseconds.
pub(crate) fn apple_host_time_to_nanos(host_time: u64) -> u64 {
    let info = apple_timebase_info();
    let scaled =
        u128::from(host_time).saturating_mul(u128::from(info.numer)) / u128::from(info.denom);

    scaled.min(u64::MAX as u128) as u64
}

/// Return the nominal Apple host-time resolution in nanoseconds.
pub(crate) fn apple_host_time_resolution_nanos() -> u64 {
    let info = apple_timebase_info();
    let numer = u64::from(info.numer);
    let denom = u64::from(info.denom);

    numer.div_ceil(denom).max(1)
}

/// Convert an Apple host-time tick value into process-relative monotonic nanoseconds.
pub(crate) fn apple_host_time_to_process_nanos(host_time: u64) -> u64 {
    let epoch = *APPLE_HOST_TIME_EPOCH.get_or_init(apple_host_time_now);
    let delta = host_time.saturating_sub(epoch);

    apple_host_time_to_nanos(delta)
}

/// Convert process-relative monotonic nanoseconds into an Apple host-time tick value.
pub(crate) fn apple_process_nanos_to_host_time(nanos: u64) -> u64 {
    let epoch = *APPLE_HOST_TIME_EPOCH.get_or_init(apple_host_time_now);
    let info = apple_timebase_info();
    let ticks = u128::from(nanos).saturating_mul(u128::from(info.denom)) / u128::from(info.numer);
    let ticks = ticks.min(u64::MAX as u128) as u64;

    epoch.saturating_add(ticks)
}

/// Return the current Apple process-relative monotonic timestamp in nanoseconds.
pub(crate) fn apple_process_monotonic_nanos() -> u64 {
    apple_host_time_to_process_nanos(apple_host_time_now())
}
