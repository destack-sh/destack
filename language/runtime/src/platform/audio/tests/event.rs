use super::super::{
    AudioBackend, AudioBackendSelectionPolicy, AudioEventDeliveryMode, AudioEventKind,
    AudioEventOverflowPolicy, AudioEventSource, AudioEventSubscriptionFlags,
    AudioEventSubscriptionOptions, core as audio_core,
};
#[cfg(windows)]
use super::core::backend_availability_rows;
use super::core::{
    DeterministicSequence, backend_event_support_rows, event_batch_kind_rows, event_batch_len,
    event_batch_sequence_rows, harness_event_options, open_null_duplex_stream,
    open_null_playback_stream,
};
use super::{assert_ok_or_expected_error, assert_platform_error_code, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{AudioStreamHandle, ResourceId};

/// Build one default null backend event subscription payload.
fn default_event_options() -> AudioEventSubscriptionOptions {
    AudioEventSubscriptionOptions {
        backend: AudioBackend::Null,
        backend_policy: AudioBackendSelectionPolicy::Strict,
        flags: AudioEventSubscriptionFlags(0),
        delivery_mode: AudioEventDeliveryMode::Auto,
        overflow_policy: AudioEventOverflowPolicy::DropOldest,
        stream: None,
        queue_capacity: 0,
        poll_interval_ns: 0,
    }
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_event_open_try_read_close_roundtrip() {
    with_harness_context(|mut context| {
        let event_options = default_event_options();
        let event_options = harness_event_options(&mut context, event_options);
        let events = context.destack_audio_event_open(event_options)?;
        let _ = assert_ok_or_expected_error(
            context.destack_audio_event_try_read(events),
            &[PlatformErrorCode::IoWouldBlock],
        )?;
        context.destack_audio_event_close(events)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_event_open_rejects_unknown_subscription_flag_bits() {
    with_harness_context(|mut context| {
        let mut event_options = default_event_options();
        event_options.flags = AudioEventSubscriptionFlags(0x8000_0000);

        let event_options = harness_event_options(&mut context, event_options);
        assert_platform_error_code(
            context.destack_audio_event_open(event_options),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_event_open_native_only_delivers_stream_events_without_polling() {
    with_harness_context(|mut context| {
        let (device, stream) = open_null_duplex_stream(&mut context)?;

        let mut event_options = default_event_options();
        event_options.flags = audio_core::EVENT_SUBSCRIBE_STREAM;
        event_options.stream = Some(stream);
        event_options.delivery_mode = AudioEventDeliveryMode::NativeOnly;
        let event_options = harness_event_options(&mut context, event_options);
        let events = context.destack_audio_event_open(event_options)?;

        context.destack_audio_stream_stop(stream)?;
        let batch = context.destack_audio_event_read_batch(events, 8, 50_000_000)?;
        let count = event_batch_len(&mut context, batch)?;
        assert!(
            count >= 1,
            "native-only delivery should queue stream control transitions",
        );

        context.destack_audio_event_close(events)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_event_open_native_only_reports_stream_xruns_for_null_playback() {
    with_harness_context(|mut context| {
        let (device, stream) = open_null_playback_stream(&mut context)?;

        let mut event_options = default_event_options();
        event_options.flags = audio_core::EVENT_SUBSCRIBE_STREAM;
        event_options.stream = Some(stream);
        event_options.delivery_mode = AudioEventDeliveryMode::NativeOnly;
        let event_options = harness_event_options(&mut context, event_options);
        let events = context.destack_audio_event_open(event_options)?;

        let mut observed_xrun = false;
        for _ in 0..8 {
            let batch = assert_ok_or_expected_error(
                context.destack_audio_event_read_batch(events, 16, 50_000_000),
                &[PlatformErrorCode::IoWouldBlock],
            )?;
            let Some(batch) = batch else {
                continue;
            };
            let rows = event_batch_kind_rows(&mut context, batch)?;
            if rows.iter().any(|(kind, xrun_delta, source)| {
                *kind == AudioEventKind::StreamXRun
                    && *xrun_delta > 0
                    && *source == AudioEventSource::Native
            }) {
                observed_xrun = true;
                break;
            }
        }

        assert!(
            observed_xrun,
            "native-only delivery should surface null-backend stream xruns",
        );

        context.destack_audio_event_close(events)?;
        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_event_open_native_only_rejects_unavailable_device_lanes() {
    with_harness_context(|mut context| {
        let mut event_options = default_event_options();
        event_options.flags = audio_core::EVENT_SUBSCRIBE_DEVICE_HOTPLUG;
        event_options.delivery_mode = AudioEventDeliveryMode::NativeOnly;

        let event_options = harness_event_options(&mut context, event_options);
        assert_platform_error_code(
            context.destack_audio_event_open(event_options),
            PlatformErrorCode::NotSupported,
        )?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_event_open_native_only_rejects_default_all_flags_subscription() {
    with_harness_context(|mut context| {
        let mut event_options = default_event_options();
        event_options.delivery_mode = AudioEventDeliveryMode::NativeOnly;

        let event_options = harness_event_options(&mut context, event_options);
        assert_platform_error_code(
            context.destack_audio_event_open(event_options),
            PlatformErrorCode::NotSupported,
        )?;

        Ok(())
    });
}

#[cfg(target_os = "macos")]
#[test]
fn test_audio_event_open_native_only_accepts_coreaudio_device_lanes_when_available() {
    with_harness_context(|mut context| {
        let mut event_options = default_event_options();
        event_options.backend = AudioBackend::CoreAudio;
        event_options.backend_policy = AudioBackendSelectionPolicy::Strict;
        event_options.flags = audio_core::EVENT_SUBSCRIBE_DEVICE_HOTPLUG;
        event_options.delivery_mode = AudioEventDeliveryMode::NativeOnly;

        let event_options = harness_event_options(&mut context, event_options);
        let result = context.destack_audio_event_open(event_options);
        match result {
            Ok(events) => {
                context.destack_audio_event_close(events)?;
                Ok(())
            }
            Err(error) => {
                let code = error.platform_error().map(|platform| platform.code);
                if code == Some(PlatformErrorCode::IoNotFound)
                    || code == Some(PlatformErrorCode::IoPermissionDenied)
                    || code == Some(PlatformErrorCode::AudioUnavailable)
                    || code == Some(PlatformErrorCode::DeviceUnavailable)
                {
                    return Ok(());
                }

                assert_ne!(
                    code,
                    Some(PlatformErrorCode::NotSupported),
                    "CoreAudio native-only device subscriptions should not report notSupported",
                );
                Err(error)
            }
        }
    });
}

#[cfg(windows)]
#[test]
fn test_audio_event_open_native_only_accepts_wasapi_device_lanes_when_available() {
    with_harness_context(|mut context| {
        let mut event_options = default_event_options();
        event_options.backend = AudioBackend::Wasapi;
        event_options.backend_policy = AudioBackendSelectionPolicy::Strict;
        event_options.flags = audio_core::EVENT_SUBSCRIBE_DEVICE_HOTPLUG;
        event_options.delivery_mode = AudioEventDeliveryMode::NativeOnly;

        let event_options = harness_event_options(&mut context, event_options);
        let result = context.destack_audio_event_open(event_options);
        match result {
            Ok(events) => {
                context.destack_audio_event_close(events)?;
                Ok(())
            }
            Err(error) => {
                let code = error.platform_error().map(|platform| platform.code);
                if code == Some(PlatformErrorCode::IoNotFound)
                    || code == Some(PlatformErrorCode::IoPermissionDenied)
                    || code == Some(PlatformErrorCode::AudioUnavailable)
                    || code == Some(PlatformErrorCode::DeviceUnavailable)
                {
                    return Ok(());
                }

                assert_ne!(
                    code,
                    Some(PlatformErrorCode::NotSupported),
                    "WASAPI native-only device subscriptions should not report notSupported",
                );
                Err(error)
            }
        }
    });
}

#[cfg(all(target_os = "linux", feature = "audio-jack"))]
#[test]
fn test_audio_event_open_native_only_accepts_jack_device_lanes_when_available() {
    with_harness_context(|mut context| {
        let mut event_options = default_event_options();
        event_options.backend = AudioBackend::Jack;
        event_options.backend_policy = AudioBackendSelectionPolicy::Strict;
        event_options.flags = audio_core::EVENT_SUBSCRIBE_DEVICE_HOTPLUG;
        event_options.delivery_mode = AudioEventDeliveryMode::NativeOnly;

        let event_options = harness_event_options(&mut context, event_options);
        let result = context.destack_audio_event_open(event_options);
        match result {
            Ok(events) => {
                context.destack_audio_event_close(events)?;
                Ok(())
            }
            Err(error) => {
                let code = error.platform_error().map(|platform| platform.code);
                if code == Some(PlatformErrorCode::IoNotFound)
                    || code == Some(PlatformErrorCode::IoPermissionDenied)
                    || code == Some(PlatformErrorCode::AudioUnavailable)
                    || code == Some(PlatformErrorCode::DeviceUnavailable)
                {
                    return Ok(());
                }

                assert_ne!(
                    code,
                    Some(PlatformErrorCode::NotSupported),
                    "JACK native-only device subscriptions should not report notSupported",
                );
                Err(error)
            }
        }
    });
}

#[cfg(all(target_os = "linux", feature = "audio-pulseaudio"))]
#[test]
fn test_audio_event_open_native_only_accepts_pulseaudio_device_lanes_when_available() {
    with_harness_context(|mut context| {
        let mut event_options = default_event_options();
        event_options.backend = AudioBackend::PulseAudio;
        event_options.backend_policy = AudioBackendSelectionPolicy::Strict;
        event_options.flags = audio_core::EVENT_SUBSCRIBE_DEVICE_HOTPLUG;
        event_options.delivery_mode = AudioEventDeliveryMode::NativeOnly;

        let event_options = harness_event_options(&mut context, event_options);
        let result = context.destack_audio_event_open(event_options);
        match result {
            Ok(events) => {
                context.destack_audio_event_close(events)?;
                Ok(())
            }
            Err(error) => {
                let code = error.platform_error().map(|platform| platform.code);
                if code == Some(PlatformErrorCode::IoNotFound)
                    || code == Some(PlatformErrorCode::IoPermissionDenied)
                    || code == Some(PlatformErrorCode::AudioUnavailable)
                    || code == Some(PlatformErrorCode::DeviceUnavailable)
                    || code == Some(PlatformErrorCode::IoInvalidData)
                {
                    return Ok(());
                }

                assert_ne!(
                    code,
                    Some(PlatformErrorCode::NotSupported),
                    "PulseAudio native-only device subscriptions should not report notSupported",
                );
                Err(error)
            }
        }
    });
}

#[cfg(all(target_os = "linux", feature = "audio-pipewire"))]
#[test]
fn test_audio_event_open_native_only_accepts_pipewire_device_lanes_when_available() {
    with_harness_context(|mut context| {
        let mut event_options = default_event_options();
        event_options.backend = AudioBackend::PipeWire;
        event_options.backend_policy = AudioBackendSelectionPolicy::Strict;
        event_options.flags = audio_core::EVENT_SUBSCRIBE_DEVICE_HOTPLUG;
        event_options.delivery_mode = AudioEventDeliveryMode::NativeOnly;

        let event_options = harness_event_options(&mut context, event_options);
        let result = context.destack_audio_event_open(event_options);
        match result {
            Ok(events) => {
                context.destack_audio_event_close(events)?;
                Ok(())
            }
            Err(error) => {
                let code = error.platform_error().map(|platform| platform.code);
                if code == Some(PlatformErrorCode::IoNotFound)
                    || code == Some(PlatformErrorCode::IoPermissionDenied)
                    || code == Some(PlatformErrorCode::AudioUnavailable)
                    || code == Some(PlatformErrorCode::DeviceUnavailable)
                    || code == Some(PlatformErrorCode::IoInvalidData)
                {
                    return Ok(());
                }

                assert_ne!(
                    code,
                    Some(PlatformErrorCode::NotSupported),
                    "PipeWire native-only device subscriptions should not report notSupported",
                );
                Err(error)
            }
        }
    });
}

#[cfg(all(target_os = "linux", feature = "audio-alsa"))]
#[test]
fn test_audio_event_open_native_only_accepts_alsa_device_lanes_when_available() {
    with_harness_context(|mut context| {
        let mut event_options = default_event_options();
        event_options.backend = AudioBackend::Alsa;
        event_options.backend_policy = AudioBackendSelectionPolicy::Strict;
        event_options.flags = audio_core::EVENT_SUBSCRIBE_DEVICE_HOTPLUG;
        event_options.delivery_mode = AudioEventDeliveryMode::NativeOnly;

        let event_options = harness_event_options(&mut context, event_options);
        let result = context.destack_audio_event_open(event_options);
        match result {
            Ok(events) => {
                context.destack_audio_event_close(events)?;
                Ok(())
            }
            Err(error) => {
                let code = error.platform_error().map(|platform| platform.code);
                if code == Some(PlatformErrorCode::IoNotFound)
                    || code == Some(PlatformErrorCode::IoPermissionDenied)
                    || code == Some(PlatformErrorCode::AudioUnavailable)
                    || code == Some(PlatformErrorCode::DeviceUnavailable)
                    || code == Some(PlatformErrorCode::IoInvalidData)
                {
                    return Ok(());
                }

                assert_ne!(
                    code,
                    Some(PlatformErrorCode::NotSupported),
                    "ALSA native-only device subscriptions should not report notSupported",
                );
                Err(error)
            }
        }
    });
}

#[cfg(all(windows, feature = "audio-asio"))]
#[test]
fn test_audio_event_open_native_only_accepts_asio_device_lanes_when_available() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_list = backend_availability_rows(&mut context, backend_list)?;
        let asio_available = backend_list
            .iter()
            .any(|(backend, available)| *backend == AudioBackend::Asio && *available);
        if !asio_available {
            return Ok(());
        }

        let mut event_options = default_event_options();
        event_options.backend = AudioBackend::Asio;
        event_options.backend_policy = AudioBackendSelectionPolicy::Strict;
        event_options.flags = audio_core::EVENT_SUBSCRIBE_DEVICE_HOTPLUG;
        event_options.delivery_mode = AudioEventDeliveryMode::NativeOnly;

        let event_options = harness_event_options(&mut context, event_options);
        let result = context.destack_audio_event_open(event_options);
        match result {
            Ok(events) => {
                context.destack_audio_event_close(events)?;
                Ok(())
            }
            Err(error) => {
                let code = error.platform_error().map(|platform| platform.code);
                if code == Some(PlatformErrorCode::IoNotFound)
                    || code == Some(PlatformErrorCode::IoPermissionDenied)
                    || code == Some(PlatformErrorCode::AudioUnavailable)
                    || code == Some(PlatformErrorCode::DeviceUnavailable)
                    || code == Some(PlatformErrorCode::IoInvalidData)
                {
                    return Ok(());
                }

                assert_ne!(
                    code,
                    Some(PlatformErrorCode::NotSupported),
                    "ASIO native-only device subscriptions should not report notSupported",
                );
                Err(error)
            }
        }
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_event_open_accepts_poll_only_delivery_mode() {
    with_harness_context(|mut context| {
        let mut event_options = default_event_options();
        event_options.delivery_mode = AudioEventDeliveryMode::PollOnly;

        let event_options = harness_event_options(&mut context, event_options);
        let events = context.destack_audio_event_open(event_options)?;
        context.destack_audio_event_close(events)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_event_open_rejects_stream_flags_without_stream_target() {
    with_harness_context(|mut context| {
        let mut event_options = default_event_options();
        event_options.flags = audio_core::EVENT_SUBSCRIBE_STREAM;

        let event_options = harness_event_options(&mut context, event_options);
        assert_platform_error_code(
            context.destack_audio_event_open(event_options),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_event_open_rejects_unknown_stream_handle() {
    with_harness_context(|mut context| {
        let mut event_options = default_event_options();
        event_options.flags = audio_core::EVENT_SUBSCRIBE_STREAM;
        event_options.stream = Some(AudioStreamHandle(ResourceId(999_999)));

        let event_options = harness_event_options(&mut context, event_options);
        assert_platform_error_code(
            context.destack_audio_event_open(event_options),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_event_open_with_stream_target_has_no_initial_snapshot_events() {
    with_harness_context(|mut context| {
        let (device, stream) = open_null_duplex_stream(&mut context)?;

        let mut event_options = default_event_options();
        event_options.flags = audio_core::EVENT_SUBSCRIBE_STREAM;
        event_options.stream = Some(stream);

        let event_options = harness_event_options(&mut context, event_options);
        let events = context.destack_audio_event_open(event_options)?;
        assert_platform_error_code(
            context.destack_audio_event_try_read(events),
            PlatformErrorCode::IoWouldBlock,
        )?;

        context.destack_audio_event_close(events)?;
        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_event_batch_reads_report_would_block_for_empty_queue() {
    with_harness_context(|mut context| {
        let event_options = harness_event_options(&mut context, default_event_options());
        let events = context.destack_audio_event_open(event_options)?;
        assert_platform_error_code(
            context.destack_audio_event_try_read_batch(events, 8),
            PlatformErrorCode::IoWouldBlock,
        )?;
        assert_platform_error_code(
            context.destack_audio_event_read_batch(events, 8, 0),
            PlatformErrorCode::IoWouldBlock,
        )?;

        context.destack_audio_event_close(events)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_event_batch_reads_reject_zero_max_events() {
    with_harness_context(|mut context| {
        let event_options = harness_event_options(&mut context, default_event_options());
        let events = context.destack_audio_event_open(event_options)?;
        assert_platform_error_code(
            context.destack_audio_event_try_read_batch(events, 0),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        assert_platform_error_code(
            context.destack_audio_event_read_batch(events, 0, 1_000_000),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        context.destack_audio_event_close(events)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_event_batch_read_returns_events_when_stream_changes_state() {
    with_harness_context(|mut context| {
        let (device, stream) = open_null_duplex_stream(&mut context)?;

        let mut event_options = default_event_options();
        event_options.flags = audio_core::EVENT_SUBSCRIBE_STREAM;
        event_options.stream = Some(stream);
        event_options.poll_interval_ns = audio_core::MIN_EVENT_POLL_INTERVAL_NS;

        let event_options = harness_event_options(&mut context, event_options);
        let events = context.destack_audio_event_open(event_options)?;
        context.destack_audio_stream_stop(stream)?;

        let batch = assert_ok_or_expected_error(
            context.destack_audio_event_read_batch(events, 8, 50_000_000),
            &[PlatformErrorCode::IoWouldBlock],
        )?;
        if let Some(batch) = batch {
            let count = event_batch_len(&mut context, batch)?;
            assert!(
                count >= 1,
                "stream state transitions should produce at least one queued event",
            );
        }

        context.destack_audio_event_close(events)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_event_sequence_rows_stay_monotonic_during_random_stream_churn() {
    with_harness_context(|mut context| {
        let (device, stream) = open_null_duplex_stream(&mut context)?;

        let mut event_options = default_event_options();
        event_options.flags = audio_core::EVENT_SUBSCRIBE_STREAM;
        event_options.stream = Some(stream);
        event_options.queue_capacity = 2;
        event_options.poll_interval_ns = audio_core::MIN_EVENT_POLL_INTERVAL_NS;

        let event_options = harness_event_options(&mut context, event_options);
        let events = context.destack_audio_event_open(event_options)?;
        let mut random = DeterministicSequence::new(0xA517_D3C4_0055_1109);
        let mut last_sequence = 0u64;
        let mut last_dropped_count = 0u64;
        let mut observed_rows = 0usize;

        for _ in 0..96 {
            let stream_result = if random.next_bool() {
                context.destack_audio_stream_start(stream)
            } else {
                context.destack_audio_stream_stop(stream)
            };
            let _ = assert_ok_or_expected_error(stream_result, &[PlatformErrorCode::IoWouldBlock])?;

            let batch = assert_ok_or_expected_error(
                context.destack_audio_event_read_batch(events, 4, 50_000_000),
                &[PlatformErrorCode::IoWouldBlock],
            )?;
            let Some(batch) = batch else {
                continue;
            };
            let rows = event_batch_sequence_rows(&mut context, batch)?;

            for (sequence, dropped_count) in rows {
                assert!(
                    sequence > last_sequence,
                    "event sequence should be strictly increasing",
                );
                assert!(
                    dropped_count >= last_dropped_count,
                    "event dropped count should be monotonic",
                );
                last_sequence = sequence;
                last_dropped_count = dropped_count;
                observed_rows += 1;
            }
        }

        assert!(
            observed_rows > 0,
            "random stream churn should eventually produce stream events",
        );

        context.destack_audio_event_close(events)?;
        let _ = context.destack_audio_stream_stop(stream);
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_event_open_matches_backend_advertised_device_subscription_flags() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_event_support_rows(&mut context, rows)?;
        let flag_rows = [
            ("device hotplug", audio_core::EVENT_SUBSCRIBE_DEVICE_HOTPLUG),
            ("default route", audio_core::EVENT_SUBSCRIBE_DEFAULT_ROUTE),
            ("format change", audio_core::EVENT_SUBSCRIBE_FORMAT_CHANGE),
            ("reroute", audio_core::EVENT_SUBSCRIBE_REROUTE),
        ];

        for (backend, available, supported_flags) in rows {
            if !available || backend == AudioBackend::Auto {
                continue;
            }

            for (label, flag) in flag_rows {
                let mut options = default_event_options();
                options.backend = backend;
                options.backend_policy = AudioBackendSelectionPolicy::Strict;
                options.flags = flag;

                let options = harness_event_options(&mut context, options);
                let result = context.destack_audio_event_open(options);
                let flag_supported = (supported_flags.0 & flag.0) != 0;
                if flag_supported {
                    match result {
                        Ok(events) => {
                            context.destack_audio_event_close(events)?;
                        }
                        Err(error) => {
                            let code = error.platform_error().map(|platform| platform.code);
                            if code == Some(PlatformErrorCode::IoNotFound)
                                || code == Some(PlatformErrorCode::IoPermissionDenied)
                                || code == Some(PlatformErrorCode::AudioUnavailable)
                                || code == Some(PlatformErrorCode::DeviceUnavailable)
                            {
                                continue;
                            }

                            assert_ne!(
                                code,
                                Some(PlatformErrorCode::NotSupported),
                                "backend {backend:?} advertises {label} subscription support but event.open returned notSupported",
                            );
                            return Err(error);
                        }
                    }
                } else {
                    assert_platform_error_code(result, PlatformErrorCode::NotSupported)?;
                }
            }
        }

        Ok(())
    });
}
