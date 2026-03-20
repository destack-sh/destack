use super::super::core::{
    BACKEND_CAPABILITY_BACKEND_DISCONNECT_EVENTS, BACKEND_CAPABILITY_DEVICE_CLOCK,
    BACKEND_CAPABILITY_EXCLUSIVE_MODE, BACKEND_CAPABILITY_LOOPBACK,
    BACKEND_CAPABILITY_NATIVE_EVENT_FEED, BACKEND_CAPABILITY_NON_INTERLEAVED,
    BACKEND_CAPABILITY_SCHEDULED_WRITE, BACKEND_CAPABILITY_SHARED_MODE,
    DEVICE_CAPABILITY_BACKEND_DISCONNECT_EVENTS, DEVICE_CAPABILITY_LOOPBACK,
    DEVICE_CAPABILITY_SCHEDULED_WRITE, DEVICE_LIST_INCLUDE_DISCONNECTED, DEVICE_OPEN_RAW,
    STREAM_FLAG_EXPLICIT_SAMPLE_FORMAT, STREAM_FLAG_MINIMIZE_LATENCY, STREAM_FLAG_NO_AUTO_CONVERT,
    STREAM_FLAG_REPORT_XRUN, SUPPORTED_STREAM_CLOCK_INPUT_ADC, SUPPORTED_STREAM_CLOCK_OUTPUT_DAC,
};
use super::super::{
    AudioBackend, AudioBackendSelectionPolicy, AudioDeviceDirection, AudioDeviceListFlags,
    AudioDeviceListRequest, AudioDeviceOpenFlags, AudioDeviceOpenOptions, AudioShareMode,
};
use super::core::{
    backend_descriptor_summaries, backend_is_available_for_host_execution, backend_support_rows,
    backend_support_rows_with_capabilities, descriptor_count,
    device_descriptor_direction_from_value, device_descriptor_identity_from_value,
    device_descriptor_stream_clock_domains_from_value, device_direction_capability_rows,
    harness_device_options, harness_list_request, harness_string, has_available_host_backend,
    string_from_harness_value,
};
use super::{
    assert_code_is_one_of, assert_not_supported_result, assert_platform_error_code,
    error_code_from_runtime_error, with_harness_context,
};
use crate::platform::core::BackendSupport;
use crate::platform::diagnostic::PlatformErrorCode;

const HOST_AUDIO_LIST_ALLOWED_ERRORS: [PlatformErrorCode; 4] = [
    PlatformErrorCode::IoNotFound,
    PlatformErrorCode::IoPermissionDenied,
    PlatformErrorCode::AudioUnavailable,
    PlatformErrorCode::DeviceUnavailable,
];
const HOST_AUDIO_OPEN_ALLOWED_ERRORS: [PlatformErrorCode; 5] = [
    PlatformErrorCode::IoNotFound,
    PlatformErrorCode::IoPermissionDenied,
    PlatformErrorCode::IoInvalidData,
    PlatformErrorCode::AudioUnavailable,
    PlatformErrorCode::DeviceUnavailable,
];

