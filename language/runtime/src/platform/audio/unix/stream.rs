use std::time::Duration;

use super::core as audio_platform_core;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::audio::{
    AudioEventKind, AudioStreamAvailability, AudioStreamConfig, AudioStreamDescriptor,
    AudioStreamOpenOptions, AudioStreamRequirementFlags, AudioStreamState, AudioStreamStateKind,
    AudioStreamStatusFlags, AudioStreamSupport, AudioStreamTiming, core as audio_core,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{PlatformError, core as core_platform, resource};
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
///
/// Read one point-in-time sample of immediately readable and writable frame counts.
/// Values are advisory and can change immediately after read.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream-space query operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_availability(
    binding: &BindingCallContext,
    out: *mut AudioStreamAvailability,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let resolved_binding = audio_core::resolve_stream_host_state(
        binding,
        handle,
        "destack.audio.stream.availability",
    )?;
    unsafe {
        *out = audio_core::stream_availability_snapshot(binding, &resolved_binding);
    }

    Ok(())
}

/// Close one audio stream.
///
/// Close one host audio stream and release backend buffers and synchronization state.
/// Stream handle becomes invalid after close completes.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream close operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_close(
    binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let stream =
        audio_core::resolve_stream_host_state(binding, handle, "destack.audio.stream.close")?;
    stream.unbind_runtime();

    if !binding.worker().resources.remove_and_finalize(
        &binding.world(),
        handle.0,
        Some(binding.engine()),
    ) {
        return Err(core_platform::io_not_found(
            "destack.audio.stream.close",
            format!("unknown audio stream handle {}", handle.0.0),
        ));
    }

    Ok(())
}

/// Drain one playback stream.
///
/// Wait for one playback stream to consume currently queued samples.
/// Drain timeout is expressed in nanoseconds in the runtime monotonic domain.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available drain or synchronized-stop operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_drain(
    binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let resolved_binding =
        audio_core::resolve_stream_host_state(binding, handle, "destack.audio.stream.drain")?;
    audio_core::ensure_playback_direction(
        resolved_binding.direction,
        "destack.audio.stream.drain",
    )?;

    let deadline = audio_core::host_monotonic_nanos().saturating_add(timeoutns);
    let mut state = resolved_binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    loop {
        if state.playback_samples.is_empty() {
            return Ok(());
        }

        if timeoutns == 0 || audio_core::host_monotonic_nanos() >= deadline {
            return Err(audio_core::audio_would_block(
                "destack.audio.stream.drain",
                "stream drain timed out",
            ));
        }

        let remaining = deadline.saturating_sub(audio_core::host_monotonic_nanos());
        let duration = Duration::from_nanos(remaining.max(1));
        let wait = resolved_binding
            .sync
            .wake
            .wait_timeout(state, duration)
            .unwrap_or_else(|error| error.into_inner());
        state = wait.0;
    }
}

/// Flush buffered stream data.
///
/// Drop pending buffered data for one stream without closing it.
/// Flushing semantics are backend-defined for capture and duplex streams.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream flush or reset operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_flush(
    binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let resolved_binding =
        audio_core::resolve_stream_host_state(binding, handle, "destack.audio.stream.flush")?;
    audio_core::host_stream_flush(&resolved_binding)?;

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
///
/// Read one normalized view of negotiated stream parameters and backend mode.
/// Values reflect backend negotiation outcomes and can differ from open-time requests.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream-parameter query operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_descriptor(
    binding: &BindingCallContext,
    out: *mut AudioStreamDescriptor,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let resolved_binding =
        audio_core::resolve_stream_host_state(binding, handle, "destack.audio.stream.descriptor")?;
    unsafe {
        *out = audio_core::stream_descriptor(binding, &resolved_binding);
    }

    Ok(())
}

