#[cfg(target_os = "macos")]
use std::sync::OnceLock;

#[cfg(target_os = "macos")]
use crate::platform::audio::core as audio_core;

#[cfg(target_os = "macos")]
use super::abi::AudioConvertHostTimeToNanos;

/// Host-time to runtime-monotonic offset used for timestamp correlation.
#[cfg(target_os = "macos")]
static COREAUDIO_HOST_TIME_OFFSET_NS: OnceLock<i128> = OnceLock::new();

/// Convert one CoreAudio host-time value into runtime monotonic nanoseconds.
#[cfg(target_os = "macos")]
pub(super) fn coreaudio_host_time_to_mono_ns(host_time: u64) -> u64 {
    // map one native host-time clock sample into nanoseconds
    let host_time_ns = unsafe { AudioConvertHostTimeToNanos(host_time) };

    // resolve one stable offset from host-time to runtime monotonic
    let offset_ns = COREAUDIO_HOST_TIME_OFFSET_NS.get_or_init(|| {
        let mono_now = audio_core::host_monotonic_nanos() as i128;
        mono_now - host_time_ns as i128
    });

    // apply offset and clamp into u64 space
    let mapped_ns = host_time_ns as i128 + *offset_ns;
    if mapped_ns <= 0 {
        return 0;
    }

    mapped_ns.min(u64::MAX as i128) as u64
}
