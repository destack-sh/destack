use super::super::{
    AudioBackend, AudioBackendSelectionPolicy, AudioChannelLayout, AudioClockDomain,
    AudioDeviceDirection, AudioDeviceOpenFlags, AudioDeviceOpenOptions, AudioEventDeliveryMode,
    AudioEventOverflowPolicy, AudioEventSubscriptionOptions, AudioSampleFormat, AudioShareMode,
    AudioStreamConfig, AudioStreamFlags, AudioStreamOpenOptions, AudioStreamRequirementFlags,
    AudioStreamStateKind, AudioStreamTransferMode, core as audio_core,
};
use super::{
    AudioHarnessContext, assert_code_is_not_not_supported, assert_not_supported_result,
    assert_ok_or_expected_error, assert_platform_error_code, core, error_code_from_runtime_error,
    is_not_supported_code, with_harness_context,
};
use crate::diagnostic::RuntimeResult;
use crate::platform::core::BackendSupport;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{AudioDeviceHandle, AudioEventHandle, AudioStreamHandle};
use audio_core::{
    BACKEND_CAPABILITY_DEVICE_CLOCK, BACKEND_CAPABILITY_EXCLUSIVE_MODE,
    BACKEND_CAPABILITY_NON_INTERLEAVED, BACKEND_CAPABILITY_SHARED_MODE, EVENT_SUBSCRIBE_STREAM,
    MIN_EVENT_POLL_INTERVAL_NS, STREAM_FLAG_MINIMIZE_LATENCY, STREAM_FLAG_NON_INTERLEAVED,
    STREAM_REQUIRE_SCHEDULED_WRITE,
};
use core::{
    DeterministicSequence, backend_is_available_for_host_execution, backend_support_rows,
    backend_support_rows_with_capabilities, byte_len, event_batch_sequence_rows, harness_bytes,
    harness_bytes_slices, harness_device_options, harness_event_options,
    harness_mutable_bytes_slices, harness_stream_config, harness_stream_options, harness_string,
    open_null_duplex_stream, stream_descriptor_flags_from_value, stream_open_with_default_options,
    stream_state_from_value, stream_support_from_value, string_from_harness_value,
};

const RANDOM_NULL_INTERLEAVING_ITERATIONS: usize = 128;
const RANDOM_HOST_INTERLEAVING_ITERATIONS: usize = 96;
const RANDOM_STREAM_IO_BYTES: usize = 256;
const STREAM_STOP_ALLOWED_ERRORS: [PlatformErrorCode; 4] = [
    PlatformErrorCode::IoWouldBlock,
    PlatformErrorCode::IoInterrupted,
    PlatformErrorCode::IoBusy,
    PlatformErrorCode::NotSupported,
];
const STREAM_IO_TRANSIENT_ALLOWED_ERRORS: [PlatformErrorCode; 6] = [
    PlatformErrorCode::IoWouldBlock,
    PlatformErrorCode::IoInterrupted,
    PlatformErrorCode::IoBusy,
    PlatformErrorCode::NotSupported,
    PlatformErrorCode::AudioUnavailable,
    PlatformErrorCode::DeviceUnavailable,
];
const STREAM_OPEN_OPTIONAL_ERRORS: [PlatformErrorCode; 3] = [
    PlatformErrorCode::NotSupported,
    PlatformErrorCode::IoInvalidData,
    PlatformErrorCode::IoNotFound,
];
const STREAM_START_OPTIONAL_ERRORS: [PlatformErrorCode; 3] = [
    PlatformErrorCode::NotSupported,
    PlatformErrorCode::IoInvalidData,
    PlatformErrorCode::IoInterrupted,
];

/// Randomized interleaving operation for stream and event stress tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StreamInterleavingTask {
    /// Open stream resources.
    Open,
    /// Close stream resources.
    Close,
    /// Toggle running state with start or stop.
    ToggleRunning,
    /// Attempt one nonblocking write.
    TryWrite,
    /// Attempt one nonblocking read.
    TryRead,
    /// Toggle stream event subscription open or close.
    ToggleEventSubscription,
    /// Attempt one nonblocking event batch read.
    TryReadEventBatch,
    /// Sample stream state.
    ReadState,
}

impl StreamInterleavingTask {
    /// Draw one deterministic operation from one random sequence.
    fn draw(random: &mut DeterministicSequence) -> Self {
        match random.next_index(8) {
            0 => Self::Open,
            1 => Self::Close,
            2 => Self::ToggleRunning,
            3 => Self::TryWrite,
            4 => Self::TryRead,
            5 => Self::ToggleEventSubscription,
            6 => Self::TryReadEventBatch,
            _ => Self::ReadState,
        }
    }
}

/// Build one stream event subscription for one backend and stream.
fn stream_event_options(
    backend: AudioBackend,
    stream: AudioStreamHandle,
) -> AudioEventSubscriptionOptions {
    AudioEventSubscriptionOptions {
        backend,
        backend_policy: AudioBackendSelectionPolicy::Strict,
        flags: EVENT_SUBSCRIBE_STREAM,
        delivery_mode: AudioEventDeliveryMode::Auto,
        overflow_policy: AudioEventOverflowPolicy::DropOldest,
        stream: Some(stream),
        queue_capacity: 2,
        poll_interval_ns: MIN_EVENT_POLL_INTERVAL_NS,
    }
}

/// Build one deterministic byte payload for read or write operations.
fn random_payload(random: &mut DeterministicSequence, len: usize) -> Vec<u8> {
    (0..len)
        .map(|_| (random.next_u64() & 0xff) as u8)
        .collect::<Vec<_>>()
}

/// Assert one decoded event sequence row batch is monotonic with rolling state.
fn assert_event_rows_monotonic(
    rows: &[(u64, u64)],
    last_sequence: &mut u64,
    last_dropped_count: &mut u64,
) {
    for (sequence, dropped_count) in rows {
        assert!(
            *sequence > *last_sequence,
            "event sequence should be strictly increasing",
        );
        assert!(
            *dropped_count >= *last_dropped_count,
            "event dropped count should be monotonic",
        );
        *last_sequence = *sequence;
        *last_dropped_count = *dropped_count;
    }
}

/// Close one optional stream-event subscription.
fn close_event_handle(
    context: &mut AudioHarnessContext<'_>,
    events: &mut Option<AudioEventHandle>,
) -> RuntimeResult<()> {
    let Some(event_handle) = events.take() else {
        return Ok(());
    };

    context.destack_audio_event_close(event_handle)
}

/// Close one optional stream and device pair.
fn close_stream_device_pair(
    context: &mut AudioHarnessContext<'_>,
    device: &mut Option<AudioDeviceHandle>,
    stream: &mut Option<AudioStreamHandle>,
    is_running: &mut bool,
) -> RuntimeResult<()> {
    if let Some(stream_handle) = stream.take() {
        if *is_running {
            let _ = assert_ok_or_expected_error(
                context.destack_audio_stream_stop(stream_handle),
                &STREAM_STOP_ALLOWED_ERRORS,
            )?;
        }

        *is_running = false;
        context.destack_audio_stream_close(stream_handle)?;
    }

    if let Some(device_handle) = device.take() {
        context.destack_audio_device_close(device_handle)?;
    }

    Ok(())
}