/// Open one audio stream on one device.
///
/// Create one host audio stream with explicit sample format, channel, and period configuration.
/// Open options carry optional tuning hints and strict requirement lanes.
/// Any unsatisfied requirement must fail open with `notSupported`.
/// Buffering and latency behavior follow host backend contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream creation APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
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

    audio_core::validate_stream_config(config)?;
    audio_core::validate_stream_open_options(options, "destack.audio.stream.open")?;

    let device_binding =
        audio_core::resolve_device_host_state(binding, device, "destack.audio.stream.open")?;
    let stream_device = audio_core::stream_device_from_state(&device_binding);
    audio_core::validate_stream_open_options_for_backend(
        options,
        stream_device.backend,
        "destack.audio.stream.open",
    )?;

    let stream = if stream_device.is_null {
        audio_core::open_null_stream(
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

    let satisfied_requirements = audio_core::satisfied_stream_requirements(&stream);
    audio_core::ensure_stream_requirements_satisfied(
        options.requirements,
        satisfied_requirements,
        "destack.audio.stream.open",
    )?;

    let resource_id = binding.worker().resources.insert(
        &binding.world(),
        ResourceEntry::new(ResourceKind::AudioStream)
            .with_label(audio_core::AUDIO_STREAM_RESOURCE_LABEL)
            .with_payload(stream.clone())
            .with_finalizer(audio_core::AudioStreamFinalizer::new(stream.clone())),
        Some(binding.engine()),
    );
    let stream_handle = resource::AudioStreamHandle(resource_id);

    // bind the stream handle into runtime-owned event publishing state
    let runtime_state = audio_core::runtime_state(binding);
    stream.bind_runtime(&runtime_state, stream_handle);

    unsafe {
        *out = stream_handle;
    }

    Ok(())
}

/// Check one audio stream configuration for backend support.
///
/// Check one stream configuration and return backend negotiation results without opening one long-lived stream handle.
/// Requirement flags are resolved into `satisfiedRequirements` and `unsatisfiedRequirements`.
///
/// # Platform
/// Unix and Windows.
/// Mirrors PortAudio `Pa_IsFormatSupported` intent and miniaudio native-format probing behavior.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
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

    audio_core::validate_stream_config(config)?;
    audio_core::validate_stream_open_options(options, "destack.audio.stream.support")?;

    let device_binding =
        audio_core::resolve_device_host_state(binding, device, "destack.audio.stream.support")?;
    let stream_device = audio_core::stream_device_from_state(&device_binding);
    audio_core::validate_stream_open_options_for_backend(
        options,
        stream_device.backend,
        "destack.audio.stream.support",
    )?;

    let stream = if stream_device.is_null {
        audio_core::open_null_stream(
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

    let mut descriptor = audio_core::stream_descriptor(binding, &stream);
    descriptor.requested_flags = options.flags;
    descriptor.requested_requirements = options.requirements;
    let effective_requirements = audio_core::satisfied_stream_requirements(&stream);
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
///
/// Read one packet of captured interleaved audio frames from the capture stream.
/// Packet sizing and buffering follow host backend capture contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream read or capture-client operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.capture`.
///
/// # Replay
/// External, recordable.
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

    let max_read_bytes = audio_core::resolved_max_stream_read_bytes(binding);
    if maxbytes > max_read_bytes {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "maxbytes",
            "maxbytes exceeds supported audio read limit",
        ))
        .boxed());
    }

    let resolved_binding =
        audio_core::resolve_stream_host_state(binding, handle, "destack.audio.stream.read")?;
    audio_core::ensure_capture_direction(resolved_binding.direction, "destack.audio.stream.read")?;

    let frame_size =
        audio_core::frame_bytes(resolved_binding.requested.format, resolved_binding.channels)?;
    let maxbytes = (maxbytes as usize / frame_size) * frame_size;
    if maxbytes == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "maxbytes",
            "maxbytes must be at least one frame",
        ))
        .boxed());
    }

    let scalar_budget = maxbytes / audio_core::sample_bytes(resolved_binding.requested.format);

    let mut state = resolved_binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    while state.capture_samples.is_empty() && !audio_core::stream_state_is_terminal(&state) {
        state = resolved_binding
            .sync
            .wake
            .wait(state)
            .unwrap_or_else(|error| error.into_inner());
    }

    if audio_core::stream_state_is_terminal(&state) {
        return Err(audio_core::stream_shutdown_error(
            "destack.audio.stream.read",
            &state,
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
        *out = binding.store_slice(audio_core::encode_audio_bytes(
            &samples,
            resolved_binding.requested.format,
        ));
    }

    Ok(())
}

/// Read one packet into vectorized buffers.
///
/// Read one packet of captured audio frames into multiple byte slices.
/// Buffers can represent segmented interleaved payloads or channel planes when non-interleaved mode is active.
///
/// # Platform
/// Unix and Windows.
/// Mirrors readv-style capture behavior and backend non-interleaved lanes where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.capture`.
///
/// # Replay
/// External, recordable.
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

    let max_read_bytes = audio_core::resolved_max_stream_read_bytes(binding) as usize;
    let maxbytes = capacity_bytes.min(max_read_bytes).min(u32::MAX as usize) as u32;
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
///
/// Apply one mute state for one stream processing lane.
/// This controls stream-level mute and does not imply global endpoint mute ownership.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO stream-level mute paths where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_set_mute(
    binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
    muted: bool,
) -> RuntimeResult<()> {
    let resolved_binding =
        audio_core::resolve_stream_host_state(binding, handle, "destack.audio.stream.setMute")?;
    audio_core::ensure_stream_capability(
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
///
/// Apply one stream label for host mixers and diagnostics where supported.
/// Backend label visibility and truncation follow host policy.
///
/// # Platform
/// Unix and Windows.
/// Mirrors cubeb `cubeb_stream_set_name` behavior where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_set_name(
    binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let resolved_binding =
        audio_core::resolve_stream_host_state(binding, handle, "destack.audio.stream.setName")?;
    let name = audio_core::read_utf8(name, "name")?;
    let mut stream_name = resolved_binding
        .name
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    *stream_name = name;

    Ok(())
}

/// Set one stream gain multiplier.
///
/// Apply one linear gain multiplier for one stream processing lane.
/// This controls stream-level gain and does not imply global endpoint mixer ownership.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO stream-level gain paths where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
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
        audio_core::resolve_stream_host_state(binding, handle, "destack.audio.stream.setVolume")?;
    audio_core::ensure_stream_capability(
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
///
/// Transition one opened stream to running state and begin host DMA or scheduler processing.
/// Start timing follows host backend scheduling semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream start operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_start(
    binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let resolved_binding =
        audio_core::resolve_stream_host_state(binding, handle, "destack.audio.stream.start")?;

    // reject control transitions for terminal streams
    {
        let state = resolved_binding
            .sync
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if audio_core::stream_state_is_terminal(&state) {
            return Err(audio_core::stream_shutdown_error(
                "destack.audio.stream.start",
                &state,
            ));
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
    if let Err(error) = audio_core::host_stream_start(&resolved_binding) {
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
        audio_core::publish_stream_event_native(
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
    audio_core::publish_stream_event_native(
        handle,
        &resolved_binding,
        AudioEventKind::StreamStateChanged,
        status_flags,
        0,
    );

    Ok(())
}

/// Pause or resume one audio stream.
///
/// Transition one running stream into paused state and back.
/// Pause support is backend-dependent.
///
/// # Platform
/// Unix and Windows.
/// Mirrors libsoundio `soundio_outstream_pause` and SDL stream-device pause semantics.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_pause(
    binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
    pause: bool,
) -> RuntimeResult<()> {
    let resolved_binding =
        audio_core::resolve_stream_host_state(binding, handle, "destack.audio.stream.pause")?;

    // reject control transitions for terminal streams
    {
        let state = resolved_binding
            .sync
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if audio_core::stream_state_is_terminal(&state) {
            return Err(audio_core::stream_shutdown_error(
                "destack.audio.stream.pause",
                &state,
            ));
        }
    }

    audio_core::ensure_stream_capability(
        resolved_binding.runtime_capabilities.supports_pause,
        "destack.audio.stream.pause",
    )?;
    audio_core::host_stream_pause(&resolved_binding, pause)?;

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
    audio_core::publish_stream_event_native(
        handle,
        &resolved_binding,
        AudioEventKind::StreamStateChanged,
        status_flags,
        0,
    );

    Ok(())
}

/// Abort one audio stream immediately.
///
/// Request one immediate stream stop without graceful drain.
/// Pending buffered data can be discarded.
///
/// # Platform
/// Unix and Windows.
/// Mirrors PortAudio `Pa_AbortStream` semantics.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_abort(
    binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let resolved_binding =
        audio_core::resolve_stream_host_state(binding, handle, "destack.audio.stream.abort")?;

    // reject control transitions for terminal streams
    {
        let state = resolved_binding
            .sync
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if audio_core::stream_state_is_terminal(&state) {
            return Err(audio_core::stream_shutdown_error(
                "destack.audio.stream.abort",
                &state,
            ));
        }
    }

    audio_core::host_stream_stop(&resolved_binding)?;

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
    audio_core::publish_stream_event_native(
        handle,
        &resolved_binding,
        AudioEventKind::StreamStateChanged,
        status_flags,
        0,
    );

    Ok(())
}

/// Read one stream state.
///
/// Read one point-in-time state sample of stream run state and backend buffering metrics.
/// State values are advisory and can change immediately after read.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream query primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_state(
    binding: &BindingCallContext,
    out: *mut AudioStreamState,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let resolved_binding =
        audio_core::resolve_stream_host_state(binding, handle, "destack.audio.stream.state")?;
    unsafe {
        *out = audio_core::stream_state_snapshot(&resolved_binding);
    }

    Ok(())
}

/// Stop one audio stream.
///
/// Transition one running stream to stopped state and flush host backend scheduling.
/// Buffered frames can be discarded based on host backend semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream stop operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_stop(
    binding: &BindingCallContext,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    let resolved_binding =
        audio_core::resolve_stream_host_state(binding, handle, "destack.audio.stream.stop")?;

    // reject control transitions for terminal streams
    {
        let state = resolved_binding
            .sync
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if audio_core::stream_state_is_terminal(&state) {
            return Err(audio_core::stream_shutdown_error(
                "destack.audio.stream.stop",
                &state,
            ));
        }
    }

    audio_core::host_stream_stop(&resolved_binding)?;

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
    audio_core::publish_stream_event_native(
        handle,
        &resolved_binding,
        AudioEventKind::StreamStateChanged,
        status_flags,
        0,
    );

    Ok(())
}

/// Read one stream timing sample.
///
/// Read one full timing sample that correlates stream position and available backend clocks.
/// Missing optional lanes are reported through `has*` fields instead of `notSupported`.
/// Timing values are intended for drift correction and synchronization.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream-clock query operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_stream_timing(
    binding: &BindingCallContext,
    out: *mut AudioStreamTiming,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let resolved_binding =
        audio_core::resolve_stream_host_state(binding, handle, "destack.audio.stream.timing")?;
    unsafe {
        *out = audio_core::stream_timing_snapshot(binding, &resolved_binding);
    }

    Ok(())
}

