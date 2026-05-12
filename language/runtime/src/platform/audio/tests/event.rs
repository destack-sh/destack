use super::super::{
    AudioBackend, AudioBackendSelectionPolicy, AudioEventDeliveryMode, AudioEventKind,
    AudioEventOverflowPolicy, AudioEventSource, AudioEventSubscriptionFlags,
    AudioEventSubscriptionOptions, core as audio_core,
};
use super::core::{
    DeterministicSequence, backend_event_support_rows, event_batch_len, event_batch_sequence_rows,
    harness_event_options, open_null_duplex_stream,
};
use super::{
    assert_code_is_one_of, assert_not_supported_result, assert_ok_or_expected_error,
    assert_platform_error_code, error_code_from_runtime_error, with_harness_context,
};
use crate::platform::core::BackendSupport;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{AudioStreamHandle, ResourceId};

const HOST_AUDIO_EVENT_ALLOWED_ERRORS: [PlatformErrorCode; 4] = [
    PlatformErrorCode::IoNotFound,
    PlatformErrorCode::IoPermissionDenied,
    PlatformErrorCode::AudioUnavailable,
    PlatformErrorCode::DeviceUnavailable,
];
const RANDOM_STREAM_CHURN_ITERATIONS: usize = 32;
const RANDOM_STREAM_CHURN_READ_TIMEOUT_NS: u64 = 10_000_000;

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
        event_options.stream = Some(AudioStreamHandle(ResourceId::local(999_999)));

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

        for _ in 0..RANDOM_STREAM_CHURN_ITERATIONS {
            let stream_result = if random.next_bool() {
                context.destack_audio_stream_start(stream)
            } else {
                context.destack_audio_stream_stop(stream)
            };
            let _ = assert_ok_or_expected_error(stream_result, &[PlatformErrorCode::IoWouldBlock])?;

            let batch = assert_ok_or_expected_error(
                context.destack_audio_event_read_batch(
                    events,
                    4,
                    RANDOM_STREAM_CHURN_READ_TIMEOUT_NS,
                ),
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

        for (backend, support, supported_flags) in rows {
            if support != BackendSupport::Available || backend == AudioBackend::Auto {
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
                            let code = error_code_from_runtime_error(&error);
                            assert_code_is_one_of(
                                code,
                                &HOST_AUDIO_EVENT_ALLOWED_ERRORS,
                                &format!(
                                    "backend {backend:?} advertises {label} subscription support but event.open failed outside the allowed host error set"
                                ),
                            )?;
                        }
                    }
                } else {
                    assert_not_supported_result(result)?;
                }
            }
        }

        Ok(())
    });
}
