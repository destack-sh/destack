use std::time::Duration;

use super::core as audio_platform_core;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::audio::core::{
    AUDIO_STREAM_RESOURCE_LABEL, AudioStreamFinalizer, EVENT_POLL_INTERVAL_NS,
    MAX_STREAM_READ_BYTES, STREAM_STATUS_INPUT_UNDERFLOW, STREAM_STATUS_OUTPUT_OVERFLOW,
    audio_not_found, audio_would_block, decode_audio_bytes, encode_audio_bytes,
    ensure_capture_direction, ensure_playback_direction, ensure_stream_capability,
    ensure_stream_requirements_satisfied, frame_bytes, host_monotonic_nanos, host_stream_flush,
    host_stream_pause, host_stream_start, host_stream_stop, open_null_stream,
    publish_stream_event_native, read_utf8, resolve_device_host_state, resolve_stream_host_state,
    runtime_state, sample_bytes, satisfied_stream_requirements, stream_availability_snapshot,
    stream_descriptor, stream_device_from_state, stream_shutdown_error, stream_state_is_terminal,
    stream_state_snapshot, stream_timing_snapshot, validate_stream_config,
    validate_stream_open_options, validate_stream_open_options_for_backend,
    wait_for_stream_presentation_time,
};
use crate::platform::audio::{
    AudioEventKind, AudioStreamAvailability, AudioStreamConfig, AudioStreamDescriptor,
    AudioStreamOpenOptions, AudioStreamRequirementFlags, AudioStreamState, AudioStreamStateKind,
    AudioStreamStatusFlags, AudioStreamSupport, AudioStreamTiming,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Flatten one vectorized byte-buffer payload into one contiguous payload.
unsafe fn flatten_vectorized_buffers(
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<Vec<u8>> {
    let buffers = unsafe { buffers.as_slice()? };
    let mut bytes = Vec::new();
    for buffer in buffers {
        let chunk = unsafe { buffer.as_slice()? };
        bytes.extend_from_slice(chunk);
    }

    Ok(bytes)
}

/// Return total writable byte capacity for one vectorized output payload.
unsafe fn vectorized_capacity_bytes(buffers: NativeSlice<NativeSlice<u8>>) -> RuntimeResult<usize> {
    let buffers = unsafe { buffers.as_slice()? };
    let mut total = 0usize;
    for buffer in buffers {
        total = total.saturating_add(buffer.len as usize);
    }

    Ok(total)
}

/// Copy one contiguous payload into one vectorized mutable byte-buffer payload.
unsafe fn copy_into_vectorized_buffers(
    buffers: NativeSlice<NativeSlice<u8>>,
    data: &[u8],
) -> RuntimeResult<u64> {
    let buffers = unsafe { buffers.as_mut_slice()? };
    let mut copied = 0usize;

    for buffer in buffers {
        let writable = unsafe { buffer.as_mut_slice()? };
        if copied >= data.len() {
            break;
        }

        let chunk_len = writable.len().min(data.len() - copied);
        writable[..chunk_len].copy_from_slice(&data[copied..copied + chunk_len]);
        copied = copied.saturating_add(chunk_len);
    }

    Ok(copied as u64)
}

/// Read one stream immediate availability sample.
pub(crate) unsafe fn destack_audio_stream_availability(
    binding: &BindingCallContext,
    out: *mut AudioStreamAvailability,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let resolved_binding =
        resolve_stream_host_state(binding, handle, "destack.audio.stream.availability")?;
    unsafe {
        *out = stream_availability_snapshot(binding, &resolved_binding);
    }

    Ok(())
}

/// Close one audio stream.
pub(crate) unsafe fn destack_audio_stream_close(
    binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let stream = resolve_stream_host_state(binding, handle, "destack.audio.stream.close")?;
    stream.unbind_runtime();

    if !binding.worker().resources.remove_and_finalize(
        binding.world(),
        handle.0,
        Some(binding.engine()),
    ) {
        return Err(audio_not_found(
            "destack.audio.stream.close",
            format!("unknown audio stream handle {}", handle.0.local_id),
        ));
    }

    Ok(())
}

/// Drain one playback stream.
pub(crate) unsafe fn destack_audio_stream_drain(
    binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let resolved_binding =
        resolve_stream_host_state(binding, handle, "destack.audio.stream.drain")?;
    ensure_playback_direction(resolved_binding.direction, "destack.audio.stream.drain")?;

    let deadline = host_monotonic_nanos().saturating_add(timeoutns);
    let mut state = resolved_binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    loop {
        if state.playback_samples.is_empty() {
            return Ok(());
        }

        if timeoutns == 0 || host_monotonic_nanos() >= deadline {
            return Err(audio_would_block(
                "destack.audio.stream.drain",
                "stream drain timed out",
            ));
        }

        let remaining = deadline.saturating_sub(host_monotonic_nanos());
        let duration = Duration::from_nanos(remaining.min(EVENT_POLL_INTERVAL_NS));
        let wait = resolved_binding
            .sync
            .wake
            .wait_timeout(state, duration)
            .unwrap_or_else(|error| error.into_inner());
        state = wait.0;
    }
}

/// Flush buffered stream data.
pub(crate) unsafe fn destack_audio_stream_flush(
    binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let resolved_binding =
        resolve_stream_host_state(binding, handle, "destack.audio.stream.flush")?;
    host_stream_flush(&resolved_binding)?;

    let mut state = resolved_binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    state.playback_samples.clear();
    state.capture_samples.clear();
    drop(state);
    resolved_binding.sync.wake.notify_all();

    Ok(())
}

/// Read one stream negotiated configuration descriptor.
pub(crate) unsafe fn destack_audio_stream_descriptor(
    binding: &BindingCallContext,
    out: *mut AudioStreamDescriptor,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let resolved_binding =
        resolve_stream_host_state(binding, handle, "destack.audio.stream.descriptor")?;
    unsafe {
        *out = stream_descriptor(binding, &resolved_binding);
    }

    Ok(())
}

/// Open one audio stream on one device.
pub(crate) unsafe fn destack_audio_stream_open(
    binding: &BindingCallContext,
    out: *mut resource::AudioStreamHandle,
    device: resource::AudioDeviceHandle,
    config: AudioStreamConfig,
    options: AudioStreamOpenOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    validate_stream_config(config)?;
    validate_stream_open_options(options, "destack.audio.stream.open")?;

    let device_binding = resolve_device_host_state(binding, device, "destack.audio.stream.open")?;
    let stream_device = stream_device_from_state(&device_binding);
    validate_stream_open_options_for_backend(
        options,
        stream_device.backend,
        "destack.audio.stream.open",
    )?;

    let stream = if stream_device.is_null {
        open_null_stream(
            &stream_device,
            device_binding.opened_direction,
            config,
            device_binding.options.share_mode,
            options.flags,
            options.requirements,
        )
    } else {
        audio_platform_core::open_host_stream(
            &stream_device,
            config,
            device_binding.options.share_mode,
            device_binding.options.flags,
            options.flags,
            options.requirements,
        )?
    };

    let satisfied_requirements = satisfied_stream_requirements(&stream);
    ensure_stream_requirements_satisfied(
        options.requirements,
        satisfied_requirements,
        "destack.audio.stream.open",
    )?;

    let resource_id = binding.worker().resources.insert(
        binding.world(),
        ResourceEntry::new(ResourceKind::AudioStream)
            .with_label(AUDIO_STREAM_RESOURCE_LABEL)
            .with_payload(stream.clone())
            .with_finalizer(AudioStreamFinalizer::new(stream.clone())),
        Some(binding.engine()),
    );
    let stream_handle = resource::AudioStreamHandle(resource_id);

    // bind the stream handle into runtime-owned event publishing state
    let runtime_state = runtime_state(binding);
    stream.bind_runtime(&runtime_state, stream_handle);

    unsafe {
        *out = stream_handle;
    }

    Ok(())
}

/// Check one audio stream configuration for backend support.
pub(crate) unsafe fn destack_audio_stream_support(
    binding: &BindingCallContext,
    out: *mut AudioStreamSupport,
    device: resource::AudioDeviceHandle,
    config: AudioStreamConfig,
    options: AudioStreamOpenOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    validate_stream_config(config)?;
    validate_stream_open_options(options, "destack.audio.stream.support")?;

    let device_binding =
        resolve_device_host_state(binding, device, "destack.audio.stream.support")?;
    let stream_device = stream_device_from_state(&device_binding);
    validate_stream_open_options_for_backend(
        options,
        stream_device.backend,
        "destack.audio.stream.support",
    )?;

    let stream = if stream_device.is_null {
        open_null_stream(
            &stream_device,
            device_binding.opened_direction,
            config,
            device_binding.options.share_mode,
            options.flags,
            options.requirements,
        )
    } else {
        audio_platform_core::open_host_stream(
            &stream_device,
            config,
            device_binding.options.share_mode,
            device_binding.options.flags,
            options.flags,
            options.requirements,
        )?
    };

    let mut descriptor = stream_descriptor(binding, &stream);
    descriptor.requested_flags = options.flags;
    descriptor.requested_requirements = options.requirements;
    let effective_requirements = satisfied_stream_requirements(&stream);
    descriptor.effective_requirements = effective_requirements;

    let unsatisfied_requirements =
        AudioStreamRequirementFlags(options.requirements.0 & !effective_requirements.0);
    let supported = unsatisfied_requirements.0 == 0;

    unsafe {
        *out = AudioStreamSupport {
            supported,
            descriptor,
            satisfied_requirements: effective_requirements,
            unsatisfied_requirements,
        };
    }

    Ok(())
}

/// Read one packet of captured audio frames.
pub(crate) unsafe fn destack_audio_stream_read(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::AudioStreamHandle,
    maxbytes: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    if maxbytes == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "maxbytes",
            "maxbytes must be greater than zero",
        ))
        .boxed());
    }

    if maxbytes > MAX_STREAM_READ_BYTES {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "maxbytes",
            "maxbytes exceeds supported audio read limit",
        ))
        .boxed());
    }

    let resolved_binding = resolve_stream_host_state(binding, handle, "destack.audio.stream.read")?;
    ensure_capture_direction(resolved_binding.direction, "destack.audio.stream.read")?;

    let frame_size = frame_bytes(resolved_binding.requested.format, resolved_binding.channels)?;
    let maxbytes = (maxbytes as usize / frame_size) * frame_size;
    if maxbytes == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "maxbytes",
            "maxbytes must be at least one frame",
        ))
        .boxed());
    }

    let scalar_budget = maxbytes / sample_bytes(resolved_binding.requested.format);

    let mut state = resolved_binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    while state.capture_samples.is_empty() && !stream_state_is_terminal(&state) {
        state = resolved_binding
            .sync
            .wake
            .wait(state)
            .unwrap_or_else(|error| error.into_inner());
    }

    if stream_state_is_terminal(&state) {
        return Err(stream_shutdown_error("destack.audio.stream.read", &state));
    }

    let samples_to_read = state.capture_samples.len().min(scalar_budget);
    let mut samples = Vec::with_capacity(samples_to_read);
    for _ in 0..samples_to_read {
        if let Some(sample) = state.capture_samples.pop_front() {
            samples.push(sample);
        }
    }

    drop(state);
    resolved_binding.sync.wake.notify_all();

    unsafe {
        *out = binding.store_slice(encode_audio_bytes(
            &samples,
            resolved_binding.requested.format,
        ));
    }

    Ok(())
}

