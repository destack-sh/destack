use super::super::{AudioClockDomain, AudioStreamClockDomain};
use super::core::{clock_snapshot_from_value, open_null_duplex_stream, open_null_playback_stream};
use super::{assert_platform_error_code, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;

#[cfg(any(unix, windows))]
#[test]
fn test_audio_clock_now_returns_monotonic_and_wall_samples() {
    with_harness_context(|mut context| {
        let monotonic_ns = context.destack_audio_clock_now(AudioClockDomain::Monotonic)?;
        let wall_ns = context.destack_audio_clock_now(AudioClockDomain::Wall)?;
        assert!(monotonic_ns > 0, "clock.now monotonic should be non-zero");
        assert!(wall_ns > 0, "clock.now wall should be non-zero");
        assert_platform_error_code(
            context.destack_audio_clock_now(AudioClockDomain::Device),
            PlatformErrorCode::NotSupported,
        )?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_stream_clock_returns_device_and_endpoint_domains() {
    with_harness_context(|mut context| {
        let (device, stream) = open_null_duplex_stream(&mut context)?;

        let device_snapshot = clock_snapshot_from_value(
            context.destack_audio_stream_clock(stream, AudioStreamClockDomain::Device)?,
        );
        let callback_snapshot = clock_snapshot_from_value(
            context.destack_audio_stream_clock(stream, AudioStreamClockDomain::Callback)?,
        );
        let input_snapshot = clock_snapshot_from_value(
            context.destack_audio_stream_clock(stream, AudioStreamClockDomain::InputAdc)?,
        );
        let output_snapshot = clock_snapshot_from_value(
            context.destack_audio_stream_clock(stream, AudioStreamClockDomain::OutputDac)?,
        );

        assert!(
            device_snapshot.clock_ns > 0,
            "stream.clock device domain should be non-zero",
        );
        assert!(
            callback_snapshot.clock_ns > 0,
            "stream.clock callback domain should be non-zero",
        );
        assert!(
            input_snapshot.clock_ns > 0,
            "stream.clock input domain should be non-zero",
        );
        assert!(
            output_snapshot.clock_ns > 0,
            "stream.clock output domain should be non-zero",
        );

        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_stream_clock_rejects_input_adc_for_playback_streams() {
    with_harness_context(|mut context| {
        let (device, stream) = open_null_playback_stream(&mut context)?;

        assert_platform_error_code(
            context.destack_audio_stream_clock(stream, AudioStreamClockDomain::InputAdc),
            PlatformErrorCode::NotSupported,
        )?;

        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}
