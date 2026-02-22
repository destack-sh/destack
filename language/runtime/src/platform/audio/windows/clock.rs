use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::{
    AudioClockDomain, AudioClockSnapshot, AudioStreamClockDomain, core as audio_core,
};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Read one timestamp in one selected clock domain.
///
/// Read one clock timestamp for the selected domain.
/// Domain availability and precision follow host backend behavior.
/// `AudioClockDomain.Device` requires one backend-wide device timeline and can return `notSupported` otherwise.
///
/// # Platform
/// Unix and Windows.
/// Mirrors PortAudio `Pa_GetStreamTime` monotonic stream-time semantics.
/// Mirrors cubeb stream and latency clock snapshot semantics.
/// Mirrors host monotonic and wall clock query semantics.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_clock_now(
    context: &BindingCallContext,
    out: *mut u64,
    domain: AudioClockDomain,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let value = audio_core::clock_now_for_domain(context, domain)?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read one stream clock snapshot.
///
/// Read one synchronized stream-position and clock timestamp snapshot.
/// Snapshot values are advisory and can change immediately after read.
/// Domain-specific lanes like `InputAdc`, `OutputDac`, and `Device` can return `notSupported` when the opened stream does not expose them.
///
/// # Platform
/// Unix and Windows.
/// Mirrors PortAudio `PaStreamCallbackTimeInfo` input and output timestamp correlation.
/// Mirrors ASIO `bufferSwitchTimeInfo` and time-info correlation semantics.
/// Mirrors cubeb `cubeb_stream_get_position` plus latency-correlation snapshots.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_clock(
    context: &BindingCallContext,
    out: *mut AudioClockSnapshot,
    handle: resource::AudioStreamHandle,
    domain: AudioStreamClockDomain,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let binding =
        audio_core::resolve_stream_binding(context, handle, "destack.audio.clock.stream")?;
    let snapshot = audio_core::stream_clock_snapshot(context, &binding, domain)?;
    unsafe {
        *out = snapshot;
    }

    Ok(())
}