/// Read one packet into vectorized buffers.
pub(crate) unsafe fn destack_audio_stream_readv(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::AudioStreamHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let capacity_bytes = unsafe { vectorized_capacity_bytes(buffers)? };
    if capacity_bytes == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "buffers",
            "buffers must expose at least one writable byte",
        ))
        .boxed());
    }

    let maxbytes = capacity_bytes
        .min(MAX_STREAM_READ_BYTES as usize)
        .min(u32::MAX as usize) as u32;
    let mut packet_out = std::mem::MaybeUninit::<NativeSlice<u8>>::uninit();
    unsafe {
        destack_audio_stream_read(binding, packet_out.as_mut_ptr(), handle, maxbytes)?;
    }
    let packet = unsafe { packet_out.assume_init() };
    let packet_bytes = unsafe { packet.as_slice()? };
    let copied = unsafe { copy_into_vectorized_buffers(buffers, packet_bytes)? };

    unsafe {
        *out = copied;
    }
    Ok(())
}

/// Set one stream mute state.
pub(crate) unsafe fn destack_audio_stream_set_mute(
    binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
    muted: bool,
) -> RuntimeResult<()> {
    let resolved_binding =
        resolve_stream_host_state(binding, handle, "destack.audio.stream.setMute")?;
    ensure_stream_capability(
        resolved_binding.runtime_capabilities.supports_mute,
        "destack.audio.stream.setMute",
    )?;

    let mut state = resolved_binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    state.muted = muted;

    Ok(())
}

