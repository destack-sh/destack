use super::super::{
    AudioBackend, AudioBackendOpenFlags, AudioBackendSelectionPolicy, AudioChannelLayout,
    AudioDeviceDirection, AudioDeviceOpenFlags, AudioDeviceOpenOptions, AudioSampleFormat,
    AudioShareMode, AudioStreamConfig, AudioStreamFlags, AudioStreamStateKind,
    AudioStreamTransferMode,
};
use super::core::{
    backend_availability_rows, byte_len, harness_bytes, harness_bytes_slices,
    harness_device_options, harness_mutable_bytes_slices, harness_stream_config, harness_string,
    open_null_duplex_stream, stream_snapshot_support_from_value, stream_state_from_value,
    string_from_harness_value,
};
use super::{assert_ok_or_expected_error, assert_platform_error_code, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;

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
            backend_flags: AudioBackendOpenFlags(0),
            backend_hint: context.call_context.store_string(""),
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
            flags: AudioStreamFlags(0),
        };
        let invalid_config = harness_stream_config(&mut context, invalid_config);
        assert_platform_error_code(
            context.destack_audio_stream_open(device, invalid_config),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        context.destack_audio_device_close(device)?;
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
            backend_flags: AudioBackendOpenFlags(0),
            backend_hint: context.call_context.store_string(""),
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
            flags: AudioStreamFlags(0),
        };
        let stream_config = harness_stream_config(&mut context, stream_config);
        let stream = context.destack_audio_stream_open(device, stream_config)?;
        context.destack_audio_stream_start(stream)?;

        let payload = harness_bytes(&mut context, &[0u8; 32])?;
        assert_platform_error_code(
            context.destack_audio_stream_try_write(stream, payload),
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
fn test_audio_host_stream_open_close_when_backend_is_available() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_list = backend_availability_rows(&mut context, backend_list)?;
        let backend = backend_list
            .into_iter()
            .find(|(backend, available)| {
                *available && *backend != AudioBackend::Auto && *backend != AudioBackend::Null
            })
            .map(|(backend, _available)| backend);
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
                let code = error.platform_error().map(|platform| platform.code);
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
            backend_flags: AudioBackendOpenFlags(0),
            backend_hint: context.call_context.store_string(""),
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
            flags: AudioStreamFlags(0),
        };
        let config = harness_stream_config(&mut context, config);
        let stream = context.destack_audio_stream_open(device, config)?;
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
fn test_audio_host_stream_write_at_reports_not_supported_when_backend_lacks_schedule_lane() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_list = backend_availability_rows(&mut context, backend_list)?;
        let backend = backend_list
            .into_iter()
            .find(|(backend, available)| {
                *available && *backend != AudioBackend::Auto && *backend != AudioBackend::Null
            })
            .map(|(backend, _available)| backend);
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
                let code = error.platform_error().map(|platform| platform.code);
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
            backend_flags: AudioBackendOpenFlags(0),
            backend_hint: context.call_context.store_string(""),
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
            flags: AudioStreamFlags(0),
        };
        let config = harness_stream_config(&mut context, config);
        let stream = context.destack_audio_stream_open(device, config)?;
        context.destack_audio_stream_start(stream)?;

        let snapshot = context.destack_audio_stream_snapshot(stream)?;
        let (supports_write_at, _supports_pause, _supports_volume, _supports_mute) =
            stream_snapshot_support_from_value(snapshot);
        let payload = harness_bytes(&mut context, &[0u8; 32])?;
        let write_at_result = context.destack_audio_stream_write_at(stream, payload, 0);
        if supports_write_at {
            let _ =
                assert_ok_or_expected_error(write_at_result, &[PlatformErrorCode::IoWouldBlock])?;
        } else {
            assert_platform_error_code(write_at_result, PlatformErrorCode::NotSupported)?;
        }

        context.destack_audio_stream_stop(stream)?;
        context.destack_audio_stream_close(stream)?;
        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_coreaudio_loopback_open_start_stop_when_available() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_list = backend_availability_rows(&mut context, backend_list)?;
        let coreaudio_available = backend_list
            .iter()
            .any(|(backend, available)| *backend == AudioBackend::CoreAudio && *available);
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
                let code = error.platform_error().map(|platform| platform.code);
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
            backend_flags: AudioBackendOpenFlags(0),
            backend_hint: context.call_context.store_string(""),
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
            flags: AudioStreamFlags(0),
        };
        let config = harness_stream_config(&mut context, config);
        let stream = context.destack_audio_stream_open(device, config)?;
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
fn test_audio_null_stream_snapshot_reports_expected_support_flags() {
    with_harness_context(|mut context| {
        let (device, stream) = open_null_duplex_stream(&mut context)?;

        let snapshot = context.destack_audio_stream_snapshot(stream)?;
        let (supports_write_at, supports_pause, supports_volume, supports_mute) =
            stream_snapshot_support_from_value(snapshot);
        assert!(
            !supports_write_at,
            "null stream should not advertise scheduled writes without host timeline scheduling",
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
