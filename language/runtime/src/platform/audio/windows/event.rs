use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::audio::core::event::{
    close_event_stream, open_event_stream, read_event, read_event_batch, try_read_event,
    try_read_event_batch,
};
use crate::platform::audio::{AudioEvent, AudioEventSubscriptionOptions};
use crate::platform::resource::AudioEventHandle;
use crate::runtime::BindingCallContext;

/// Close one audio event subscription.
pub(crate) unsafe fn destack_audio_event_close(
    binding: &BindingCallContext,
    handle: AudioEventHandle,
) -> RuntimeResult<()> {
    unsafe { close_event_stream(binding, handle) }
}

/// Open one audio event subscription.
pub(crate) unsafe fn destack_audio_event_open(
    binding: &BindingCallContext,
    out: *mut AudioEventHandle,
    options: AudioEventSubscriptionOptions,
) -> RuntimeResult<()> {
    unsafe { open_event_stream(binding, out, options) }
}

/// Wait for one audio event.
pub(crate) unsafe fn destack_audio_event_read(
    binding: &BindingCallContext,
    out: *mut AudioEvent,
    handle: AudioEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { read_event(binding, out, handle, timeoutns) }
}

/// Wait for one batch of audio events.
pub(crate) unsafe fn destack_audio_event_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeSlice<AudioEvent>,
    handle: AudioEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { read_event_batch(binding, out, handle, maxevents, timeoutns) }
}

/// Poll one audio event without blocking.
pub(crate) unsafe fn destack_audio_event_try_read(
    binding: &BindingCallContext,
    out: *mut AudioEvent,
    handle: AudioEventHandle,
) -> RuntimeResult<()> {
    unsafe { try_read_event(binding, out, handle) }
}

/// Poll one batch of audio events without blocking.
pub(crate) unsafe fn destack_audio_event_try_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeSlice<AudioEvent>,
    handle: AudioEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    unsafe { try_read_event_batch(binding, out, handle, maxevents) }
}