/// Set one stream name.
pub(crate) unsafe fn destack_audio_stream_set_name(
    binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let resolved_binding =
        resolve_stream_host_state(binding, handle, "destack.audio.stream.setName")?;
    let name = read_utf8(name, "name")?;
    let mut stream_name = resolved_binding
        .name
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    *stream_name = name;

    Ok(())
}

/// Set one stream gain multiplier.
pub(crate) unsafe fn destack_audio_stream_set_volume(
    binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
    lineargain: f64,
) -> RuntimeResult<()> {
    if !lineargain.is_finite() || lineargain < 0.0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "linearGain",
            "linearGain must be finite and non-negative",
        ))
        .boxed());
    }

    let resolved_binding =
        resolve_stream_host_state(binding, handle, "destack.audio.stream.setVolume")?;
    ensure_stream_capability(
        resolved_binding.runtime_capabilities.supports_volume,
        "destack.audio.stream.setVolume",
    )?;

    let mut state = resolved_binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    state.volume = lineargain;

    Ok(())
}

/// Start one audio stream.
pub(crate) unsafe fn destack_audio_stream_start(
    binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let resolved_binding =
        resolve_stream_host_state(binding, handle, "destack.audio.stream.start")?;

    // reject control transitions for terminal streams
    {
        let state = resolved_binding
            .sync
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if stream_state_is_terminal(&state) {
            return Err(stream_shutdown_error("destack.audio.stream.start", &state));
        }
    }

    {
        let mut state = resolved_binding
            .sync
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.state_kind = AudioStreamStateKind::Starting;
        state.running = false;
    }

    resolved_binding.sync.wake.notify_all();
    if let Err(error) = host_stream_start(&resolved_binding) {
        let mut state = resolved_binding
            .sync
            .state
            .lock()
            .unwrap_or_else(|inner| inner.into_inner());
        state.running = false;
        state.paused = false;
        state.state_kind = AudioStreamStateKind::Stopped;
        let status_flags = state.status_flags;
        drop(state);
        resolved_binding.sync.wake.notify_all();
        publish_stream_event_native(
            handle,
            &resolved_binding,
            AudioEventKind::StreamStateChanged,
            status_flags,
            0,
        );

        return Err(error);
    }

    let mut state = resolved_binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    state.running = true;
    state.paused = false;
    state.state_kind = AudioStreamStateKind::Running;
    state.last_backend_message = None;
    let status_flags = state.status_flags;
    drop(state);
    resolved_binding.sync.wake.notify_all();
    publish_stream_event_native(
        handle,
        &resolved_binding,
        AudioEventKind::StreamStateChanged,
        status_flags,
        0,
    );

    Ok(())
}

