use super::super::{
    AudioBackend, AudioBackendSelectionPolicy, AudioEventSubscriptionFlags,
    AudioEventSubscriptionOptions, core as audio_core,
};
use super::core::{harness_event_options, open_null_duplex_stream};
use super::{assert_ok_or_expected_error, assert_platform_error_code, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{AudioStreamHandle, ResourceId};

/// Build one default null backend event subscription payload.
fn default_event_options() -> AudioEventSubscriptionOptions {
    AudioEventSubscriptionOptions {
        backend: AudioBackend::Null,
        backend_policy: AudioBackendSelectionPolicy::Strict,
        flags: AudioEventSubscriptionFlags(0),
        has_stream: false,
        stream: AudioStreamHandle(ResourceId(0)),
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
        event_options.has_stream = true;
        event_options.stream = AudioStreamHandle(ResourceId(999_999));

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
        event_options.has_stream = true;
        event_options.stream = stream;

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