/// Return one first available host backend, excluding auto and null backends.
fn first_available_host_backend(
    context: &mut AudioHarnessContext<'_>,
) -> RuntimeResult<Option<AudioBackend>> {
    let backend_list = context.destack_audio_backend_list()?;
    let backend_rows = backend_support_rows(context, backend_list)?;
    let backend = backend_rows.iter().find_map(|(backend, _support)| {
        // specimen tests only run on concrete host backends that are usable here
        if *backend == AudioBackend::Auto || *backend == AudioBackend::Null {
            return None;
        }

        if !backend_is_available_for_host_execution(&backend_rows, *backend) {
            return None;
        }

        Some(*backend)
    });

    Ok(backend)
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_stream_open_rejects_zero_channels() {
    with_harness_context(|mut context| {
        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Playback,
            backend: AudioBackend::Null,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };
        let device_id = harness_string(&mut context, "audio:null:playback");
        let device_options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, device_options)?;

        let invalid_config = AudioStreamConfig {
            sample_rate: 48_000,
            channels: 0,
            channel_layout: AudioChannelLayout::Stereo,
            channel_mask: 0b11,
            format: AudioSampleFormat::F32,
            period_frames: 128,
            transfer_mode: AudioStreamTransferMode::Push,
        };
        let invalid_config = harness_stream_config(&mut context, invalid_config);
        assert_platform_error_code(
            stream_open_with_default_options(&mut context, device, invalid_config),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        context.destack_audio_device_close(device)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_stream_open_rejects_unknown_stream_flags() {
    with_harness_context(|mut context| {
        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Playback,
            backend: AudioBackend::Null,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };
        let device_id = harness_string(&mut context, "audio:null:playback");
        let device_options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, device_options)?;

        let invalid_config = AudioStreamConfig {
            sample_rate: 48_000,
            channels: 2,
            channel_layout: AudioChannelLayout::Stereo,
            channel_mask: 0b11,
            format: AudioSampleFormat::F32,
            period_frames: 128,
            transfer_mode: AudioStreamTransferMode::Push,
        };
        let invalid_config = harness_stream_config(&mut context, invalid_config);
        let invalid_options = harness_stream_options(
            &mut context,
            core::default_stream_open_options_with_flags(AudioStreamFlags(0x8000_0000)),
        );
        assert_platform_error_code(
            context.destack_audio_stream_open(device, invalid_config, invalid_options),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        context.destack_audio_device_close(device)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_stream_open_accepts_known_stream_flags() {
    with_harness_context(|mut context| {
        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Playback,
            backend: AudioBackend::Null,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };
        let device_id = harness_string(&mut context, "audio:null:playback");
        let device_options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, device_options)?;

        let config = AudioStreamConfig {
            sample_rate: 48_000,
            channels: 2,
            channel_layout: AudioChannelLayout::Stereo,
            channel_mask: 0b11,
            format: AudioSampleFormat::F32,
            period_frames: 128,
            transfer_mode: AudioStreamTransferMode::Push,
        };
        let config = harness_stream_config(&mut context, config);
        let stream_options = harness_stream_options(
            &mut context,
            core::default_stream_open_options_with_flags(AudioStreamFlags(
                STREAM_FLAG_MINIMIZE_LATENCY.0,
            )),
        );
        let stream = context.destack_audio_stream_open(device, config, stream_options)?;
        context.destack_audio_stream_close(stream)?;

        context.destack_audio_device_close(device)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_stream_open_rejects_unsatisfied_requirements() {
    with_harness_context(|mut context| {
        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Capture,
            backend: AudioBackend::Null,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };
        let device_id = harness_string(&mut context, "audio:null:capture");
        let device_options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, device_options)?;

        let config = AudioStreamConfig {
            sample_rate: 48_000,
            channels: 2,
            channel_layout: AudioChannelLayout::Stereo,
            channel_mask: 0b11,
            format: AudioSampleFormat::F32,
            period_frames: 128,
            transfer_mode: AudioStreamTransferMode::Push,
        };
        let config = harness_stream_config(&mut context, config);
        let stream_options = harness_stream_options(
            &mut context,
            AudioStreamOpenOptions {
                flags: AudioStreamFlags(0),
                requirements: AudioStreamRequirementFlags(STREAM_REQUIRE_SCHEDULED_WRITE.0),
            },
        );
        assert_not_supported_result(context.destack_audio_stream_open(
            device,
            config,
            stream_options,
        ))?;

        context.destack_audio_device_close(device)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_stream_support_reports_unsatisfied_requirements() {
    with_harness_context(|mut context| {
        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Capture,
            backend: AudioBackend::Null,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };
        let device_id = harness_string(&mut context, "audio:null:capture");
        let device_options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, device_options)?;

        let config = AudioStreamConfig {
            sample_rate: 48_000,
            channels: 2,
            channel_layout: AudioChannelLayout::Stereo,
            channel_mask: 0b11,
            format: AudioSampleFormat::F32,
            period_frames: 128,
            transfer_mode: AudioStreamTransferMode::Push,
        };
        let config = harness_stream_config(&mut context, config);
        let stream_options = harness_stream_options(
            &mut context,
            AudioStreamOpenOptions {
                flags: AudioStreamFlags(0),
                requirements: AudioStreamRequirementFlags(STREAM_REQUIRE_SCHEDULED_WRITE.0),
            },
        );
        let support = context.destack_audio_stream_support(device, config, stream_options)?;
        let (supported, _satisfied_requirements, unsatisfied_requirements) =
            stream_support_from_value(support);
        assert!(!supported);
        assert_eq!(
            unsatisfied_requirements.0 & STREAM_REQUIRE_SCHEDULED_WRITE.0,
            STREAM_REQUIRE_SCHEDULED_WRITE.0,
        );

        context.destack_audio_device_close(device)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_stream_open_non_interleaved_matches_backend_capability() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_list = backend_support_rows_with_capabilities(&mut context, backend_list)?;

        for (backend, support, capability_flags) in backend_list {
            if support != BackendSupport::Available
                || backend == AudioBackend::Auto
                || backend == AudioBackend::Null
            {
                continue;
            }

            let default_id = match context.destack_audio_device_default(
                AudioDeviceDirection::Playback,
                backend,
                AudioBackendSelectionPolicy::Strict,
            ) {
                Ok(value) => string_from_harness_value(&mut context, value)?,
                Err(error) => {
                    let code = error_code_from_runtime_error(&error);
                    if code == Some(PlatformErrorCode::IoNotFound) {
                        continue;
                    }

                    return Err(error);
                }
            };

            let share_mode = if backend == AudioBackend::Asio {
                AudioShareMode::Exclusive
            } else {
                AudioShareMode::Shared
            };
            let options = AudioDeviceOpenOptions {
                direction: AudioDeviceDirection::Playback,
                backend,
                backend_policy: AudioBackendSelectionPolicy::Strict,
                share_mode,
                flags: AudioDeviceOpenFlags(0),
            };

            let device_id = harness_string(&mut context, &default_id);
            let options = harness_device_options(&mut context, options);
            let device = match context.destack_audio_device_open(device_id, options) {
                Ok(device) => device,
                Err(error) => {
                    let code = error_code_from_runtime_error(&error);
                    if code == Some(PlatformErrorCode::IoNotFound) {
                        continue;
                    }

                    return Err(error);
                }
            };

            let stream_config = AudioStreamConfig {
                sample_rate: 48_000,
                channels: 2,
                channel_layout: AudioChannelLayout::Stereo,
                channel_mask: 0b11,
                format: AudioSampleFormat::F32,
                period_frames: 128,
                transfer_mode: AudioStreamTransferMode::Push,
            };
            let stream_config = harness_stream_config(&mut context, stream_config);
            let stream_options = harness_stream_options(
                &mut context,
                core::default_stream_open_options_with_flags(AudioStreamFlags(
                    STREAM_FLAG_NON_INTERLEAVED.0,
                )),
            );
            let result = context.destack_audio_stream_open(device, stream_config, stream_options);
            let supports_non_interleaved =
                (capability_flags.0 & BACKEND_CAPABILITY_NON_INTERLEAVED.0) != 0;
            if !supports_non_interleaved {
                assert_not_supported_result(result)?;
                context.destack_audio_device_close(device)?;
                continue;
            }

            match result {
                Ok(stream) => {
                    context.destack_audio_stream_close(stream)?;
                }
                Err(error) => {
                    let code = error_code_from_runtime_error(&error);
                    assert_code_is_not_not_supported(
                        code,
                        &format!(
                            "backend {backend:?} advertises non-interleaved but stream.open returned notSupported"
                        ),
                    )?;
                }
            }

            context.destack_audio_device_close(device)?;
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_stream_open_uses_opened_device_direction() {
    with_harness_context(|mut context| {
        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Capture,
            backend: AudioBackend::Null,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };
        let device_id = harness_string(&mut context, "audio:null:duplex");
        let device_options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, device_options)?;

        let stream_config = AudioStreamConfig {
            sample_rate: 48_000,
            channels: 2,
            channel_layout: AudioChannelLayout::Stereo,
            channel_mask: 0b11,
            format: AudioSampleFormat::F32,
            period_frames: 128,
            transfer_mode: AudioStreamTransferMode::Push,
        };
        let stream_config = harness_stream_config(&mut context, stream_config);
        let stream = stream_open_with_default_options(&mut context, device, stream_config)?;
        context.destack_audio_stream_start(stream)?;

        let payload = harness_bytes(&mut context, &[0u8; 32])?;
        assert_not_supported_result(context.destack_audio_stream_try_write(stream, payload))?;

        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_stream_randomized_interleaving_on_null_duplex() {
    with_harness_context(|mut context| {
        let mut random = DeterministicSequence::new(0x4C90_7811_22A5_B0D1);
        let mut device_handle: Option<AudioDeviceHandle> = None;
        let mut stream_handle: Option<AudioStreamHandle> = None;
        let mut event_handle: Option<AudioEventHandle> = None;
        let mut is_running = false;
        let mut last_event_sequence = 0u64;
        let mut last_event_dropped_count = 0u64;

        for _ in 0..RANDOM_NULL_INTERLEAVING_ITERATIONS {
            match StreamInterleavingTask::draw(&mut random) {
                StreamInterleavingTask::Open => {
                    if stream_handle.is_some() {
                        continue;
                    }

                    let (opened_device, opened_stream) = open_null_duplex_stream(&mut context)?;
                    device_handle = Some(opened_device);
                    stream_handle = Some(opened_stream);
                    is_running = true;
                }
                StreamInterleavingTask::Close => {
                    close_event_handle(&mut context, &mut event_handle)?;
                    close_stream_device_pair(
                        &mut context,
                        &mut device_handle,
                        &mut stream_handle,
                        &mut is_running,
                    )?;
                }
                StreamInterleavingTask::ToggleRunning => {
                    let Some(stream_handle_value) = stream_handle else {
                        continue;
                    };

                    let stream_result = if is_running {
                        context.destack_audio_stream_stop(stream_handle_value)
                    } else {
                        context.destack_audio_stream_start(stream_handle_value)
                    };
                    let _ = assert_ok_or_expected_error(
                        stream_result,
                        &[PlatformErrorCode::IoWouldBlock],
                    )?;
                    let stream_state = context.destack_audio_stream_state(stream_handle_value)?;
                    let stream_state = stream_state_from_value(stream_state);
                    is_running = stream_state.running;
                }
                StreamInterleavingTask::TryWrite => {
                    let Some(stream_handle_value) = stream_handle else {
                        continue;
                    };

                    let payload = random_payload(&mut random, RANDOM_STREAM_IO_BYTES);
                    let payload = harness_bytes(&mut context, &payload)?;
                    let _ = assert_ok_or_expected_error(
                        context.destack_audio_stream_try_write(stream_handle_value, payload),
                        &[PlatformErrorCode::IoWouldBlock],
                    )?;
                }
                StreamInterleavingTask::TryRead => {
                    let Some(stream_handle_value) = stream_handle else {
                        continue;
                    };

                    let read = assert_ok_or_expected_error(
                        context.destack_audio_stream_try_read(
                            stream_handle_value,
                            RANDOM_STREAM_IO_BYTES as u32,
                        ),
                        &[PlatformErrorCode::IoWouldBlock],
                    )?;
                    if let Some(read) = read {
                        assert!(byte_len(&mut context, read)? <= RANDOM_STREAM_IO_BYTES);
                    }
                }
                StreamInterleavingTask::ToggleEventSubscription => {
                    let Some(stream_handle_value) = stream_handle else {
                        continue;
                    };

                    if event_handle.is_some() {
                        close_event_handle(&mut context, &mut event_handle)?;
                        continue;
                    }

                    let event_options =
                        stream_event_options(AudioBackend::Null, stream_handle_value);
                    let event_options = harness_event_options(&mut context, event_options);
                    let opened_event = context.destack_audio_event_open(event_options)?;
                    event_handle = Some(opened_event);
                    last_event_sequence = 0;
                    last_event_dropped_count = 0;
                }
                StreamInterleavingTask::TryReadEventBatch => {
                    let Some(event_handle_value) = event_handle else {
                        continue;
                    };

                    let batch = assert_ok_or_expected_error(
                        context.destack_audio_event_try_read_batch(event_handle_value, 4),
                        &[PlatformErrorCode::IoWouldBlock],
                    )?;
                    let Some(batch) = batch else {
                        continue;
                    };
                    let rows = event_batch_sequence_rows(&mut context, batch)?;
                    assert_event_rows_monotonic(
                        &rows,
                        &mut last_event_sequence,
                        &mut last_event_dropped_count,
                    );
                }
                StreamInterleavingTask::ReadState => {
                    let Some(stream_handle_value) = stream_handle else {
                        continue;
                    };

                    let stream_state = context.destack_audio_stream_state(stream_handle_value)?;
                    let stream_state = stream_state_from_value(stream_state);
                    is_running = stream_state.running;
                }
            }
        }

        close_event_handle(&mut context, &mut event_handle)?;
        close_stream_device_pair(
            &mut context,
            &mut device_handle,
            &mut stream_handle,
            &mut is_running,
        )?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_stream_randomized_interleaving_on_available_host_backend() {
    with_harness_context(|mut context| {
        let Some(backend) = first_available_host_backend(&mut context)? else {
            return Ok(());
        };

        let device_id = match context.destack_audio_device_default(
            AudioDeviceDirection::Playback,
            backend,
            AudioBackendSelectionPolicy::Strict,
        ) {
            Ok(value) => string_from_harness_value(&mut context, value)?,
            Err(error) => {
                let code = error_code_from_runtime_error(&error);
                if code == Some(PlatformErrorCode::IoNotFound) {
                    return Ok(());
                }

                return Err(error);
            }
        };
        let mut random = DeterministicSequence::new(0x7611_A9CE_8815_E023);
        let mut device_handle: Option<AudioDeviceHandle> = None;
        let mut stream_handle: Option<AudioStreamHandle> = None;
        let mut event_handle: Option<AudioEventHandle> = None;
        let mut is_running = false;
        let mut last_event_sequence = 0u64;
        let mut last_event_dropped_count = 0u64;

        for _ in 0..RANDOM_HOST_INTERLEAVING_ITERATIONS {
            match StreamInterleavingTask::draw(&mut random) {
                StreamInterleavingTask::Open => {
                    if stream_handle.is_some() {
                        continue;
                    }

                    let opened_device = if let Some(opened_device) = device_handle {
                        opened_device
                    } else {
                        let options = AudioDeviceOpenOptions {
                            direction: AudioDeviceDirection::Playback,
                            backend,
                            backend_policy: AudioBackendSelectionPolicy::Strict,
                            share_mode: AudioShareMode::Shared,
                            flags: AudioDeviceOpenFlags(0),
                        };
                        let device_id = harness_string(&mut context, &device_id);
                        let options = harness_device_options(&mut context, options);
                        let opened_device =
                            match context.destack_audio_device_open(device_id, options) {
                                Ok(device) => device,
                                Err(error) => {
                                    let code = error_code_from_runtime_error(&error);
                                    if code == Some(PlatformErrorCode::IoNotFound)
                                        || code == Some(PlatformErrorCode::IoPermissionDenied)
                                        || code == Some(PlatformErrorCode::AudioUnavailable)
                                        || code == Some(PlatformErrorCode::DeviceUnavailable)
                                    {
                                        return Ok(());
                                    }

                                    return Err(error);
                                }
                            };
                        device_handle = Some(opened_device);
                        opened_device
                    };

                    let stream_config = AudioStreamConfig {
                        sample_rate: 48_000,
                        channels: 2,
                        channel_layout: AudioChannelLayout::Stereo,
                        channel_mask: 0b11,
                        format: AudioSampleFormat::F32,
                        period_frames: 128,
                        transfer_mode: AudioStreamTransferMode::Push,
                    };
                    let stream_config = harness_stream_config(&mut context, stream_config);
                    let stream_options =
                        harness_stream_options(&mut context, core::default_stream_open_options());
                    let opened_stream = match context.destack_audio_stream_open(
                        opened_device,
                        stream_config,
                        stream_options,
                    ) {
                        Ok(stream) => stream,
                        Err(error) => {
                            let code = error_code_from_runtime_error(&error);
                            if is_not_supported_code(code)
                                || code == Some(PlatformErrorCode::IoInvalidData)
                            {
                                return Ok(());
                            }

                            return Err(error);
                        }
                    };
                    stream_handle = Some(opened_stream);
                    let _ = assert_ok_or_expected_error(
                        context.destack_audio_stream_start(opened_stream),
                        &[
                            PlatformErrorCode::IoWouldBlock,
                            PlatformErrorCode::IoInterrupted,
                            PlatformErrorCode::IoBusy,
                        ],
                    )?;
                    let stream_state = context.destack_audio_stream_state(opened_stream)?;
                    let stream_state = stream_state_from_value(stream_state);
                    is_running = stream_state.running;
                }
                StreamInterleavingTask::Close => {
                    close_event_handle(&mut context, &mut event_handle)?;
                    close_stream_device_pair(
                        &mut context,
                        &mut device_handle,
                        &mut stream_handle,
                        &mut is_running,
                    )?;
                }
                StreamInterleavingTask::ToggleRunning => {
                    let Some(stream_handle_value) = stream_handle else {
                        continue;
                    };

                    let stream_result = if is_running {
                        context.destack_audio_stream_stop(stream_handle_value)
                    } else {
                        context.destack_audio_stream_start(stream_handle_value)
                    };
                    let _ = assert_ok_or_expected_error(
                        stream_result,
                        &[
                            PlatformErrorCode::IoWouldBlock,
                            PlatformErrorCode::IoInterrupted,
                            PlatformErrorCode::IoBusy,
                        ],
                    )?;
                    let stream_state = context.destack_audio_stream_state(stream_handle_value)?;
                    let stream_state = stream_state_from_value(stream_state);
                    is_running = stream_state.running;
                }
                StreamInterleavingTask::TryWrite => {
                    let Some(stream_handle_value) = stream_handle else {
                        continue;
                    };

                    let payload = random_payload(&mut random, RANDOM_STREAM_IO_BYTES);
                    let payload = harness_bytes(&mut context, &payload)?;
                    let _ = assert_ok_or_expected_error(
                        context.destack_audio_stream_try_write(stream_handle_value, payload),
                        &STREAM_IO_TRANSIENT_ALLOWED_ERRORS,
                    )?;
                }
                StreamInterleavingTask::TryRead => {
                    let Some(stream_handle_value) = stream_handle else {
                        continue;
                    };

                    let read = assert_ok_or_expected_error(
                        context.destack_audio_stream_try_read(
                            stream_handle_value,
                            RANDOM_STREAM_IO_BYTES as u32,
                        ),
                        &STREAM_IO_TRANSIENT_ALLOWED_ERRORS,
                    )?;
                    if let Some(read) = read {
                        assert!(byte_len(&mut context, read)? <= RANDOM_STREAM_IO_BYTES);
                    }
                }
                StreamInterleavingTask::ToggleEventSubscription => {
                    let Some(stream_handle_value) = stream_handle else {
                        continue;
                    };

                    if event_handle.is_some() {
                        close_event_handle(&mut context, &mut event_handle)?;
                        continue;
                    }

                    let event_options = stream_event_options(backend, stream_handle_value);
                    let event_options = harness_event_options(&mut context, event_options);
                    let opened_event = match context.destack_audio_event_open(event_options) {
                        Ok(event) => event,
                        Err(error) => {
                            let code = error_code_from_runtime_error(&error);
                            if is_not_supported_code(code) {
                                continue;
                            }

                            return Err(error);
                        }
                    };
                    event_handle = Some(opened_event);
                    last_event_sequence = 0;
                    last_event_dropped_count = 0;
                }
                StreamInterleavingTask::TryReadEventBatch => {
                    let Some(event_handle_value) = event_handle else {
                        continue;
                    };

                    let batch = assert_ok_or_expected_error(
                        context.destack_audio_event_try_read_batch(event_handle_value, 4),
                        &[
                            PlatformErrorCode::IoWouldBlock,
                            PlatformErrorCode::IoInterrupted,
                            PlatformErrorCode::IoBusy,
                        ],
                    )?;
                    let Some(batch) = batch else {
                        continue;
                    };
                    let rows = event_batch_sequence_rows(&mut context, batch)?;
                    assert_event_rows_monotonic(
                        &rows,
                        &mut last_event_sequence,
                        &mut last_event_dropped_count,
                    );
                }
                StreamInterleavingTask::ReadState => {
                    let Some(stream_handle_value) = stream_handle else {
                        continue;
                    };

                    let stream_state = assert_ok_or_expected_error(
                        context.destack_audio_stream_state(stream_handle_value),
                        &[
                            PlatformErrorCode::IoInvalidData,
                            PlatformErrorCode::AudioUnavailable,
                            PlatformErrorCode::DeviceUnavailable,
                        ],
                    )?;
                    if let Some(stream_state) = stream_state {
                        let stream_state = stream_state_from_value(stream_state);
                        is_running = stream_state.running;
                    }
                }
            }
        }

        close_event_handle(&mut context, &mut event_handle)?;
        close_stream_device_pair(
            &mut context,
            &mut device_handle,
            &mut stream_handle,
            &mut is_running,
        )?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_host_stream_open_close_when_backend_is_available() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_list = backend_support_rows(&mut context, backend_list)?;
        let backend = backend_list.iter().find_map(|(backend, _support)| {
            // specimen tests only run on concrete host backends that are usable here
            if *backend == AudioBackend::Auto || *backend == AudioBackend::Null {
                return None;
            }

            if !backend_is_available_for_host_execution(&backend_list, *backend) {
                return None;
            }

            Some(*backend)
        });
        let Some(backend) = backend else {
            return Ok(());
        };

        let device_id = match context.destack_audio_device_default(
            AudioDeviceDirection::Playback,
            backend,
            AudioBackendSelectionPolicy::Strict,
        ) {
            Ok(value) => string_from_harness_value(&mut context, value)?,
            Err(error) => {
                let code = error_code_from_runtime_error(&error);
                if code == Some(PlatformErrorCode::IoNotFound) {
                    return Ok(());
                }
                return Err(error);
            }
        };

        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Playback,
            backend,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };
        let device_id = harness_string(&mut context, &device_id);
        let options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, options)?;

        let config = AudioStreamConfig {
            sample_rate: 48_000,
            channels: 1,
            channel_layout: AudioChannelLayout::Mono,
            channel_mask: 0b1,
            format: AudioSampleFormat::F32,
            period_frames: 128,
            transfer_mode: AudioStreamTransferMode::Push,
        };
        let config = harness_stream_config(&mut context, config);
        let stream = stream_open_with_default_options(&mut context, device, config)?;
        context.destack_audio_stream_start(stream)?;

        let payload = (0..256usize)
            .flat_map(|index| {
                let sample = (index as f32 / 256.0) * 0.1;
                sample.to_le_bytes()
            })
            .collect::<Vec<_>>();
        let payload = harness_bytes(&mut context, &payload)?;
        let written = context.destack_audio_stream_write(stream, payload)?;
        assert!(written > 0, "host stream write should accept payload");

        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_host_stream_write_at_rejects_when_backend_lacks_schedule_lane() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_list = backend_support_rows(&mut context, backend_list)?;
        let backend = backend_list.iter().find_map(|(backend, _support)| {
            // specimen tests only run on concrete host backends that are usable here
            if *backend == AudioBackend::Auto || *backend == AudioBackend::Null {
                return None;
            }

            if !backend_is_available_for_host_execution(&backend_list, *backend) {
                return None;
            }

            Some(*backend)
        });
        let Some(backend) = backend else {
            return Ok(());
        };

        let device_id = match context.destack_audio_device_default(
            AudioDeviceDirection::Playback,
            backend,
            AudioBackendSelectionPolicy::Strict,
        ) {
            Ok(value) => string_from_harness_value(&mut context, value)?,
            Err(error) => {
                let code = error_code_from_runtime_error(&error);
                if code == Some(PlatformErrorCode::IoNotFound) {
                    return Ok(());
                }
                return Err(error);
            }
        };

        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Playback,
            backend,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };
        let device_id = harness_string(&mut context, &device_id);
        let options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, options)?;

        let config = AudioStreamConfig {
            sample_rate: 48_000,
            channels: 2,
            channel_layout: AudioChannelLayout::Stereo,
            channel_mask: 0b11,
            format: AudioSampleFormat::F32,
            period_frames: 128,
            transfer_mode: AudioStreamTransferMode::Push,
        };
        let config = harness_stream_config(&mut context, config);
        let Some(stream) = assert_ok_or_expected_error(
            stream_open_with_default_options(&mut context, device, config),
            &STREAM_OPEN_OPTIONAL_ERRORS,
        )?
        else {
            context.destack_audio_device_close(device)?;
            return Ok(());
        };

        let Some(()) = assert_ok_or_expected_error(
            context.destack_audio_stream_start(stream),
            &STREAM_START_OPTIONAL_ERRORS,
        )?
        else {
            context.destack_audio_stream_close(stream)?;
            context.destack_audio_device_close(device)?;
            return Ok(());
        };

        let snapshot = context.destack_audio_stream_descriptor(stream)?;
        let (
            supports_write_at,
            _supports_pause,
            _supports_volume,
            _supports_mute,
            _supports_hardware_timestamps,
        ) = stream_descriptor_flags_from_value(snapshot);
        let payload = harness_bytes(&mut context, &[0u8; 32])?;
        let write_at_result = context.destack_audio_stream_write_at(stream, payload, 0);
        if supports_write_at {
            let _ =
                assert_ok_or_expected_error(write_at_result, &[PlatformErrorCode::IoWouldBlock])?;
        } else {
            assert_not_supported_result(write_at_result)?;
        }

        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_stream_descriptor_flags_match_control_behavior_for_available_backends() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_rows = backend_support_rows_with_capabilities(&mut context, backend_list)?;

        for (backend, support, capability_flags) in backend_rows {
            if support != BackendSupport::Available
                || backend == AudioBackend::Auto
                || backend == AudioBackend::Null
            {
                continue;
            }

            let device_id = match context.destack_audio_device_default(
                AudioDeviceDirection::Playback,
                backend,
                AudioBackendSelectionPolicy::Strict,
            ) {
                Ok(value) => string_from_harness_value(&mut context, value)?,
                Err(error) => {
                    let code = error_code_from_runtime_error(&error);
                    if code == Some(PlatformErrorCode::IoNotFound) {
                        continue;
                    }

                    return Err(error);
                }
            };

            let share_mode = if (capability_flags.0 & BACKEND_CAPABILITY_SHARED_MODE.0) != 0 {
                AudioShareMode::Shared
            } else {
                AudioShareMode::Exclusive
            };
            let share_mode_supported = match share_mode {
                AudioShareMode::Shared => {
                    (capability_flags.0 & BACKEND_CAPABILITY_SHARED_MODE.0) != 0
                }
                AudioShareMode::Exclusive => {
                    (capability_flags.0 & BACKEND_CAPABILITY_EXCLUSIVE_MODE.0) != 0
                }
            };
            if !share_mode_supported {
                continue;
            }

            let options = AudioDeviceOpenOptions {
                direction: AudioDeviceDirection::Playback,
                backend,
                backend_policy: AudioBackendSelectionPolicy::Strict,
                share_mode,
                flags: AudioDeviceOpenFlags(0),
            };
            let device_id = harness_string(&mut context, &device_id);
            let options = harness_device_options(&mut context, options);
            let device = match context.destack_audio_device_open(device_id, options) {
                Ok(device) => device,
                Err(error) => {
                    let code = error_code_from_runtime_error(&error);
                    if is_not_supported_code(code) {
                        panic!(
                            "backend {backend:?} advertised share mode {share_mode:?} but device open returned notSupported",
                        );
                    }
                    continue;
                }
            };

            let config = AudioStreamConfig {
                sample_rate: 48_000,
                channels: 2,
                channel_layout: AudioChannelLayout::Stereo,
                channel_mask: 0b11,
                format: AudioSampleFormat::F32,
                period_frames: 128,
                transfer_mode: AudioStreamTransferMode::Push,
            };
            let config = harness_stream_config(&mut context, config);
            let stream = match stream_open_with_default_options(&mut context, device, config) {
                Ok(stream) => stream,
                Err(error) => {
                    context.destack_audio_device_close(device)?;
                    return Err(error);
                }
            };

            context.destack_audio_stream_start(stream)?;

            let snapshot = context.destack_audio_stream_descriptor(stream)?;
            let (
                supports_write_at,
                supports_pause,
                supports_volume,
                supports_mute,
                supports_hardware_timestamps,
            ) = stream_descriptor_flags_from_value(snapshot);

            let backend_supports_device_clock =
                (capability_flags.0 & BACKEND_CAPABILITY_DEVICE_CLOCK.0) != 0;
            assert_eq!(
                supports_hardware_timestamps, backend_supports_device_clock,
                "backend {backend:?} snapshot hardware-timestamp lane should match backend capability advertisement",
            );

            let pause_result = context.destack_audio_stream_pause(stream, true);
            if supports_pause {
                let _ =
                    assert_ok_or_expected_error(pause_result, &[PlatformErrorCode::IoWouldBlock])?;
                let _ = context.destack_audio_stream_pause(stream, false);
            } else {
                assert_not_supported_result(pause_result)?;
            }

            let volume_result = context.destack_audio_stream_set_volume(stream, 0.75);
            if supports_volume {
                if let Err(error) = volume_result {
                    let code = error_code_from_runtime_error(&error);
                    assert_code_is_not_not_supported(
                        code,
                        &format!("backend {backend:?} snapshot claimed supports_volume"),
                    )?;
                }
            } else {
                assert_not_supported_result(volume_result)?;
            }

            let mute_result = context.destack_audio_stream_set_mute(stream, true);
            if supports_mute {
                if let Err(error) = mute_result {
                    let code = error_code_from_runtime_error(&error);
                    assert_code_is_not_not_supported(
                        code,
                        &format!("backend {backend:?} snapshot claimed supports_mute"),
                    )?;
                }
            } else {
                assert_not_supported_result(mute_result)?;
            }

            let payload = harness_bytes(&mut context, &[0u8; 64])?;
            let write_at_result = context.destack_audio_stream_write_at(stream, payload, 0);
            if supports_write_at {
                if let Err(error) = write_at_result {
                    let code = error_code_from_runtime_error(&error);
                    assert_code_is_not_not_supported(
                        code,
                        &format!("backend {backend:?} snapshot claimed supports_write_at"),
                    )?;
                }
            } else {
                assert_not_supported_result(write_at_result)?;
            }

            context.destack_audio_stream_stop(stream)?;
            context.destack_audio_stream_close(stream)?;
            context.destack_audio_device_close(device)?;
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_coreaudio_loopback_open_start_stop_when_available() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_list = backend_support_rows(&mut context, backend_list)?;
        let coreaudio_available =
            backend_is_available_for_host_execution(&backend_list, AudioBackend::CoreAudio);
        if !coreaudio_available {
            return Ok(());
        }

        let device_id = match context.destack_audio_device_default(
            AudioDeviceDirection::Loopback,
            AudioBackend::CoreAudio,
            AudioBackendSelectionPolicy::Strict,
        ) {
            Ok(value) => string_from_harness_value(&mut context, value)?,
            Err(error) => {
                let code = error_code_from_runtime_error(&error);
                if code == Some(PlatformErrorCode::IoNotFound) {
                    return Ok(());
                }
                return Err(error);
            }
        };

        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Loopback,
            backend: AudioBackend::CoreAudio,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };
        let device_id = harness_string(&mut context, &device_id);
        let options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, options)?;

        let config = AudioStreamConfig {
            sample_rate: 48_000,
            channels: 2,
            channel_layout: AudioChannelLayout::Stereo,
            channel_mask: 0b11,
            format: AudioSampleFormat::F32,
            period_frames: 128,
            transfer_mode: AudioStreamTransferMode::Push,
        };
        let config = harness_stream_config(&mut context, config);
        let Some(stream) = assert_ok_or_expected_error(
            stream_open_with_default_options(&mut context, device, config),
            &STREAM_OPEN_OPTIONAL_ERRORS,
        )?
        else {
            context.destack_audio_device_close(device)?;
            return Ok(());
        };

        let Some(()) = assert_ok_or_expected_error(
            context.destack_audio_stream_start(stream),
            &STREAM_START_OPTIONAL_ERRORS,
        )?
        else {
            context.destack_audio_stream_close(stream)?;
            context.destack_audio_device_close(device)?;
            return Ok(());
        };

        let _ = assert_ok_or_expected_error(
            context.destack_audio_stream_try_read(stream, 256),
            &[PlatformErrorCode::IoWouldBlock],
        )?;

        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_asio_playback_open_start_stop_when_available() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_list = backend_support_rows(&mut context, backend_list)?;
        let asio_available =
            backend_is_available_for_host_execution(&backend_list, AudioBackend::Asio);
        if !asio_available {
            return Ok(());
        }

        let device_id = match context.destack_audio_device_default(
            AudioDeviceDirection::Playback,
            AudioBackend::Asio,
            AudioBackendSelectionPolicy::Strict,
        ) {
            Ok(value) => string_from_harness_value(&mut context, value)?,
            Err(error) => {
                let code = error_code_from_runtime_error(&error);
                if code == Some(PlatformErrorCode::IoNotFound) {
                    return Ok(());
                }
                return Err(error);
            }
        };

        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Playback,
            backend: AudioBackend::Asio,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Exclusive,
            flags: AudioDeviceOpenFlags(0),
        };
        let device_id = harness_string(&mut context, &device_id);
        let options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, options)?;

        let config = AudioStreamConfig {
            sample_rate: 48_000,
            channels: 2,
            channel_layout: AudioChannelLayout::Stereo,
            channel_mask: 0b11,
            format: AudioSampleFormat::F32,
            period_frames: 128,
            transfer_mode: AudioStreamTransferMode::Push,
        };
        let config = harness_stream_config(&mut context, config);
        let stream = stream_open_with_default_options(&mut context, device, config)?;
        context.destack_audio_stream_start(stream)?;

        let payload = harness_bytes(&mut context, &[0u8; 256])?;
        let _ = assert_ok_or_expected_error(
            context.destack_audio_stream_try_write(stream, payload),
            &[PlatformErrorCode::IoWouldBlock],
        )?;

        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_asio_open_rejects_shared_mode_when_available() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_list = backend_support_rows(&mut context, backend_list)?;
        let asio_available =
            backend_is_available_for_host_execution(&backend_list, AudioBackend::Asio);
        if !asio_available {
            return Ok(());
        }

        let device_id = match context.destack_audio_device_default(
            AudioDeviceDirection::Playback,
            AudioBackend::Asio,
            AudioBackendSelectionPolicy::Strict,
        ) {
            Ok(value) => string_from_harness_value(&mut context, value)?,
            Err(error) => {
                let code = error_code_from_runtime_error(&error);
                if code == Some(PlatformErrorCode::IoNotFound) {
                    return Ok(());
                }
                return Err(error);
            }
        };

        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Playback,
            backend: AudioBackend::Asio,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };
        let device_id = harness_string(&mut context, &device_id);
        let options = harness_device_options(&mut context, options);
        assert_not_supported_result(context.destack_audio_device_open(device_id, options))?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_alsa_playback_open_start_stop_when_available() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_list = backend_support_rows(&mut context, backend_list)?;
        let alsa_available =
            backend_is_available_for_host_execution(&backend_list, AudioBackend::Alsa);
        if !alsa_available {
            return Ok(());
        }

        let device_id = match context.destack_audio_device_default(
            AudioDeviceDirection::Playback,
            AudioBackend::Alsa,
            AudioBackendSelectionPolicy::Strict,
        ) {
            Ok(value) => string_from_harness_value(&mut context, value)?,
            Err(error) => {
                let code = error_code_from_runtime_error(&error);
                if code == Some(PlatformErrorCode::IoNotFound) {
                    return Ok(());
                }
                return Err(error);
            }
        };

        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Playback,
            backend: AudioBackend::Alsa,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };
        let device_id = harness_string(&mut context, &device_id);
        let options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, options)?;

        let config = AudioStreamConfig {
            sample_rate: 48_000,
            channels: 2,
            channel_layout: AudioChannelLayout::Stereo,
            channel_mask: 0b11,
            format: AudioSampleFormat::F32,
            period_frames: 128,
            transfer_mode: AudioStreamTransferMode::Push,
        };
        let config = harness_stream_config(&mut context, config);
        let stream = stream_open_with_default_options(&mut context, device, config)?;
        context.destack_audio_stream_start(stream)?;

        let payload = harness_bytes(&mut context, &[0u8; 256])?;
        let _ = assert_ok_or_expected_error(
            context.destack_audio_stream_try_write(stream, payload),
            &[PlatformErrorCode::IoWouldBlock],
        )?;

        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_pipewire_playback_open_start_stop_when_available() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_list = backend_support_rows(&mut context, backend_list)?;
        let pipewire_available =
            backend_is_available_for_host_execution(&backend_list, AudioBackend::PipeWire);
        if !pipewire_available {
            return Ok(());
        }

        let device_id = match context.destack_audio_device_default(
            AudioDeviceDirection::Playback,
            AudioBackend::PipeWire,
            AudioBackendSelectionPolicy::Strict,
        ) {
            Ok(value) => string_from_harness_value(&mut context, value)?,
            Err(error) => {
                let code = error_code_from_runtime_error(&error);
                if code == Some(PlatformErrorCode::IoNotFound) {
                    return Ok(());
                }
                return Err(error);
            }
        };

        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Playback,
            backend: AudioBackend::PipeWire,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };
        let device_id = harness_string(&mut context, &device_id);
        let options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, options)?;

        let config = AudioStreamConfig {
            sample_rate: 48_000,
            channels: 2,
            channel_layout: AudioChannelLayout::Stereo,
            channel_mask: 0b11,
            format: AudioSampleFormat::F32,
            period_frames: 128,
            transfer_mode: AudioStreamTransferMode::Push,
        };
        let config = harness_stream_config(&mut context, config);
        let stream = stream_open_with_default_options(&mut context, device, config)?;
        context.destack_audio_stream_start(stream)?;

        let payload = harness_bytes(&mut context, &[0u8; 256])?;
        let _ = assert_ok_or_expected_error(
            context.destack_audio_stream_try_write(stream, payload),
            &[PlatformErrorCode::IoWouldBlock],
        )?;

        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_pipewire_loopback_open_start_stop_when_available() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_list = backend_support_rows(&mut context, backend_list)?;
        let pipewire_available =
            backend_is_available_for_host_execution(&backend_list, AudioBackend::PipeWire);
        if !pipewire_available {
            return Ok(());
        }

        let device_id = match context.destack_audio_device_default(
            AudioDeviceDirection::Loopback,
            AudioBackend::PipeWire,
            AudioBackendSelectionPolicy::Strict,
        ) {
            Ok(value) => string_from_harness_value(&mut context, value)?,
            Err(error) => {
                let code = error_code_from_runtime_error(&error);
                if code == Some(PlatformErrorCode::IoNotFound) {
                    return Ok(());
                }
                return Err(error);
            }
        };

        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Loopback,
            backend: AudioBackend::PipeWire,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };
        let device_id = harness_string(&mut context, &device_id);
        let options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, options)?;

        let config = AudioStreamConfig {
            sample_rate: 48_000,
            channels: 2,
            channel_layout: AudioChannelLayout::Stereo,
            channel_mask: 0b11,
            format: AudioSampleFormat::F32,
            period_frames: 128,
            transfer_mode: AudioStreamTransferMode::Push,
        };
        let config = harness_stream_config(&mut context, config);
        let stream = stream_open_with_default_options(&mut context, device, config)?;
        context.destack_audio_stream_start(stream)?;

        let _ = assert_ok_or_expected_error(
            context.destack_audio_stream_try_read(stream, 256),
            &[PlatformErrorCode::IoWouldBlock],
        )?;

        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_pulseaudio_playback_open_start_stop_when_available() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_list = backend_support_rows(&mut context, backend_list)?;
        let pulseaudio_available =
            backend_is_available_for_host_execution(&backend_list, AudioBackend::PulseAudio);
        if !pulseaudio_available {
            return Ok(());
        }

        let device_id = match context.destack_audio_device_default(
            AudioDeviceDirection::Playback,
            AudioBackend::PulseAudio,
            AudioBackendSelectionPolicy::Strict,
        ) {
            Ok(value) => string_from_harness_value(&mut context, value)?,
            Err(error) => {
                let code = error_code_from_runtime_error(&error);
                if code == Some(PlatformErrorCode::IoNotFound) {
                    return Ok(());
                }
                return Err(error);
            }
        };

        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Playback,
            backend: AudioBackend::PulseAudio,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };
        let device_id = harness_string(&mut context, &device_id);
        let options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, options)?;

        let config = AudioStreamConfig {
            sample_rate: 48_000,
            channels: 2,
            channel_layout: AudioChannelLayout::Stereo,
            channel_mask: 0b11,
            format: AudioSampleFormat::F32,
            period_frames: 128,
            transfer_mode: AudioStreamTransferMode::Push,
        };
        let config = harness_stream_config(&mut context, config);
        let stream = stream_open_with_default_options(&mut context, device, config)?;
        context.destack_audio_stream_start(stream)?;

        let payload = harness_bytes(&mut context, &[0u8; 256])?;
        let _ = assert_ok_or_expected_error(
            context.destack_audio_stream_try_write(stream, payload),
            &[PlatformErrorCode::IoWouldBlock],
        )?;

        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_pulseaudio_loopback_open_start_stop_when_available() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_list = backend_support_rows(&mut context, backend_list)?;
        let pulseaudio_available =
            backend_is_available_for_host_execution(&backend_list, AudioBackend::PulseAudio);
        if !pulseaudio_available {
            return Ok(());
        }

        let device_id = match context.destack_audio_device_default(
            AudioDeviceDirection::Loopback,
            AudioBackend::PulseAudio,
            AudioBackendSelectionPolicy::Strict,
        ) {
            Ok(value) => string_from_harness_value(&mut context, value)?,
            Err(error) => {
                let code = error_code_from_runtime_error(&error);
                if code == Some(PlatformErrorCode::IoNotFound) {
                    return Ok(());
                }
                return Err(error);
            }
        };

        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Loopback,
            backend: AudioBackend::PulseAudio,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };
        let device_id = harness_string(&mut context, &device_id);
        let options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, options)?;

        let config = AudioStreamConfig {
            sample_rate: 48_000,
            channels: 2,
            channel_layout: AudioChannelLayout::Stereo,
            channel_mask: 0b11,
            format: AudioSampleFormat::F32,
            period_frames: 128,
            transfer_mode: AudioStreamTransferMode::Push,
        };
        let config = harness_stream_config(&mut context, config);
        let stream = stream_open_with_default_options(&mut context, device, config)?;
        context.destack_audio_stream_start(stream)?;

        let _ = assert_ok_or_expected_error(
            context.destack_audio_stream_try_read(stream, 256),
            &[PlatformErrorCode::IoWouldBlock],
        )?;

        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_jack_playback_open_start_stop_when_available() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_list = backend_support_rows(&mut context, backend_list)?;
        let jack_available =
            backend_is_available_for_host_execution(&backend_list, AudioBackend::Jack);
        if !jack_available {
            return Ok(());
        }

        let device_id = match context.destack_audio_device_default(
            AudioDeviceDirection::Playback,
            AudioBackend::Jack,
            AudioBackendSelectionPolicy::Strict,
        ) {
            Ok(value) => string_from_harness_value(&mut context, value)?,
            Err(error) => {
                let code = error_code_from_runtime_error(&error);
                if code == Some(PlatformErrorCode::IoNotFound) {
                    return Ok(());
                }
                return Err(error);
            }
        };

        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Playback,
            backend: AudioBackend::Jack,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };
        let device_id = harness_string(&mut context, &device_id);
        let options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, options)?;

        let config = AudioStreamConfig {
            sample_rate: 48_000,
            channels: 2,
            channel_layout: AudioChannelLayout::Stereo,
            channel_mask: 0b11,
            format: AudioSampleFormat::F32,
            period_frames: 128,
            transfer_mode: AudioStreamTransferMode::Push,
        };
        let config = harness_stream_config(&mut context, config);
        let stream = stream_open_with_default_options(&mut context, device, config)?;
        context.destack_audio_stream_start(stream)?;

        let payload = harness_bytes(&mut context, &[0u8; 256])?;
        let _ = assert_ok_or_expected_error(
            context.destack_audio_stream_try_write(stream, payload),
            &[PlatformErrorCode::IoWouldBlock],
        )?;

        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(target_os = "android")]
