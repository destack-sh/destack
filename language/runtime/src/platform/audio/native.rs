#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::bindings_generated as bindings;
use crate::platform::{NativeSlice, NativeStringRef, PlatformError};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::audio::{
    AudioDeviceDirection, AudioDeviceInfo, AudioStreamConfig, AudioStreamState,
};
use crate::platform::resource;

/// Stub for destack.audio.device.close.
pub unsafe fn destack_audio_device_close(
    context: &RuntimeCallContext,
    handle: resource::AudioDeviceHandle,
) -> RuntimeResult<()> {
    context.check_policy(AUDIO_DEVICE_CLOSE)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.device.close")).boxed())
}

/// Stub for destack.audio.device.list.
pub unsafe fn destack_audio_device_list(
    context: &RuntimeCallContext,
    out: *mut NativeSlice<AudioDeviceInfo>,
) -> RuntimeResult<()> {
    context.check_policy(AUDIO_DEVICE_LIST)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.device.list")).boxed())
}

/// Stub for destack.audio.device.open.
pub unsafe fn destack_audio_device_open(
    context: &RuntimeCallContext,
    out: *mut resource::AudioDeviceHandle,
    id: NativeStringRef,
    direction: AudioDeviceDirection,
) -> RuntimeResult<()> {
    context.check_policy(AUDIO_DEVICE_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, id, direction);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.device.open")).boxed())
}

/// Stub for destack.audio.stream.close.
pub unsafe fn destack_audio_stream_close(
    context: &RuntimeCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    context.check_policy(AUDIO_STREAM_CLOSE)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.stream.close")).boxed())
}

/// Stub for destack.audio.stream.open.
pub unsafe fn destack_audio_stream_open(
    context: &RuntimeCallContext,
    out: *mut resource::AudioStreamHandle,
    device: resource::AudioDeviceHandle,
    config: AudioStreamConfig,
) -> RuntimeResult<()> {
    context.check_policy(AUDIO_STREAM_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, device, config);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.stream.open")).boxed())
}

/// Stub for destack.audio.stream.read.
pub unsafe fn destack_audio_stream_read(
    context: &RuntimeCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::AudioStreamHandle,
    maxbytes: u32,
) -> RuntimeResult<()> {
    context.check_policy(AUDIO_STREAM_READ)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, maxbytes);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.stream.read")).boxed())
}

/// Stub for destack.audio.stream.start.
pub unsafe fn destack_audio_stream_start(
    context: &RuntimeCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    context.check_policy(AUDIO_STREAM_START)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.stream.start")).boxed())
}

/// Stub for destack.audio.stream.state.
pub unsafe fn destack_audio_stream_state(
    context: &RuntimeCallContext,
    out: *mut AudioStreamState,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    context.check_policy(AUDIO_STREAM_STATE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.stream.state")).boxed())
}

/// Stub for destack.audio.stream.stop.
pub unsafe fn destack_audio_stream_stop(
    context: &RuntimeCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    context.check_policy(AUDIO_STREAM_STOP)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.stream.stop")).boxed())
}

/// Stub for destack.audio.stream.write.
pub unsafe fn destack_audio_stream_write(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: resource::AudioStreamHandle,
    data: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(AUDIO_STREAM_WRITE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, data);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.stream.write")).boxed())
}