/// Try to read one packet of captured audio frames without blocking.
///
/// Read one packet of captured interleaved audio frames without waiting.
/// Empty input state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available nonblocking stream read operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.capture`.
///
/// # Replay
/// External, recordable.
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
        audio_core::resolve_stream_host_state(binding, handle, "destack.audio.stream.tryRead")?;
    audio_core::ensure_capture_direction(
        resolved_binding.direction,
        "destack.audio.stream.tryRead",
    )?;

    let frame_size =
        audio_core::frame_bytes(resolved_binding.requested.format, resolved_binding.channels)?;
    let maxbytes = (maxbytes as usize / frame_size) * frame_size;
    if maxbytes == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "maxbytes",
            "maxbytes must be at least one frame",
        ))
        .boxed());
    }

    let scalar_budget = maxbytes / audio_core::sample_bytes(resolved_binding.requested.format);

    let mut state = resolved_binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    if audio_core::stream_state_is_terminal(&state) {
        return Err(audio_core::stream_shutdown_error(
            "destack.audio.stream.tryRead",
            &state,
        ));
    }

    if state.capture_samples.is_empty() {
        state.input_underflow_count = state.input_underflow_count.saturating_add(1);
        state.status_flags = AudioStreamStatusFlags(
            state.status_flags.0 | audio_core::STREAM_STATUS_INPUT_UNDERFLOW.0,
        );
        return Err(audio_core::audio_would_block(
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
        *out = binding.store_slice(audio_core::encode_audio_bytes(
            &samples,
            resolved_binding.requested.format,
        ));
    }

    Ok(())
}

/// Try to read one packet into vectorized buffers without blocking.
///
/// Read one packet of captured audio frames into multiple byte slices without waiting.
/// Empty input state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Mirrors nonblocking readv-style capture behavior.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.capture`.
///
/// # Replay
/// External, recordable.
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

    let max_read_bytes = audio_core::resolved_max_stream_read_bytes(binding) as usize;
    let maxbytes = capacity_bytes.min(max_read_bytes).min(u32::MAX as usize) as u32;
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
///
/// Submit one packet of interleaved audio frames to the playback stream without waiting.
/// Empty output space is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available nonblocking stream write operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
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
        audio_core::resolve_stream_host_state(binding, handle, "destack.audio.stream.tryWrite")?;
    audio_core::ensure_playback_direction(
        resolved_binding.direction,
        "destack.audio.stream.tryWrite",
    )?;

    let input = unsafe { data.as_slice()? };
    let decoded = audio_core::decode_audio_bytes(input, resolved_binding.requested.format)?;

    let mut state = resolved_binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    if audio_core::stream_state_is_terminal(&state) {
        return Err(audio_core::stream_shutdown_error(
            "destack.audio.stream.tryWrite",
            &state,
        ));
    }

    let free = resolved_binding
        .playback_capacity_samples()
        .saturating_sub(state.playback_samples.len());
    if free == 0 {
        state.output_overflow_count = state.output_overflow_count.saturating_add(1);
        state.status_flags = AudioStreamStatusFlags(
            state.status_flags.0 | audio_core::STREAM_STATUS_OUTPUT_OVERFLOW.0,
        );
        return Err(audio_core::audio_would_block(
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

    let bytes = written_samples
        .saturating_mul(audio_core::sample_bytes(resolved_binding.requested.format))
        as u64;
    unsafe {
        *out = bytes;
    }

    Ok(())
}

/// Try to write one packet from vectorized buffers without blocking.
///
/// Submit one packet of audio frames from multiple byte slices without waiting.
/// Empty output space is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Mirrors nonblocking writev-style submission behavior.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
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
///
/// Submit one packet of interleaved audio frames to the playback stream.
/// Short writes can occur when host buffers are near capacity.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream write or render-client operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
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
        audio_core::resolve_stream_host_state(binding, handle, "destack.audio.stream.write")?;
    audio_core::ensure_playback_direction(
        resolved_binding.direction,
        "destack.audio.stream.write",
    )?;

    let input = unsafe { data.as_slice()? };
    let decoded = audio_core::decode_audio_bytes(input, resolved_binding.requested.format)?;

    let mut offset = 0usize;
    let mut state = resolved_binding
        .sync
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    while offset < decoded.len() {
        while state.playback_samples.len() >= resolved_binding.playback_capacity_samples()
            && !audio_core::stream_state_is_terminal(&state)
        {
            state = resolved_binding
                .sync
                .wake
                .wait(state)
                .unwrap_or_else(|error| error.into_inner());
        }

        if audio_core::stream_state_is_terminal(&state) {
            return Err(audio_core::stream_shutdown_error(
                "destack.audio.stream.write",
                &state,
            ));
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
        .saturating_mul(audio_core::sample_bytes(resolved_binding.requested.format))
        as u64;
    unsafe {
        *out = bytes;
    }

    Ok(())
}

/// Write one packet from vectorized buffers.
///
/// Submit one packet of audio frames from multiple byte slices.
/// Buffers can represent segmented interleaved payloads or channel planes when non-interleaved mode is active.
///
/// # Platform
/// Unix and Windows.
/// Mirrors writev-style submission behavior and backend non-interleaved lanes where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
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
///
/// Submit one packet of interleaved audio frames for one target presentation timestamp.
/// Scheduling precision depends on host backend timing guarantees.
/// This is one optional scheduling lane and can return `notSupported` when unavailable.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available scheduled-render operations when available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.playback.schedule`.
///
/// # Replay
/// External, recordable.
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
        audio_core::resolve_stream_host_state(binding, handle, "destack.audio.stream.writeAt")?;
    audio_core::ensure_playback_direction(
        resolved_binding.direction,
        "destack.audio.stream.writeAt",
    )?;
    audio_core::ensure_stream_capability(
        resolved_binding.runtime_capabilities.supports_write_at,
        "destack.audio.stream.writeAt",
    )?;

    audio_core::wait_for_stream_presentation_time(
        binding,
        &resolved_binding,
        "destack.audio.stream.writeAt",
        presentationtimens,
    )?;

    unsafe { destack_audio_stream_write(binding, out, handle, data) }
}

/// Write one vectorized packet for one target presentation time.
///
/// Submit one packet of audio frames from multiple byte slices for one target presentation timestamp.
/// Buffers can represent segmented interleaved payloads or channel planes when non-interleaved mode is active.
/// Scheduling precision depends on host backend timing guarantees.
///
/// # Platform
/// Unix and Windows.
/// Mirrors scheduled-render operations where available and extends them for writev-style payload submission.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.playback.schedule`.
///
/// # Replay
/// External, recordable.
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
