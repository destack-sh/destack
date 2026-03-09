#[cfg(target_os = "macos")]
use crate::platform::core as core_platform;

/// Convert a CoreAudio host-time value into runtime monotonic nanoseconds.
#[cfg(target_os = "macos")]
pub(super) fn coreaudio_host_time_to_mono_ns(host_time: u64) -> u64 {
    core_platform::apple_host_time_to_process_nanos(host_time)
}