/// Pause or resume one audio stream.
pub(crate) unsafe fn destack_audio_stream_pause(
    binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
    pause: bool,
) -> RuntimeResult<()> {
    let resolved_binding =
        resolve_stream_host_state(binding, handle, "destack.audio.stream.pause")?;

    // reject control transitions for terminal streams
    {
        let state = resolved_binding
            .sync
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if stream_state_is_terminal(&state) {
            return Err(stream_shutdown_error("destack.audio.stream.pause", &state));
        }
    }

    ensure_stream_capability(
        resolved_binding.runtime_capabilities.supports_pause,
        "destack.audio.stream.pause",
    )?;
    host_stream_pause(&resolved_binding, pause)?;

    let mut state = resolved_binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    state.paused = pause;
    state.running = !pause;
    state.state_kind = if pause {
        AudioStreamStateKind::Paused
    } else {
        AudioStreamStateKind::Running
    };
    let status_flags = state.status_flags;
    drop(state);
    resolved_binding.sync.wake.notify_all();
    publish_stream_event_native(
        handle,
        &resolved_binding,
        AudioEventKind::StreamStateChanged,
        status_flags,
        0,
    );

    Ok(())
}

/// Abort one audio stream immediately.
pub(crate) unsafe fn destack_audio_stream_abort(
    binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let resolved_binding =
        resolve_stream_host_state(binding, handle, "destack.audio.stream.abort")?;

    // reject control transitions for terminal streams
    {
        let state = resolved_binding
            .sync
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if stream_state_is_terminal(&state) {
            return Err(stream_shutdown_error("destack.audio.stream.abort", &state));
        }
    }

    host_stream_stop(&resolved_binding)?;

    let mut state = resolved_binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    state.running = false;
    state.paused = false;
    state.state_kind = AudioStreamStateKind::Stopped;
    state.playback_samples.clear();
    state.capture_samples.clear();
    let status_flags = state.status_flags;
    drop(state);
    resolved_binding.sync.wake.notify_all();
    publish_stream_event_native(
        handle,
        &resolved_binding,
        AudioEventKind::StreamStateChanged,
        status_flags,
        0,
    );

    Ok(())
}