#[test]
fn test_audio_aaudio_playback_open_start_stop_when_available() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_list = backend_support_rows(&mut context, backend_list)?;
        let aaudio_available =
            backend_is_available_for_host_execution(&backend_list, AudioBackend::AAudio);
        if !aaudio_available {
            return Ok(());
        }

        let device_id = match context.destack_audio_device_default(
            AudioDeviceDirection::Playback,
            AudioBackend::AAudio,
            AudioBackendSelectionPolicy::Strict,
        ) {
            Ok(value) => string_from_harness_value(&mut context, value)?,
            Err(error) => {
                let code = error_code_from_runtime_error(&error);
                if code == Some(PlatformErrorCode::IoNotFound) {
                    return Ok(());
                }
                return Err(error);
            }
        };

        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Playback,
            backend: AudioBackend::AAudio,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };
        let device_id = harness_string(&mut context, &device_id);
        let options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, options)?;

        let config = AudioStreamConfig {
            sample_rate: 48_000,
            channels: 2,
            channel_layout: AudioChannelLayout::Stereo,
            channel_mask: 0b11,
            format: AudioSampleFormat::F32,
            period_frames: 128,
            transfer_mode: AudioStreamTransferMode::Push,
        };
        let config = harness_stream_config(&mut context, config);
        let stream = stream_open_with_default_options(&mut context, device, config)?;
        context.destack_audio_stream_start(stream)?;

        let payload = harness_bytes(&mut context, &[0u8; 256])?;
        let _ = assert_ok_or_expected_error(
            context.destack_audio_stream_try_write(stream, payload),
            &[PlatformErrorCode::IoWouldBlock],
        )?;

        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(target_os = "android")]
