use super::super::{
    AudioBackend, AudioBackendSelectionPolicy, AudioEventSubscriptionFlags,
    AudioEventSubscriptionOptions,
};
use super::core::harness_event_options;
use super::{assert_ok_or_expected_error, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{AudioStreamHandle, ResourceId};

#[cfg(any(unix, windows))]
#[test]
fn test_audio_event_open_try_read_close_roundtrip() {
    with_harness_context(|mut context| {
        let event_options = AudioEventSubscriptionOptions {
            backend: AudioBackend::Null,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            flags: AudioEventSubscriptionFlags(0),
            has_stream: false,
            stream: AudioStreamHandle(ResourceId(0)),
            queue_capacity: 0,
            poll_interval_ns: 0,
        };

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
