use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::core::{
    clock_now_for_domain, resolve_stream_host_state, stream_clock_snapshot,
};
use crate::platform::audio::{AudioClockDomain, AudioClockSnapshot, AudioStreamClockDomain};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Read one timestamp in one selected clock domain.
pub(crate) unsafe fn destack_audio_clock_now(
    binding: &BindingCallContext,
    out: *mut u64,
    domain: AudioClockDomain,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let value = clock_now_for_domain(binding, domain)?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read one stream clock snapshot.
pub(crate) unsafe fn destack_audio_stream_clock(
    binding: &BindingCallContext,
    out: *mut AudioClockSnapshot,
    handle: resource::AudioStreamHandle,
    domain: AudioStreamClockDomain,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let resolved_binding =
        resolve_stream_host_state(binding, handle, "destack.audio.clock.stream")?;
    let snapshot = stream_clock_snapshot(binding, &resolved_binding, domain)?;
    unsafe {
        *out = snapshot;
    }

    Ok(())
}