#[cfg(any(unix, windows))]
#[test]
fn test_audio_null_device_list_contains_rows() {
    with_harness_context(|mut context| {
        let request = AudioDeviceListRequest {
            direction: AudioDeviceDirection::Playback,
            backend: AudioBackend::Null,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            flags: AudioDeviceListFlags(0),
        };

        let list_request = harness_list_request(&mut context, request);
        let rows = context.destack_audio_device_list(list_request)?;
        let row_count = descriptor_count(&mut context, rows)?;
        assert!(
            row_count >= 1,
            "null backend list should expose at least one row"
        );

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_loopback_device_list_contains_only_loopback_capable_rows() {
    with_harness_context(|mut context| {
        let request = AudioDeviceListRequest {
            direction: AudioDeviceDirection::Loopback,
            backend: AudioBackend::Null,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            flags: AudioDeviceListFlags(0),
        };

        let list_request = harness_list_request(&mut context, request);
        let rows = context.destack_audio_device_list(list_request)?;
        let rows = device_direction_capability_rows(&mut context, rows)?;
        assert!(
            !rows.is_empty(),
            "loopback device list should expose at least one loopback row",
        );

        for (direction, capability_flags) in rows {
            assert_eq!(direction, AudioDeviceDirection::Loopback);
            assert_ne!(capability_flags.0 & DEVICE_CAPABILITY_LOOPBACK.0, 0);
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_null_device_scheduled_write_capability_matches_direction() {
    with_harness_context(|mut context| {
        let request = AudioDeviceListRequest {
            direction: AudioDeviceDirection::Capture,
            backend: AudioBackend::Null,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            flags: AudioDeviceListFlags(0),
        };

        let list_request = harness_list_request(&mut context, request);
        let rows = context.destack_audio_device_list(list_request)?;
        let rows = device_direction_capability_rows(&mut context, rows)?;
        assert!(
            !rows.is_empty(),
            "null capture list should expose at least one row",
        );

        for (direction, capability_flags) in rows {
            let supports_scheduled_write =
                (capability_flags.0 & DEVICE_CAPABILITY_SCHEDULED_WRITE.0) != 0;

            if direction == AudioDeviceDirection::Duplex {
                assert!(
                    supports_scheduled_write,
                    "null duplex rows should advertise scheduled write capability",
                );
            } else if direction == AudioDeviceDirection::Capture {
                assert!(
                    !supports_scheduled_write,
                    "null capture rows should not advertise scheduled write capability",
                );
            }
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_backend_list_contains_null_and_available_backend() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_support_rows(&mut context, rows)?;
        assert!(
            rows.iter().any(|(backend, support)| {
                *backend == AudioBackend::Null && *support == BackendSupport::Available
            }),
            "backend list should always expose one available null backend",
        );
        assert!(
            rows.iter()
                .any(|(_backend, support)| *support == BackendSupport::Available),
            "backend list should expose at least one available backend",
        );

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_null_backend_supported_stream_flags_are_explicit() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_descriptor_summaries(&mut context, rows)?;
        let null = rows
            .iter()
            .find(|row| row.backend == AudioBackend::Null)
            .expect("backend list should contain null selector");

        assert_ne!(
            null.supported_stream_flags.0 & STREAM_FLAG_REPORT_XRUN.0,
            0,
            "null backend should advertise xrun reporting support",
        );
        assert_eq!(
            null.supported_stream_flags.0 & STREAM_FLAG_MINIMIZE_LATENCY.0,
            0,
            "null backend should not advertise unimplemented latency hint support",
        );
        assert_ne!(
            null.supported_stream_flags.0 & STREAM_FLAG_EXPLICIT_SAMPLE_FORMAT.0,
            0,
            "null backend should advertise exact sample-format support",
        );

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_backend_list_reports_completed_stream_option_lanes() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_descriptor_summaries(&mut context, rows)?;

        let alsa = rows
            .iter()
            .find(|row| row.backend == AudioBackend::Alsa)
            .expect("backend list should contain alsa selector");
        if alsa.support == BackendSupport::Available {
            assert_ne!(
                alsa.supported_stream_flags.0 & STREAM_FLAG_MINIMIZE_LATENCY.0,
                0,
                "alsa should advertise low-latency stream support",
            );
            assert_ne!(
                alsa.supported_stream_flags.0 & STREAM_FLAG_EXPLICIT_SAMPLE_FORMAT.0,
                0,
                "alsa should advertise explicit sample-format support",
            );
            assert_ne!(
                alsa.supported_stream_flags.0 & STREAM_FLAG_NO_AUTO_CONVERT.0,
                0,
                "alsa should advertise no-auto-convert stream support",
            );
        }

        let aaudio = rows
            .iter()
            .find(|row| row.backend == AudioBackend::AAudio)
            .expect("backend list should contain aaudio selector");
        if aaudio.support == BackendSupport::Available {
            assert_ne!(
                aaudio.supported_stream_flags.0 & STREAM_FLAG_MINIMIZE_LATENCY.0,
                0,
                "aaudio should advertise low-latency stream support",
            );
            assert_ne!(
                aaudio.supported_stream_flags.0 & STREAM_FLAG_EXPLICIT_SAMPLE_FORMAT.0,
                0,
                "aaudio should advertise explicit sample-format support",
            );
        }

        let available_host_row = rows
            .iter()
            .find(|row| {
                row.backend != AudioBackend::Null && row.support == BackendSupport::Available
            })
            .expect("host backend list should expose one available runtime backend");
        assert_ne!(
            available_host_row.supported_stream_flags.0 & STREAM_FLAG_EXPLICIT_SAMPLE_FORMAT.0,
            0,
            "available host backends should advertise explicit sample-format support",
        );

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_backend_list_reports_auto_availability_from_host_backends() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_support_rows(&mut context, rows)?;

        let auto_support = rows
            .iter()
            .find(|(backend, _support)| *backend == AudioBackend::Auto)
            .map(|(_backend, support)| *support)
            .unwrap_or(BackendSupport::UnsupportedTarget);
        let host_available = has_available_host_backend(&rows);

        if host_available {
            assert_eq!(auto_support, BackendSupport::Available);
        } else {
            assert_ne!(auto_support, BackendSupport::Available);
        }
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_backend_list_sets_capabilities_for_available_rows() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_support_rows_with_capabilities(&mut context, rows)?;

        for (backend, support, capability_flags) in rows {
            if support != BackendSupport::Available {
                continue;
            }

            assert_ne!(
                capability_flags.0, 0,
                "available backend {backend:?} should advertise non-empty capabilities"
            );

            if backend == AudioBackend::Null {
                assert_ne!(capability_flags.0 & BACKEND_CAPABILITY_SHARED_MODE.0, 0);
                assert_ne!(capability_flags.0 & BACKEND_CAPABILITY_LOOPBACK.0, 0);
            }

            if backend == AudioBackend::CoreAudio || backend == AudioBackend::Wasapi {
                assert_ne!(capability_flags.0 & BACKEND_CAPABILITY_EXCLUSIVE_MODE.0, 0);
                assert_ne!(capability_flags.0 & BACKEND_CAPABILITY_LOOPBACK.0, 0);
                assert_ne!(capability_flags.0 & BACKEND_CAPABILITY_DEVICE_CLOCK.0, 0);
                assert_ne!(capability_flags.0 & BACKEND_CAPABILITY_SCHEDULED_WRITE.0, 0);
            }

            if backend == AudioBackend::Asio || backend == AudioBackend::Alsa {
                assert_ne!(capability_flags.0 & BACKEND_CAPABILITY_EXCLUSIVE_MODE.0, 0);
            }

            if backend == AudioBackend::AAudio {
                assert_ne!(capability_flags.0 & BACKEND_CAPABILITY_EXCLUSIVE_MODE.0, 0);
            }

            if backend == AudioBackend::OpenSLES {
                assert_ne!(capability_flags.0 & BACKEND_CAPABILITY_SHARED_MODE.0, 0);
                assert_eq!(capability_flags.0 & BACKEND_CAPABILITY_EXCLUSIVE_MODE.0, 0);
            }

            if backend == AudioBackend::Asio {
                assert_eq!(capability_flags.0 & BACKEND_CAPABILITY_SHARED_MODE.0, 0);
                assert_ne!(capability_flags.0 & BACKEND_CAPABILITY_NON_INTERLEAVED.0, 0);
            }

            if backend == AudioBackend::PulseAudio {
                assert_ne!(capability_flags.0 & BACKEND_CAPABILITY_SHARED_MODE.0, 0);
                assert_eq!(capability_flags.0 & BACKEND_CAPABILITY_EXCLUSIVE_MODE.0, 0);
                assert_ne!(capability_flags.0 & BACKEND_CAPABILITY_LOOPBACK.0, 0);
            }

            if backend == AudioBackend::PipeWire {
                assert_ne!(capability_flags.0 & BACKEND_CAPABILITY_SHARED_MODE.0, 0);
                assert_eq!(capability_flags.0 & BACKEND_CAPABILITY_EXCLUSIVE_MODE.0, 0);
                assert_ne!(capability_flags.0 & BACKEND_CAPABILITY_LOOPBACK.0, 0);
            }

            if backend == AudioBackend::Jack {
                assert_ne!(capability_flags.0 & BACKEND_CAPABILITY_SHARED_MODE.0, 0);
                assert_eq!(capability_flags.0 & BACKEND_CAPABILITY_EXCLUSIVE_MODE.0, 0);
            }

            if backend == AudioBackend::CoreAudio
                || backend == AudioBackend::Wasapi
                || backend == AudioBackend::Alsa
                || backend == AudioBackend::PulseAudio
                || backend == AudioBackend::PipeWire
                || backend == AudioBackend::Jack
                || backend == AudioBackend::Asio
            {
                assert_ne!(
                    capability_flags.0 & BACKEND_CAPABILITY_NATIVE_EVENT_FEED.0,
                    0,
                    "backend {backend:?} should advertise native device-event ingress",
                );
            }

            if backend == AudioBackend::Null
                || backend == AudioBackend::AAudio
                || backend == AudioBackend::OpenSLES
            {
                assert_eq!(
                    capability_flags.0 & BACKEND_CAPABILITY_NATIVE_EVENT_FEED.0,
                    0,
                    "backend {backend:?} should not advertise native device-event ingress",
                );
            }
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_backend_list_support_contract_matches_advertised_lanes() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_descriptor_summaries(&mut context, rows)?;
        let auto = rows
            .iter()
            .find(|row| row.backend == AudioBackend::Auto)
            .expect("backend list should contain auto selector");
        let null = rows
            .iter()
            .find(|row| row.backend == AudioBackend::Null)
            .expect("backend list should contain null selector");
        let has_available_host_backend = rows.iter().any(|row| {
            row.backend != AudioBackend::Auto
                && row.backend != AudioBackend::Null
                && row.support == BackendSupport::Available
        });

        assert_eq!(auto.priority, u16::MAX);
        assert_eq!(null.priority, 0);
        assert_eq!(null.support, BackendSupport::Available);

        if has_available_host_backend {
            assert_eq!(auto.support, BackendSupport::Available);
            assert_ne!(auto.capability_flags.0, 0);
            assert_ne!(auto.supported_device_list_flags.0, 0);
        } else {
            assert_ne!(auto.support, BackendSupport::Available);
            assert_eq!(auto.capability_flags.0, 0);
            assert_eq!(auto.supported_device_list_flags.0, 0);
            assert_eq!(auto.supported_device_open_flags.0, 0);
            assert_eq!(auto.supported_stream_flags.0, 0);
            assert_eq!(auto.supported_stream_requirement_flags.0, 0);
            assert_eq!(auto.supported_event_subscription_flags.0, 0);
            assert_eq!(auto.supported_stream_clock_domains.0, 0);
        }

        for row in rows {
            if row.support == BackendSupport::Available {
                continue;
            }

            assert_eq!(
                row.capability_flags.0, 0,
                "unavailable backend {:?} should not advertise capability lanes",
                row.backend
            );
            assert_eq!(
                row.supported_device_list_flags.0, 0,
                "unavailable backend {:?} should not advertise device list lanes",
                row.backend
            );
            assert_eq!(
                row.supported_device_open_flags.0, 0,
                "unavailable backend {:?} should not advertise device open lanes",
                row.backend
            );
            assert_eq!(
                row.supported_stream_flags.0, 0,
                "unavailable backend {:?} should not advertise stream flags",
                row.backend
            );
            assert_eq!(
                row.supported_stream_requirement_flags.0, 0,
                "unavailable backend {:?} should not advertise stream requirements",
                row.backend
            );
            assert_eq!(
                row.supported_event_subscription_flags.0, 0,
                "unavailable backend {:?} should not advertise event lanes",
                row.backend
            );
            assert_eq!(
                row.supported_stream_clock_domains.0, 0,
                "unavailable backend {:?} should not advertise stream clock lanes",
                row.backend
            );
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_backend_disconnect_capability_matches_device_rows() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_support_rows_with_capabilities(&mut context, rows)?;

        for (backend, support, backend_capability_flags) in rows {
            if support != BackendSupport::Available || backend == AudioBackend::Auto {
                continue;
            }

            let backend_disconnect_supported =
                (backend_capability_flags.0 & BACKEND_CAPABILITY_BACKEND_DISCONNECT_EVENTS.0) != 0;
            let mut listed_rows = 0usize;

            for direction in [
                AudioDeviceDirection::Playback,
                AudioDeviceDirection::Capture,
                AudioDeviceDirection::Duplex,
                AudioDeviceDirection::Loopback,
            ] {
                let request = AudioDeviceListRequest {
                    direction,
                    backend,
                    backend_policy: AudioBackendSelectionPolicy::Strict,
                    flags: AudioDeviceListFlags(0),
                };

                let list_request = harness_list_request(&mut context, request);
                let list = match context.destack_audio_device_list(list_request) {
                    Ok(list) => list,
                    Err(error) => {
                        let code = error_code_from_runtime_error(&error);
                        if code == Some(PlatformErrorCode::IoNotFound) {
                            continue;
                        }

                        return Err(error);
                    }
                };
                let rows = device_direction_capability_rows(&mut context, list)?;
                listed_rows = listed_rows.saturating_add(rows.len());

                for (_direction, device_capability_flags) in rows {
                    let device_disconnect_supported = (device_capability_flags.0
                        & DEVICE_CAPABILITY_BACKEND_DISCONNECT_EVENTS.0)
                        != 0;
                    assert_eq!(
                        device_disconnect_supported, backend_disconnect_supported,
                        "backend {backend:?} disconnect capability should match every listed device row",
                    );
                }
            }

            if listed_rows == 0 {
                continue;
            }
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_backend_share_mode_capabilities_match_device_open_behavior() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_support_rows_with_capabilities(&mut context, rows)?;

        for (backend, support, backend_capability_flags) in rows {
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

            for (share_mode, capability_bit) in [
                (AudioShareMode::Shared, BACKEND_CAPABILITY_SHARED_MODE.0),
                (
                    AudioShareMode::Exclusive,
                    BACKEND_CAPABILITY_EXCLUSIVE_MODE.0,
                ),
            ] {
                let options = AudioDeviceOpenOptions {
                    direction: AudioDeviceDirection::Playback,
                    backend,
                    backend_policy: AudioBackendSelectionPolicy::Strict,
                    share_mode,
                    flags: AudioDeviceOpenFlags(0),
                };

                let device_id = harness_string(&mut context, &default_id);
                let options = harness_device_options(&mut context, options);
                let result = context.destack_audio_device_open(device_id, options);
                let mode_supported = (backend_capability_flags.0 & capability_bit) != 0;
                if !mode_supported {
                    assert_not_supported_result(result)?;
                    continue;
                }

                match result {
                    Ok(device) => {
                        context.destack_audio_device_close(device)?;
                    }
                    Err(error) => {
                        let code = error_code_from_runtime_error(&error);
                        assert_code_is_one_of(
                            code,
                            &HOST_AUDIO_OPEN_ALLOWED_ERRORS,
                            &format!(
                                "backend {backend:?} advertised share mode {share_mode:?} but device open failed outside the allowed host error set"
                            ),
                        )?;
                    }
                }
            }
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_available_host_backends_allow_strict_device_listing() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_support_rows(&mut context, rows)?;

        for (backend, support) in rows {
            if support != BackendSupport::Available
                || backend == AudioBackend::Auto
                || backend == AudioBackend::Null
            {
                continue;
            }

            let request = AudioDeviceListRequest {
                direction: AudioDeviceDirection::Playback,
                backend,
                backend_policy: AudioBackendSelectionPolicy::Strict,
                flags: AudioDeviceListFlags(0),
            };

            let list_request = harness_list_request(&mut context, request);
            match context.destack_audio_device_list(list_request) {
                Ok(rows) => {
                    let _ = descriptor_count(&mut context, rows)?;
                }
                Err(error) => {
                    let code = error_code_from_runtime_error(&error);
                    assert_code_is_one_of(
                        code,
                        &HOST_AUDIO_LIST_ALLOWED_ERRORS,
                        &format!(
                            "available backend {backend:?} should either list devices or fail with an explicit host-availability error"
                        ),
                    )?;
                }
            }
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_wasapi_device_list_exposes_loopback_and_duplex_ids_when_available() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_support_rows(&mut context, rows)?;
        let wasapi_available = backend_is_available_for_host_execution(&rows, AudioBackend::Wasapi);
        if !wasapi_available {
            return Ok(());
        }

        let request = AudioDeviceListRequest {
            direction: AudioDeviceDirection::Loopback,
            backend: AudioBackend::Wasapi,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            flags: AudioDeviceListFlags(0),
        };
        let list_request = harness_list_request(&mut context, request);
        let loopback_rows = context.destack_audio_device_list(list_request)?;
        let loopback_rows = device_direction_capability_rows(&mut context, loopback_rows)?;
        if loopback_rows.is_empty() {
            return Ok(());
        }

        for (direction, capability_flags) in loopback_rows {
            assert_eq!(direction, AudioDeviceDirection::Loopback);
            assert_ne!(capability_flags.0 & DEVICE_CAPABILITY_LOOPBACK.0, 0);
        }

        let duplex_id = context.destack_audio_device_default(
            AudioDeviceDirection::Duplex,
            AudioBackend::Wasapi,
            AudioBackendSelectionPolicy::Strict,
        );
        if let Ok(duplex_id) = duplex_id {
            let duplex_id = string_from_harness_value(&mut context, duplex_id)?;
            assert!(
                duplex_id.starts_with("wasapi:duplex:"),
                "wasapi duplex default id should use wasapi:duplex: prefix",
            );
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_host_device_defaults_use_stable_backend_prefixes_when_available() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_support_rows(&mut context, rows)?;

        #[cfg(target_os = "android")]
        let expectations = vec![
            (AudioBackend::CoreAudio, "coreaudio:"),
            (AudioBackend::Asio, "asio:playback:"),
            (AudioBackend::Alsa, "alsa:playback:"),
            (AudioBackend::PipeWire, "pipewire:playback:"),
            (AudioBackend::PulseAudio, "pulseaudio:playback:"),
            (AudioBackend::Jack, "jack:playback:"),
            (AudioBackend::AAudio, "aaudio:playback:"),
            (AudioBackend::OpenSLES, "opensles:playback:"),
        ];

        #[cfg(not(target_os = "android"))]
        let expectations = vec![
            (AudioBackend::CoreAudio, "coreaudio:"),
            (AudioBackend::Asio, "asio:playback:"),
            (AudioBackend::Alsa, "alsa:playback:"),
            (AudioBackend::PipeWire, "pipewire:playback:"),
            (AudioBackend::PulseAudio, "pulseaudio:playback:"),
            (AudioBackend::Jack, "jack:playback:"),
        ];

        for (backend, prefix) in expectations {
            if !backend_is_available_for_host_execution(&rows, backend) {
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
                    assert_eq!(code, Some(PlatformErrorCode::IoNotFound));
                    continue;
                }
            };

            assert!(
                default_id.starts_with(prefix),
                "backend {backend:?} default id should use the stable {prefix} prefix",
            );
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_device_default_returns_stable_null_ids() {
    with_harness_context(|mut context| {
        let playback = context.destack_audio_device_default(
            AudioDeviceDirection::Playback,
            AudioBackend::Null,
            AudioBackendSelectionPolicy::Strict,
        )?;
        assert_eq!(
            string_from_harness_value(&mut context, playback)?,
            "audio:null:playback",
        );

        let capture = context.destack_audio_device_default(
            AudioDeviceDirection::Capture,
            AudioBackend::Null,
            AudioBackendSelectionPolicy::Strict,
        )?;
        assert_eq!(
            string_from_harness_value(&mut context, capture)?,
            "audio:null:capture",
        );

        let duplex = context.destack_audio_device_default(
            AudioDeviceDirection::Duplex,
            AudioBackend::Null,
            AudioBackendSelectionPolicy::Strict,
        )?;
        assert_eq!(
            string_from_harness_value(&mut context, duplex)?,
            "audio:null:duplex",
        );

        let loopback = context.destack_audio_device_default(
            AudioDeviceDirection::Loopback,
            AudioBackend::Null,
            AudioBackendSelectionPolicy::Strict,
        )?;
        assert_eq!(
            string_from_harness_value(&mut context, loopback)?,
            "audio:null:loopback",
        );

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_device_rescan_rejects_unsupported_backend() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_support_rows(&mut context, rows)?;

        let unsupported_backend = rows
            .into_iter()
            .find(|(backend, support)| {
                *support != BackendSupport::Available
                    && *backend != AudioBackend::Auto
                    && *backend != AudioBackend::Null
            })
            .map(|(backend, _support)| backend);
        let Some(unsupported_backend) = unsupported_backend else {
            return Ok(());
        };

        assert_not_supported_result(context.destack_audio_device_rescan(
            unsupported_backend,
            AudioBackendSelectionPolicy::Strict,
        ))?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_device_rescan_allows_backend_fallback() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_support_rows(&mut context, rows)?;
        let has_host_backend = has_available_host_backend(&rows);

        let result = context.destack_audio_device_rescan(
            AudioBackend::Asio,
            AudioBackendSelectionPolicy::AllowFallback,
        );
        if has_host_backend {
            result?;
        } else {
            assert_not_supported_result(result)?;
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_device_open_rejects_exclusive_mode() {
    with_harness_context(|mut context| {
        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Playback,
            backend: AudioBackend::Null,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Exclusive,
            flags: AudioDeviceOpenFlags(0),
        };

        let device_id = harness_string(&mut context, "audio:null:playback");
        let options = harness_device_options(&mut context, options);
        assert_not_supported_result(context.destack_audio_device_open(device_id, options))?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_device_open_rejects_raw_flag_for_non_wasapi_backend() {
    with_harness_context(|mut context| {
        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Playback,
            backend: AudioBackend::Null,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(DEVICE_OPEN_RAW.0),
        };

        let device_id = harness_string(&mut context, "audio:null:playback");
        let options = harness_device_options(&mut context, options);
        assert_not_supported_result(context.destack_audio_device_open(device_id, options))?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_device_open_alsa_no_resample_matches_backend_support() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_support_rows(&mut context, rows)?;
        let alsa_available = backend_is_available_for_host_execution(&rows, AudioBackend::Alsa);
        if !alsa_available {
            return Ok(());
        }

        let default_id = match context.destack_audio_device_default(
            AudioDeviceDirection::Playback,
            AudioBackend::Alsa,
            AudioBackendSelectionPolicy::Strict,
        ) {
            Ok(value) => string_from_harness_value(&mut context, value)?,
            Err(error) => {
                let code = error_code_from_runtime_error(&error);
                assert_eq!(code, Some(PlatformErrorCode::IoNotFound));
                return Ok(());
            }
        };

        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Playback,
            backend: AudioBackend::Alsa,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };

        let device_id = harness_string(&mut context, &default_id);
        let options = harness_device_options(&mut context, options);
        match context.destack_audio_device_open(device_id, options) {
            Ok(device) => {
                context.destack_audio_device_close(device)?;
            }
            Err(error) => {
                let code = error_code_from_runtime_error(&error);
                assert_code_is_one_of(
                    code,
                    &HOST_AUDIO_OPEN_ALLOWED_ERRORS,
                    "alsa no-resample flag should fail only with the allowed host error set",
                )?;
            }
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_device_open_jack_no_autoconnect_matches_backend_support() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_support_rows(&mut context, rows)?;
        let jack_available = backend_is_available_for_host_execution(&rows, AudioBackend::Jack);
        if !jack_available {
            return Ok(());
        }

        let default_id = match context.destack_audio_device_default(
            AudioDeviceDirection::Playback,
            AudioBackend::Jack,
            AudioBackendSelectionPolicy::Strict,
        ) {
            Ok(value) => string_from_harness_value(&mut context, value)?,
            Err(error) => {
                let code = error_code_from_runtime_error(&error);
                assert_eq!(code, Some(PlatformErrorCode::IoNotFound));
                return Ok(());
            }
        };

        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Playback,
            backend: AudioBackend::Jack,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };

        let device_id = harness_string(&mut context, &default_id);
        let options = harness_device_options(&mut context, options);
        match context.destack_audio_device_open(device_id, options) {
            Ok(device) => {
                context.destack_audio_device_close(device)?;
            }
            Err(error) => {
                let code = error_code_from_runtime_error(&error);
                assert_code_is_one_of(
                    code,
                    &HOST_AUDIO_OPEN_ALLOWED_ERRORS,
                    "jack no-autoconnect flag should fail only with the allowed host error set",
                )?;
            }
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_device_open_require_hardware_timestamps_matches_backend_capability() {
    with_harness_context(|mut context| {
        let backend_list = context.destack_audio_backend_list()?;
        let backend_rows = backend_support_rows_with_capabilities(&mut context, backend_list)?;

        for (backend, support, capability_flags) in backend_rows {
            if support != BackendSupport::Available || backend == AudioBackend::Auto {
                continue;
            }

            let requires_device_clock =
                (capability_flags.0 & BACKEND_CAPABILITY_DEVICE_CLOCK.0) != 0;
            if !requires_device_clock {
                let options = AudioDeviceOpenOptions {
                    direction: AudioDeviceDirection::Playback,
                    backend,
                    backend_policy: AudioBackendSelectionPolicy::Strict,
                    share_mode: AudioShareMode::Shared,
                    flags: AudioDeviceOpenFlags(0),
                };

                let device_id = harness_string(&mut context, "audio:null:playback");
                let options = harness_device_options(&mut context, options);
                assert_not_supported_result(context.destack_audio_device_open(device_id, options))?;
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
            match context.destack_audio_device_open(device_id, options) {
                Ok(device) => {
                    context.destack_audio_device_close(device)?;
                }
                Err(error) => {
                    let code = error_code_from_runtime_error(&error);
                    assert_code_is_one_of(
                        code,
                        &HOST_AUDIO_OPEN_ALLOWED_ERRORS,
                        &format!(
                            "backend {backend:?} advertises device clock but requireHardwareTimestamps failed outside the allowed host error set"
                        ),
                    )?;
                }
            }
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_device_list_accepts_known_request_flags() {
    with_harness_context(|mut context| {
        let request = AudioDeviceListRequest {
            direction: AudioDeviceDirection::Playback,
            backend: AudioBackend::Null,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            flags: AudioDeviceListFlags(DEVICE_LIST_INCLUDE_DISCONNECTED.0),
        };

        let list_request = harness_list_request(&mut context, request);
        let rows = context.destack_audio_device_list(list_request)?;
        let row_count = descriptor_count(&mut context, rows)?;
        assert!(
            row_count >= 1,
            "known list flags should keep null device listing functional",
        );

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_device_open_rejects_unknown_open_flag_bits() {
    with_harness_context(|mut context| {
        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Playback,
            backend: AudioBackend::Null,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0x8000_0000),
        };

        let device_id = harness_string(&mut context, "audio:null:playback");
        let options = harness_device_options(&mut context, options);
        assert_platform_error_code(
            context.destack_audio_device_open(device_id, options),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_device_open_require_loopback_matches_backend_capability() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_support_rows_with_capabilities(&mut context, rows)?;

        for (backend, support, capability_flags) in rows {
            if support != BackendSupport::Available || backend == AudioBackend::Auto {
                continue;
            }

            let backend_supports_loopback =
                (capability_flags.0 & BACKEND_CAPABILITY_LOOPBACK.0) != 0;
            if backend_supports_loopback {
                let default_loopback = match context.destack_audio_device_default(
                    AudioDeviceDirection::Loopback,
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

                let options = AudioDeviceOpenOptions {
                    direction: AudioDeviceDirection::Loopback,
                    backend,
                    backend_policy: AudioBackendSelectionPolicy::Strict,
                    share_mode: AudioShareMode::Shared,
                    flags: AudioDeviceOpenFlags(0),
                };

                let device_id = harness_string(&mut context, &default_loopback);
                let options = harness_device_options(&mut context, options);
                match context.destack_audio_device_open(device_id, options) {
                    Ok(device) => {
                        context.destack_audio_device_close(device)?;
                    }
                    Err(error) => {
                        let code = error_code_from_runtime_error(&error);
                        assert_code_is_one_of(
                            code,
                            &HOST_AUDIO_OPEN_ALLOWED_ERRORS,
                            &format!(
                                "backend {backend:?} advertises loopback but requireLoopback failed outside the allowed host error set"
                            ),
                        )?;
                    }
                }

                continue;
            }

            let options = AudioDeviceOpenOptions {
                direction: AudioDeviceDirection::Loopback,
                backend,
                backend_policy: AudioBackendSelectionPolicy::Strict,
                share_mode: AudioShareMode::Shared,
                flags: AudioDeviceOpenFlags(0),
            };

            let device_id = harness_string(&mut context, "audio:null:playback");
            let options = harness_device_options(&mut context, options);
            assert_not_supported_result(context.destack_audio_device_open(device_id, options))?;
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_device_open_rejects_loopback_direction_without_loopback_capability() {
    with_harness_context(|mut context| {
        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Loopback,
            backend: AudioBackend::Null,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };

        let device_id = harness_string(&mut context, "audio:null:duplex");
        let options = harness_device_options(&mut context, options);
        assert_not_supported_result(context.destack_audio_device_open(device_id, options))?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_device_descriptor_reports_opened_direction() {
    with_harness_context(|mut context| {
        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Capture,
            backend: AudioBackend::Null,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };

        let device_id = harness_string(&mut context, "audio:null:duplex");
        let options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, options)?;

        let descriptor = context.destack_audio_device_descriptor(device)?;
        let descriptor_direction = device_descriptor_direction_from_value(descriptor);
        assert_eq!(descriptor_direction, AudioDeviceDirection::Capture);

        context.destack_audio_device_close(device)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_coreaudio_opened_device_descriptor_uses_stable_identity_prefixes() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_support_rows(&mut context, rows)?;
        let coreaudio_available =
            backend_is_available_for_host_execution(&rows, AudioBackend::CoreAudio);
        if !coreaudio_available {
            return Ok(());
        }

        let default_id = match context.destack_audio_device_default(
            AudioDeviceDirection::Playback,
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
            direction: AudioDeviceDirection::Playback,
            backend: AudioBackend::CoreAudio,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };

        let device_id = harness_string(&mut context, &default_id);
        let options = harness_device_options(&mut context, options);
        let device = match context.destack_audio_device_open(device_id, options) {
            Ok(device) => device,
            Err(error) => {
                let code = error_code_from_runtime_error(&error);
                if code == Some(PlatformErrorCode::AudioUnavailable)
                    || code == Some(PlatformErrorCode::DeviceUnavailable)
                    || code == Some(PlatformErrorCode::IoPermissionDenied)
                {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let descriptor = context.destack_audio_device_descriptor(device)?;
        let (device_id, group_id) =
            device_descriptor_identity_from_value(&mut context, descriptor)?;

        // coreaudio descriptors should expose stable backend-prefixed identities
        assert!(
            device_id.starts_with("coreaudio:"),
            "coreaudio device ids should use the stable coreaudio prefix",
        );
        assert!(
            group_id.starts_with("coreaudio-group:"),
            "coreaudio group ids should use the stable coreaudio group prefix",
        );

        context.destack_audio_device_close(device)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_device_descriptor_stream_clock_domains_follow_opened_direction() {
    with_harness_context(|mut context| {
        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Capture,
            backend: AudioBackend::Null,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
        };

        let device_id = harness_string(&mut context, "audio:null:duplex");
        let options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, options)?;

        let descriptor = context.destack_audio_device_descriptor(device)?;
        let supported_domains = device_descriptor_stream_clock_domains_from_value(descriptor);
        let supports_input_adc = (supported_domains.0 & SUPPORTED_STREAM_CLOCK_INPUT_ADC.0) != 0;
        let supports_output_dac = (supported_domains.0 & SUPPORTED_STREAM_CLOCK_OUTPUT_DAC.0) != 0;
        assert!(
            supports_input_adc,
            "capture-open descriptor should advertise input adc clock support",
        );
        assert!(
            !supports_output_dac,
            "capture-open descriptor should not advertise output dac clock support",
        );

        context.destack_audio_device_close(device)?;
        Ok(())
    });
}