/// Read one stream state.
pub(crate) unsafe fn destack_audio_stream_state(
    binding: &BindingCallContext,
    out: *mut AudioStreamState,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let resolved_binding =
        resolve_stream_host_state(binding, handle, "destack.audio.stream.state")?;
    unsafe {
        *out = stream_state_snapshot(&resolved_binding);
    }

    Ok(())
}

/// Stop one audio stream.
pub(crate) unsafe fn destack_audio_stream_stop(
    binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let resolved_binding = resolve_stream_host_state(binding, handle, "destack.audio.stream.stop")?;

    // reject control transitions for terminal streams
    {
        let state = resolved_binding
            .sync
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if stream_state_is_terminal(&state) {
            return Err(stream_shutdown_error("destack.audio.stream.stop", &state));
        }
    }

    host_stream_stop(&resolved_binding)?;

    {
        let mut state = resolved_binding
            .sync
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.state_kind = AudioStreamStateKind::Stopping;
        state.running = false;
    }

    resolved_binding.sync.wake.notify_all();

    let mut state = resolved_binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    state.running = false;
    state.paused = false;
    state.state_kind = AudioStreamStateKind::Stopped;
    let status_flags = state.status_flags;
    drop(state);
    resolved_binding.sync.wake.notify_all();
    publish_stream_event_native(
        handle,
        &resolved_binding,
        AudioEventKind::StreamStateChanged,
        status_flags,
        0,
    );

    Ok(())
}

/// Read one stream timing sample.
pub(crate) unsafe fn destack_audio_stream_timing(
    binding: &BindingCallContext,
    out: *mut AudioStreamTiming,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let resolved_binding =
        resolve_stream_host_state(binding, handle, "destack.audio.stream.timing")?;
    unsafe {
        *out = stream_timing_snapshot(binding, &resolved_binding);
    }

    Ok(())
}

