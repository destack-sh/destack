use std::thread;
use std::time::Duration;

use super::super::core::{
    BACKEND_CAPABILITY_EXCLUSIVE_MODE, BACKEND_CAPABILITY_SHARED_MODE,
    SUPPORTED_STREAM_CLOCK_CALLBACK, SUPPORTED_STREAM_CLOCK_DEVICE,
    SUPPORTED_STREAM_CLOCK_INPUT_ADC, SUPPORTED_STREAM_CLOCK_MONOTONIC,
    SUPPORTED_STREAM_CLOCK_OUTPUT_DAC, SUPPORTED_STREAM_CLOCK_WALL,
};
use super::super::{
    AudioBackend, AudioBackendSelectionPolicy, AudioChannelLayout, AudioClockDomain,
    AudioDeviceDirection, AudioDeviceOpenFlags, AudioDeviceOpenOptions, AudioSampleFormat,
    AudioShareMode, AudioStreamClockDomain, AudioStreamConfig, AudioStreamTransferMode,
};
use super::core::{
    backend_availability_rows_with_capabilities, clock_snapshot_from_value,
    device_descriptor_stream_clock_domains_from_value, harness_device_options,
    harness_stream_config, harness_string, open_null_duplex_stream, open_null_playback_stream,
    stream_open_with_default_options, string_from_harness_value,
};
use super::{assert_ok_or_expected_error, assert_platform_error_code, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;

#[cfg(any(unix, windows))]
#[test]
fn test_audio_clock_now_returns_monotonic_and_wall_samples() {
    with_harness_context(|mut context| {
        let monotonic_ns = context.destack_audio_clock_now(AudioClockDomain::Monotonic)?;
        let wall_ns = context.destack_audio_clock_now(AudioClockDomain::Wall)?;
        assert!(monotonic_ns > 0, "clock.now monotonic should be non-zero");
        assert!(wall_ns > 0, "clock.now wall should be non-zero");

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_stream_clock_returns_device_and_endpoint_domains() {
    with_harness_context(|mut context| {
        let (device, stream) = open_null_duplex_stream(&mut context)?;

        let mut device_snapshot = clock_snapshot_from_value(
            context.destack_audio_stream_clock(stream, AudioStreamClockDomain::Device)?,
        );
        let mut callback_snapshot = clock_snapshot_from_value(
            context.destack_audio_stream_clock(stream, AudioStreamClockDomain::Callback)?,
        );
        let mut input_snapshot = clock_snapshot_from_value(
            context.destack_audio_stream_clock(stream, AudioStreamClockDomain::InputAdc)?,
        );
        let mut output_snapshot = clock_snapshot_from_value(
            context.destack_audio_stream_clock(stream, AudioStreamClockDomain::OutputDac)?,
        );

        // wait briefly for one callback cycle to publish endpoint timing lanes
        for _ in 0..20 {
            if device_snapshot.has_device_ns
                && callback_snapshot.has_callback_ns
                && input_snapshot.has_input_adc_ns
                && output_snapshot.has_output_dac_ns
            {
                break;
            }

            thread::sleep(Duration::from_millis(5));
            device_snapshot = clock_snapshot_from_value(
                context.destack_audio_stream_clock(stream, AudioStreamClockDomain::Device)?,
            );
            callback_snapshot = clock_snapshot_from_value(
                context.destack_audio_stream_clock(stream, AudioStreamClockDomain::Callback)?,
            );
            input_snapshot = clock_snapshot_from_value(
                context.destack_audio_stream_clock(stream, AudioStreamClockDomain::InputAdc)?,
            );
            output_snapshot = clock_snapshot_from_value(
                context.destack_audio_stream_clock(stream, AudioStreamClockDomain::OutputDac)?,
            );
        }

        assert!(
            device_snapshot.has_device_ns && device_snapshot.clock_ns > 0,
            "stream.clock device domain should be available and non-zero",
        );
        assert!(
            callback_snapshot.has_callback_ns && callback_snapshot.clock_ns > 0,
            "stream.clock callback domain should be available and non-zero",
        );
        assert!(
            input_snapshot.has_input_adc_ns && input_snapshot.clock_ns > 0,
            "stream.clock input domain should be available and non-zero",
        );
        assert!(
            output_snapshot.has_output_dac_ns && output_snapshot.clock_ns > 0,
            "stream.clock output domain should be available and non-zero",
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

#[cfg(any(unix, windows))]
#[test]
fn test_audio_stream_clock_domain_support_matches_device_descriptor_for_available_backends() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_rows = backend_availability_rows_with_capabilities(&mut context, backend_list)?;
        let domain_rows = [
            (
                AudioStreamClockDomain::Monotonic,
                SUPPORTED_STREAM_CLOCK_MONOTONIC.0,
            ),
            (AudioStreamClockDomain::Wall, SUPPORTED_STREAM_CLOCK_WALL.0),
            (
                AudioStreamClockDomain::Callback,
                SUPPORTED_STREAM_CLOCK_CALLBACK.0,
            ),
            (
                AudioStreamClockDomain::Device,
                SUPPORTED_STREAM_CLOCK_DEVICE.0,
            ),
            (
                AudioStreamClockDomain::InputAdc,
                SUPPORTED_STREAM_CLOCK_INPUT_ADC.0,
            ),
            (
                AudioStreamClockDomain::OutputDac,
                SUPPORTED_STREAM_CLOCK_OUTPUT_DAC.0,
            ),
        ];

        for (backend, available, capability_flags) in backend_rows {
            if !available || backend == AudioBackend::Auto {
                continue;
            }

            let default_id = if backend == AudioBackend::Null {
                "audio:null:playback".to_string()
            } else {
                match context.destack_audio_device_default(
                    AudioDeviceDirection::Playback,
                    backend,
                    AudioBackendSelectionPolicy::Strict,
                ) {
                    Ok(value) => string_from_harness_value(&mut context, value)?,
                    Err(error) => {
                        let code = error.platform_error().map(|platform| platform.code);
                        if code == Some(PlatformErrorCode::IoNotFound) {
                            continue;
                        }

                        return Err(error);
                    }
                }
            };
            let share_mode = if (capability_flags.0 & BACKEND_CAPABILITY_SHARED_MODE.0) != 0 {
                AudioShareMode::Shared
            } else if (capability_flags.0 & BACKEND_CAPABILITY_EXCLUSIVE_MODE.0) != 0 {
                AudioShareMode::Exclusive
            } else {
                continue;
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
                    let code = error.platform_error().map(|platform| platform.code);
                    if code == Some(PlatformErrorCode::IoNotFound)
                        || code == Some(PlatformErrorCode::IoPermissionDenied)
                        || code == Some(PlatformErrorCode::AudioUnavailable)
                        || code == Some(PlatformErrorCode::DeviceUnavailable)
                    {
                        continue;
                    }

                    return Err(error);
                }
            };

            let stream_config = AudioStreamConfig {
                sample_rate: 48_000,
                channels: 1,
                channel_layout: AudioChannelLayout::Mono,
                channel_mask: 0b1,
                format: AudioSampleFormat::F32,
                period_frames: 128,
                transfer_mode: AudioStreamTransferMode::Push,
            };
            let stream_config = harness_stream_config(&mut context, stream_config);
            let stream = match stream_open_with_default_options(&mut context, device, stream_config)
            {
                Ok(stream) => stream,
                Err(error) => {
                    context.destack_audio_device_close(device)?;
                    let code = error.platform_error().map(|platform| platform.code);
                    if code == Some(PlatformErrorCode::NotSupported)
                        || code == Some(PlatformErrorCode::IoInvalidData)
                        || code == Some(PlatformErrorCode::AudioUnavailable)
                        || code == Some(PlatformErrorCode::DeviceUnavailable)
                    {
                        continue;
                    }

                    return Err(error);
                }
            };
            let _ = assert_ok_or_expected_error(
                context.destack_audio_stream_start(stream),
                &[
                    PlatformErrorCode::IoWouldBlock,
                    PlatformErrorCode::IoInterrupted,
                    PlatformErrorCode::IoBusy,
                ],
            )?;

            let descriptor = context.destack_audio_device_descriptor(device)?;
            let supported_domains = device_descriptor_stream_clock_domains_from_value(descriptor);

            for (domain, mask) in domain_rows {
                let is_supported = (supported_domains.0 & mask) != 0;
                let result = context.destack_audio_stream_clock(stream, domain);
                if is_supported {
                    if let Err(error) = result {
                        let code = error.platform_error().map(|platform| platform.code);
                        assert_ne!(
                            code,
                            Some(PlatformErrorCode::NotSupported),
                            "backend {backend:?} advertises clock domain {domain:?} but stream.clock returned notSupported",
                        );
                    }
                } else {
                    assert_platform_error_code(result, PlatformErrorCode::NotSupported)?;
                }
            }

            let _ = assert_ok_or_expected_error(
                context.destack_audio_stream_stop(stream),
                &[
                    PlatformErrorCode::IoWouldBlock,
                    PlatformErrorCode::IoInterrupted,
                    PlatformErrorCode::IoBusy,
                ],
            )?;
            context.destack_audio_stream_close(stream)?;
            context.destack_audio_device_close(device)?;
        }

        Ok(())
    });
}