#[test]
fn test_audio_opensles_playback_open_start_stop_when_available() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_list = backend_support_rows(&mut context, backend_list)?;
        let opensles_available =
            backend_is_available_for_host_execution(&backend_list, AudioBackend::OpenSLES);
        if !opensles_available {
            return Ok(());
        }

        let device_id = match context.destack_audio_device_default(
            AudioDeviceDirection::Playback,
            AudioBackend::OpenSLES,
            AudioBackendSelectionPolicy::Strict,
        ) {
            Ok(value) => string_from_harness_value(&mut context, value)?,
            Err(error) => {
                let code = error_code_from_runtime_error(&error);
                if code == Some(PlatformErrorCode::IoNotFound) {
                    return Ok(());
                }
                return Err(error);
            }
        };

        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Playback,
            backend: AudioBackend::OpenSLES,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };
        let device_id = harness_string(&mut context, &device_id);
        let options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, options)?;

        let config = AudioStreamConfig {
            sample_rate: 48_000,
            channels: 2,
            channel_layout: AudioChannelLayout::Stereo,
            channel_mask: 0b11,
            format: AudioSampleFormat::F32,
            period_frames: 128,
            transfer_mode: AudioStreamTransferMode::Push,
        };
        let config = harness_stream_config(&mut context, config);
        let stream = stream_open_with_default_options(&mut context, device, config)?;
        context.destack_audio_stream_start(stream)?;

        let payload = harness_bytes(&mut context, &[0u8; 256])?;
        let _ = assert_ok_or_expected_error(
            context.destack_audio_stream_try_write(stream, payload),
            &[PlatformErrorCode::IoWouldBlock],
        )?;

        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_wasapi_loopback_open_start_stop_when_available() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_list = backend_support_rows(&mut context, backend_list)?;
        let wasapi_available =
            backend_is_available_for_host_execution(&backend_list, AudioBackend::Wasapi);
        if !wasapi_available {
            return Ok(());
        }

        let device_id = match context.destack_audio_device_default(
            AudioDeviceDirection::Loopback,
            AudioBackend::Wasapi,
            AudioBackendSelectionPolicy::Strict,
        ) {
            Ok(value) => string_from_harness_value(&mut context, value)?,
            Err(error) => {
                let code = error_code_from_runtime_error(&error);
                if code == Some(PlatformErrorCode::IoNotFound) {
                    return Ok(());
                }
                return Err(error);
            }
        };

        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Loopback,
            backend: AudioBackend::Wasapi,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };
        let device_id = harness_string(&mut context, &device_id);
        let options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, options)?;

        let config = AudioStreamConfig {
            sample_rate: 48_000,
            channels: 2,
            channel_layout: AudioChannelLayout::Stereo,
            channel_mask: 0b11,
            format: AudioSampleFormat::F32,
            period_frames: 128,
            transfer_mode: AudioStreamTransferMode::Push,
        };
        let config = harness_stream_config(&mut context, config);
        let Some(stream) = assert_ok_or_expected_error(
            stream_open_with_default_options(&mut context, device, config),
            &STREAM_OPEN_OPTIONAL_ERRORS,
        )?
        else {
            context.destack_audio_device_close(device)?;
            return Ok(());
        };

        let Some(()) = assert_ok_or_expected_error(
            context.destack_audio_stream_start(stream),
            &STREAM_START_OPTIONAL_ERRORS,
        )?
        else {
            context.destack_audio_stream_close(stream)?;
            context.destack_audio_device_close(device)?;
            return Ok(());
        };

        let _ = assert_ok_or_expected_error(
            context.destack_audio_stream_try_read(stream, 256),
            &[PlatformErrorCode::IoWouldBlock],
        )?;

        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_wasapi_duplex_open_start_stop_when_available() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_list = backend_support_rows(&mut context, backend_list)?;
        let wasapi_available =
            backend_is_available_for_host_execution(&backend_list, AudioBackend::Wasapi);
        if !wasapi_available {
            return Ok(());
        }

        let device_id = match context.destack_audio_device_default(
            AudioDeviceDirection::Duplex,
            AudioBackend::Wasapi,
            AudioBackendSelectionPolicy::Strict,
        ) {
            Ok(value) => string_from_harness_value(&mut context, value)?,
            Err(error) => {
                let code = error_code_from_runtime_error(&error);
                if code == Some(PlatformErrorCode::IoNotFound) {
                    return Ok(());
                }
                return Err(error);
            }
        };

        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Duplex,
            backend: AudioBackend::Wasapi,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };
        let device_id = harness_string(&mut context, &device_id);
        let options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, options)?;

        let config = AudioStreamConfig {
            sample_rate: 48_000,
            channels: 2,
            channel_layout: AudioChannelLayout::Stereo,
            channel_mask: 0b11,
            format: AudioSampleFormat::F32,
            period_frames: 128,
            transfer_mode: AudioStreamTransferMode::Push,
        };
        let config = harness_stream_config(&mut context, config);
        let stream = stream_open_with_default_options(&mut context, device, config)?;
        context.destack_audio_stream_start(stream)?;

        let write_payload = harness_bytes(&mut context, &[0u8; 256])?;
        let _ = assert_ok_or_expected_error(
            context.destack_audio_stream_try_write(stream, write_payload),
            &[PlatformErrorCode::IoWouldBlock],
        )?;

        let _ = assert_ok_or_expected_error(
            context.destack_audio_stream_try_read(stream, 256),
            &[PlatformErrorCode::IoWouldBlock],
        )?;

        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_null_stream_lifecycle_and_io() {
    with_harness_context(|mut context| {
        let (device, stream) = open_null_duplex_stream(&mut context)?;

        let payload = (0..512usize)
            .flat_map(|index| {
                let sample = (index as f32 / 512.0) * 0.25;
                sample.to_le_bytes()
            })
            .collect::<Vec<_>>();

        let write_payload = harness_bytes(&mut context, &payload)?;
        let written = context.destack_audio_stream_write(stream, write_payload)?;
        assert_eq!(
            written as usize,
            payload.len(),
            "stream.write should accept full payload"
        );

        std::thread::sleep(std::time::Duration::from_millis(20));

        let read = context.destack_audio_stream_read(stream, 1024)?;
        let read_len = byte_len(&mut context, read)?;
        assert!(
            read_len > 0,
            "duplex null stream should produce capture bytes"
        );

        let state = stream_state_from_value(context.destack_audio_stream_state(stream)?);
        assert!(state.running, "stream should report running after start");

        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_null_stream_descriptor_reports_expected_support_flags() {
    with_harness_context(|mut context| {
        let (device, stream) = open_null_duplex_stream(&mut context)?;

        let snapshot = context.destack_audio_stream_descriptor(stream)?;
        let (
            supports_write_at,
            supports_pause,
            supports_volume,
            supports_mute,
            supports_hardware_timestamps,
        ) = stream_descriptor_flags_from_value(snapshot);
        assert!(
            supports_write_at,
            "null stream should advertise scheduled writes to match backend and device capabilities",
        );
        assert!(
            supports_pause,
            "null stream should support pause and resume"
        );
        assert!(
            supports_volume,
            "null stream should support stream volume control"
        );
        assert!(
            supports_mute,
            "null stream should support stream mute control"
        );
        assert!(
            supports_hardware_timestamps,
            "null stream should expose synthetic timestamp support to match backend capabilities",
        );

        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_null_stream_write_at_accepts_scheduled_payload() {
    with_harness_context(|mut context| {
        let (device, stream) = open_null_duplex_stream(&mut context)?;

        let payload = harness_bytes(&mut context, &[0u8; 256])?;
        let now = context.destack_audio_clock_now(AudioClockDomain::Monotonic)?;
        let deadline = now.saturating_add(1_000_000);
        let written = context.destack_audio_stream_write_at(stream, payload, deadline)?;
        assert_eq!(written, 256, "stream.writeAt should report full byte count");

        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_stream_vectorized_io_and_controls() {
    with_harness_context(|mut context| {
        let (device, stream) = open_null_duplex_stream(&mut context)?;

        let first = vec![0u8; 256];
        let second = vec![1u8; 256];
        let writev = harness_bytes_slices(&mut context, &[&first, &second])?;
        let wrote = context.destack_audio_stream_writev(stream, writev)?;
        assert_eq!(wrote, 512, "stream.writev should report byte count");

        let try_writev = harness_bytes_slices(&mut context, &[&first, &second])?;
        let _ = assert_ok_or_expected_error(
            context.destack_audio_stream_try_writev(stream, try_writev),
            &[PlatformErrorCode::IoWouldBlock],
        )?;

        std::thread::sleep(std::time::Duration::from_millis(20));

        let mut read_buffers = vec![vec![0u8; 256], vec![0u8; 256]];
        let readv = harness_mutable_bytes_slices(&mut context, &mut read_buffers)?;
        let read_bytes = context.destack_audio_stream_readv(stream, readv)?;
        assert!(
            read_bytes > 0 && read_bytes <= 512,
            "stream.readv should read some bytes up to total buffer capacity",
        );

        let mut try_read_buffers = vec![vec![0u8; 128], vec![0u8; 128]];
        let try_readv = harness_mutable_bytes_slices(&mut context, &mut try_read_buffers)?;
        let _ = assert_ok_or_expected_error(
            context.destack_audio_stream_try_readv(stream, try_readv),
            &[PlatformErrorCode::IoWouldBlock],
        )?;

        context.destack_audio_stream_pause(stream, true)?;
        let paused = stream_state_from_value(context.destack_audio_stream_state(stream)?);
        assert_eq!(paused.state, AudioStreamStateKind::Paused);
        assert!(paused.paused);

        context.destack_audio_stream_pause(stream, false)?;
        let running = stream_state_from_value(context.destack_audio_stream_state(stream)?);
        assert_eq!(running.state, AudioStreamStateKind::Running);
        assert!(running.running);

        context.destack_audio_stream_set_mute(stream, true)?;
        context.destack_audio_stream_set_mute(stream, false)?;
        let stream_name = harness_string(&mut context, "music");
        context.destack_audio_stream_set_name(stream, stream_name)?;
        context.destack_audio_stream_set_volume(stream, 0.25)?;
        assert_platform_error_code(
            context.destack_audio_stream_set_volume(stream, -1.0),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        assert_platform_error_code(
            context.destack_audio_stream_set_volume(stream, f64::NAN),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        context.destack_audio_stream_abort(stream)?;
        let stopped = stream_state_from_value(context.destack_audio_stream_state(stream)?);
        assert_eq!(stopped.state, AudioStreamStateKind::Stopped);
        assert!(!stopped.running);

        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;
        Ok(())
    });
}