/// Try to read one packet of captured audio frames without blocking.
pub(crate) unsafe fn destack_audio_stream_try_read(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::AudioStreamHandle,
    maxbytes: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    if maxbytes == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "maxbytes",
            "maxbytes must be greater than zero",
        ))
        .boxed());
    }

    let resolved_binding =
        resolve_stream_host_state(binding, handle, "destack.audio.stream.tryRead")?;
    ensure_capture_direction(resolved_binding.direction, "destack.audio.stream.tryRead")?;

    let frame_size = frame_bytes(resolved_binding.requested.format, resolved_binding.channels)?;
    let maxbytes = (maxbytes as usize / frame_size) * frame_size;
    if maxbytes == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "maxbytes",
            "maxbytes must be at least one frame",
        ))
        .boxed());
    }

    let scalar_budget = maxbytes / sample_bytes(resolved_binding.requested.format);

    let mut state = resolved_binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    if stream_state_is_terminal(&state) {
        return Err(stream_shutdown_error(
            "destack.audio.stream.tryRead",
            &state,
        ));
    }

    if state.capture_samples.is_empty() {
        state.input_underflow_count = state.input_underflow_count.saturating_add(1);
        state.status_flags =
            AudioStreamStatusFlags(state.status_flags.0 | STREAM_STATUS_INPUT_UNDERFLOW.0);
        return Err(audio_would_block(
            "destack.audio.stream.tryRead",
            "no captured audio is currently available",
        ));
    }

    let samples_to_read = state.capture_samples.len().min(scalar_budget);
    let mut samples = Vec::with_capacity(samples_to_read);
    for _ in 0..samples_to_read {
        if let Some(sample) = state.capture_samples.pop_front() {
            samples.push(sample);
        }
    }

    drop(state);
    resolved_binding.sync.wake.notify_all();

    unsafe {
        *out = binding.store_slice(encode_audio_bytes(
            &samples,
            resolved_binding.requested.format,
        ));
    }

    Ok(())
}

/// Try to read one packet into vectorized buffers without blocking.
pub(crate) unsafe fn destack_audio_stream_try_readv(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::AudioStreamHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let capacity_bytes = unsafe { vectorized_capacity_bytes(buffers)? };
    if capacity_bytes == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "buffers",
            "buffers must expose at least one writable byte",
        ))
        .boxed());
    }

    let maxbytes = capacity_bytes
        .min(MAX_STREAM_READ_BYTES as usize)
        .min(u32::MAX as usize) as u32;
    let mut packet_out = std::mem::MaybeUninit::<NativeSlice<u8>>::uninit();
    unsafe {
        destack_audio_stream_try_read(binding, packet_out.as_mut_ptr(), handle, maxbytes)?;
    }
    let packet = unsafe { packet_out.assume_init() };
    let packet_bytes = unsafe { packet.as_slice()? };
    let copied = unsafe { copy_into_vectorized_buffers(buffers, packet_bytes)? };

    unsafe {
        *out = copied;
    }
    Ok(())
}

/// Try to write one packet of audio frames without blocking.
pub(crate) unsafe fn destack_audio_stream_try_write(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::AudioStreamHandle,
    data: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let resolved_binding =
        resolve_stream_host_state(binding, handle, "destack.audio.stream.tryWrite")?;
    ensure_playback_direction(resolved_binding.direction, "destack.audio.stream.tryWrite")?;

    let input = unsafe { data.as_slice()? };
    let decoded = decode_audio_bytes(input, resolved_binding.requested.format)?;

    let mut state = resolved_binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    if stream_state_is_terminal(&state) {
        return Err(stream_shutdown_error(
            "destack.audio.stream.tryWrite",
            &state,
        ));
    }

    let free = resolved_binding
        .playback_capacity_samples()
        .saturating_sub(state.playback_samples.len());
    if free == 0 {
        state.output_overflow_count = state.output_overflow_count.saturating_add(1);
        state.status_flags =
            AudioStreamStatusFlags(state.status_flags.0 | STREAM_STATUS_OUTPUT_OVERFLOW.0);
        return Err(audio_would_block(
            "destack.audio.stream.tryWrite",
            "playback buffer has no writable space",
        ));
    }

    let written_samples = decoded.len().min(free);
    for sample in decoded.into_iter().take(written_samples) {
        state.playback_samples.push_back(sample);
    }

    drop(state);
    resolved_binding.sync.wake.notify_all();

    let bytes =
        written_samples.saturating_mul(sample_bytes(resolved_binding.requested.format)) as u64;
    unsafe {
        *out = bytes;
    }

    Ok(())
}

/// Try to write one packet from vectorized buffers without blocking.
pub(crate) unsafe fn destack_audio_stream_try_writev(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::AudioStreamHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let flattened = unsafe { flatten_vectorized_buffers(buffers)? };
    unsafe { destack_audio_stream_try_write(binding, out, handle, binding.store_slice(flattened)) }
}

/// Write one packet of audio frames.
pub(crate) unsafe fn destack_audio_stream_write(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::AudioStreamHandle,
    data: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let resolved_binding =
        resolve_stream_host_state(binding, handle, "destack.audio.stream.write")?;
    ensure_playback_direction(resolved_binding.direction, "destack.audio.stream.write")?;

    let input = unsafe { data.as_slice()? };
    let decoded = decode_audio_bytes(input, resolved_binding.requested.format)?;

    let mut offset = 0usize;
    let mut state = resolved_binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    while offset < decoded.len() {
        while state.playback_samples.len() >= resolved_binding.playback_capacity_samples()
            && !stream_state_is_terminal(&state)
        {
            state = resolved_binding
                .sync
                .wake
                .wait(state)
                .unwrap_or_else(|error| error.into_inner());
        }

        if stream_state_is_terminal(&state) {
            return Err(stream_shutdown_error("destack.audio.stream.write", &state));
        }

        let free = resolved_binding
            .playback_capacity_samples()
            .saturating_sub(state.playback_samples.len());
        if free == 0 {
            continue;
        }

        let writable = (decoded.len() - offset).min(free);
        for sample in decoded[offset..offset + writable].iter().copied() {
            state.playback_samples.push_back(sample);
        }
        offset += writable;

        drop(state);
        resolved_binding.sync.wake.notify_all();
        state = resolved_binding
            .sync
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
    }

    drop(state);
    let bytes = decoded
        .len()
        .saturating_mul(sample_bytes(resolved_binding.requested.format)) as u64;
    unsafe {
        *out = bytes;
    }

    Ok(())
}

/// Write one packet from vectorized buffers.
pub(crate) unsafe fn destack_audio_stream_writev(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::AudioStreamHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let flattened = unsafe { flatten_vectorized_buffers(buffers)? };
    unsafe { destack_audio_stream_write(binding, out, handle, binding.store_slice(flattened)) }
}

/// Write one packet for one target presentation time.
pub(crate) unsafe fn destack_audio_stream_write_at(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::AudioStreamHandle,
    data: NativeSlice<u8>,
    presentationtimens: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let resolved_binding =
        resolve_stream_host_state(binding, handle, "destack.audio.stream.writeAt")?;
    ensure_playback_direction(resolved_binding.direction, "destack.audio.stream.writeAt")?;
    ensure_stream_capability(
        resolved_binding.runtime_capabilities.supports_write_at,
        "destack.audio.stream.writeAt",
    )?;

    wait_for_stream_presentation_time(
        binding,
        &resolved_binding,
        "destack.audio.stream.writeAt",
        presentationtimens,
    )?;

    unsafe { destack_audio_stream_write(binding, out, handle, data) }
}

/// Write one vectorized packet for one target presentation time.
pub(crate) unsafe fn destack_audio_stream_write_atv(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::AudioStreamHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    presentationtimens: u64,
) -> RuntimeResult<()> {
    let flattened = unsafe { flatten_vectorized_buffers(buffers)? };
    unsafe {
        destack_audio_stream_write_at(
            binding,
            out,
            handle,
            binding.store_slice(flattened),
            presentationtimens,
        )
    }
}
