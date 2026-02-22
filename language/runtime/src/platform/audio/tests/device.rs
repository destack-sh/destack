use super::super::core::{
    BACKEND_CAPABILITY_BACKEND_DISCONNECT_EVENTS, BACKEND_CAPABILITY_DEVICE_CLOCK,
    BACKEND_CAPABILITY_EXCLUSIVE_MODE, BACKEND_CAPABILITY_LOOPBACK,
    BACKEND_CAPABILITY_NON_INTERLEAVED, BACKEND_CAPABILITY_SCHEDULED_WRITE,
    BACKEND_CAPABILITY_SHARED_MODE, DEVICE_CAPABILITY_BACKEND_DISCONNECT_EVENTS,
    DEVICE_CAPABILITY_LOOPBACK, DEVICE_CAPABILITY_SCHEDULED_WRITE,
    DEVICE_LIST_INCLUDE_DISCONNECTED, DEVICE_OPEN_FOLLOW_DEFAULT_ROUTE, DEVICE_OPEN_LOW_LATENCY,
    DEVICE_OPEN_RAW, DEVICE_OPEN_REALTIME_THREAD, SUPPORTED_STREAM_CLOCK_INPUT_ADC,
    SUPPORTED_STREAM_CLOCK_OUTPUT_DAC,
};
use super::super::{
    AudioBackend, AudioBackendSelectionPolicy, AudioDeviceDirection, AudioDeviceListFlags,
    AudioDeviceListRequest, AudioDeviceOpenFlags, AudioDeviceOpenOptions, AudioShareMode,
};
use super::core::{
    backend_availability_rows, backend_availability_rows_with_capabilities, descriptor_count,
    device_descriptor_direction_from_value, device_descriptor_stream_clock_domains_from_value,
    device_direction_capability_rows, harness_device_options, harness_list_request, harness_string,
    string_from_harness_value,
};
use super::{assert_platform_error_code, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;

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
        let rows = backend_availability_rows(&mut context, rows)?;
        assert!(
            rows.iter()
                .any(|(backend, available)| *backend == AudioBackend::Null && *available),
            "backend list should always expose one available null backend",
        );
        assert!(
            rows.iter().any(|(_backend, available)| *available),
            "backend list should expose at least one available backend",
        );

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_backend_list_reports_auto_availability_from_host_backends() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_availability_rows(&mut context, rows)?;

        let auto_available = rows
            .iter()
            .find(|(backend, _available)| *backend == AudioBackend::Auto)
            .map(|(_backend, available)| *available)
            .unwrap_or(false);
        let host_available = rows.iter().any(|(backend, available)| {
            *available && *backend != AudioBackend::Auto && *backend != AudioBackend::Null
        });

        assert_eq!(auto_available, host_available);
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_backend_list_sets_capabilities_for_available_rows() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_availability_rows_with_capabilities(&mut context, rows)?;

        for (backend, available, capability_flags) in rows {
            if !available {
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
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_backend_disconnect_capability_matches_device_rows() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_availability_rows_with_capabilities(&mut context, rows)?;

        for (backend, available, backend_capability_flags) in rows {
            if !available || backend == AudioBackend::Auto {
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
                        let code = error.platform_error().map(|platform| platform.code);
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
        let rows = backend_availability_rows_with_capabilities(&mut context, rows)?;

        for (backend, available, backend_capability_flags) in rows {
            if !available || backend == AudioBackend::Auto || backend == AudioBackend::Null {
                continue;
            }

            let default_id = match context.destack_audio_device_default(
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
                    assert_platform_error_code(result, PlatformErrorCode::NotSupported)?;
                    continue;
                }

                match result {
                    Ok(device) => {
                        context.destack_audio_device_close(device)?;
                    }
                    Err(error) => {
                        let code = error.platform_error().map(|platform| platform.code);
                        assert_ne!(
                            code,
                            Some(PlatformErrorCode::NotSupported),
                            "backend {backend:?} advertised share mode {share_mode:?} but device open returned notSupported",
                        );
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
        let rows = backend_availability_rows(&mut context, rows)?;

        for (backend, available) in rows {
            if !available || backend == AudioBackend::Auto || backend == AudioBackend::Null {
                continue;
            }

            let request = AudioDeviceListRequest {
                direction: AudioDeviceDirection::Playback,
                backend,
                backend_policy: AudioBackendSelectionPolicy::Strict,
                flags: AudioDeviceListFlags(0),
            };

            let list_request = harness_list_request(&mut context, request);
            let result = context.destack_audio_device_list(list_request);
            if let Err(error) = result {
                let code = error.platform_error().map(|platform| platform.code);
                assert_ne!(
                    code,
                    Some(PlatformErrorCode::NotSupported),
                    "available backend {backend:?} should not fail strict listing with notSupported"
                );
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
        let rows = backend_availability_rows(&mut context, rows)?;
        let wasapi_available = rows
            .iter()
            .any(|(backend, available)| *backend == AudioBackend::Wasapi && *available);
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
fn test_audio_asio_device_default_uses_stable_prefix_when_available() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_availability_rows(&mut context, rows)?;
        let asio_available = rows
            .iter()
            .any(|(backend, available)| *backend == AudioBackend::Asio && *available);
        if !asio_available {
            return Ok(());
        }

        let playback = context.destack_audio_device_default(
            AudioDeviceDirection::Playback,
            AudioBackend::Asio,
            AudioBackendSelectionPolicy::Strict,
        );
        let playback = match playback {
            Ok(playback) => playback,
            Err(error) => {
                let code = error.platform_error().map(|platform| platform.code);
                assert_eq!(code, Some(PlatformErrorCode::IoNotFound));
                return Ok(());
            }
        };
        let playback = string_from_harness_value(&mut context, playback)?;
        assert!(
            playback.starts_with("asio:playback:"),
            "asio playback default id should use asio:playback: prefix",
        );

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_alsa_device_default_uses_stable_prefix_when_available() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_availability_rows(&mut context, rows)?;
        let alsa_available = rows
            .iter()
            .any(|(backend, available)| *backend == AudioBackend::Alsa && *available);
        if !alsa_available {
            return Ok(());
        }

        let playback = context.destack_audio_device_default(
            AudioDeviceDirection::Playback,
            AudioBackend::Alsa,
            AudioBackendSelectionPolicy::Strict,
        );
        let playback = match playback {
            Ok(playback) => playback,
            Err(error) => {
                let code = error.platform_error().map(|platform| platform.code);
                assert_eq!(code, Some(PlatformErrorCode::IoNotFound));
                return Ok(());
            }
        };
        let playback = string_from_harness_value(&mut context, playback)?;
        assert!(
            playback.starts_with("alsa:playback:"),
            "alsa playback default id should use alsa:playback: prefix",
        );

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_pipewire_device_default_uses_stable_prefix_when_available() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_availability_rows(&mut context, rows)?;
        let pipewire_available = rows
            .iter()
            .any(|(backend, available)| *backend == AudioBackend::PipeWire && *available);
        if !pipewire_available {
            return Ok(());
        }

        let playback = context.destack_audio_device_default(
            AudioDeviceDirection::Playback,
            AudioBackend::PipeWire,
            AudioBackendSelectionPolicy::Strict,
        );
        let playback = match playback {
            Ok(playback) => playback,
            Err(error) => {
                let code = error.platform_error().map(|platform| platform.code);
                assert_eq!(code, Some(PlatformErrorCode::IoNotFound));
                return Ok(());
            }
        };
        let playback = string_from_harness_value(&mut context, playback)?;
        assert!(
            playback.starts_with("pipewire:playback:"),
            "pipewire playback default id should use pipewire:playback: prefix",
        );

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_pulseaudio_device_default_uses_stable_prefix_when_available() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_availability_rows(&mut context, rows)?;
        let pulseaudio_available = rows
            .iter()
            .any(|(backend, available)| *backend == AudioBackend::PulseAudio && *available);
        if !pulseaudio_available {
            return Ok(());
        }

        let playback = context.destack_audio_device_default(
            AudioDeviceDirection::Playback,
            AudioBackend::PulseAudio,
            AudioBackendSelectionPolicy::Strict,
        );
        let playback = match playback {
            Ok(playback) => playback,
            Err(error) => {
                let code = error.platform_error().map(|platform| platform.code);
                assert_eq!(code, Some(PlatformErrorCode::IoNotFound));
                return Ok(());
            }
        };
        let playback = string_from_harness_value(&mut context, playback)?;
        assert!(
            playback.starts_with("pulseaudio:playback:"),
            "pulseaudio playback default id should use pulseaudio:playback: prefix",
        );

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_jack_device_default_uses_stable_prefix_when_available() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_availability_rows(&mut context, rows)?;
        let jack_available = rows
            .iter()
            .any(|(backend, available)| *backend == AudioBackend::Jack && *available);
        if !jack_available {
            return Ok(());
        }

        let playback = context.destack_audio_device_default(
            AudioDeviceDirection::Playback,
            AudioBackend::Jack,
            AudioBackendSelectionPolicy::Strict,
        );
        let playback = match playback {
            Ok(playback) => playback,
            Err(error) => {
                let code = error.platform_error().map(|platform| platform.code);
                assert_eq!(code, Some(PlatformErrorCode::IoNotFound));
                return Ok(());
            }
        };
        let playback = string_from_harness_value(&mut context, playback)?;
        assert!(
            playback.starts_with("jack:playback:"),
            "jack playback default id should use jack:playback: prefix",
        );

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
        let rows = backend_availability_rows(&mut context, rows)?;

        let unsupported_backend = rows
            .into_iter()
            .find(|(backend, available)| {
                !available && *backend != AudioBackend::Auto && *backend != AudioBackend::Null
            })
            .map(|(backend, _available)| backend);
        let Some(unsupported_backend) = unsupported_backend else {
            return Ok(());
        };

        assert_platform_error_code(
            context.destack_audio_device_rescan(
                unsupported_backend,
                AudioBackendSelectionPolicy::Strict,
            ),
            PlatformErrorCode::NotSupported,
        )?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_device_rescan_allows_backend_fallback() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_availability_rows(&mut context, rows)?;
        let has_host_backend = rows.iter().any(|(backend, available)| {
            *available && *backend != AudioBackend::Auto && *backend != AudioBackend::Null
        });

        let result = context.destack_audio_device_rescan(
            AudioBackend::Asio,
            AudioBackendSelectionPolicy::AllowFallback,
        );
        if has_host_backend {
            result?;
        } else {
            assert_platform_error_code(result, PlatformErrorCode::NotSupported)?;
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
        assert_platform_error_code(
            context.destack_audio_device_open(device_id, options),
            PlatformErrorCode::NotSupported,
        )?;

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
        assert_platform_error_code(
            context.destack_audio_device_open(device_id, options),
            PlatformErrorCode::NotSupported,
        )?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_audio_device_open_alsa_no_resample_matches_backend_support() {
    with_harness_context(|mut context| {
        let rows = context.destack_audio_backend_list()?;
        let rows = backend_availability_rows(&mut context, rows)?;
        let alsa_available = rows
            .iter()
            .any(|(backend, available)| *backend == AudioBackend::Alsa && *available);
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
                let code = error.platform_error().map(|platform| platform.code);
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
                let code = error.platform_error().map(|platform| platform.code);
                assert_ne!(
                    code,
                    Some(PlatformErrorCode::NotSupported),
                    "alsa no-resample flag should not route through notSupported on ALSA",
                );
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
        let rows = backend_availability_rows(&mut context, rows)?;
        let jack_available = rows
            .iter()
            .any(|(backend, available)| *backend == AudioBackend::Jack && *available);
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
                let code = error.platform_error().map(|platform| platform.code);
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
                let code = error.platform_error().map(|platform| platform.code);
                assert_ne!(
                    code,
                    Some(PlatformErrorCode::NotSupported),
                    "jack no-autoconnect flag should not route through notSupported on JACK",
                );
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
        let backend_rows = backend_availability_rows_with_capabilities(&mut context, backend_list)?;

        for (backend, available, capability_flags) in backend_rows {
            if !available || backend == AudioBackend::Auto {
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
                assert_platform_error_code(
                    context.destack_audio_device_open(device_id, options),
                    PlatformErrorCode::NotSupported,
                )?;
                continue;
            }

            let default_id = match context.destack_audio_device_default(
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
                    let code = error.platform_error().map(|platform| platform.code);
                    assert_ne!(
                        code,
                        Some(PlatformErrorCode::NotSupported),
                        "backend {backend:?} advertises device clock but requireHardwareTimestamps returned notSupported",
                    );
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
fn test_audio_device_open_accepts_known_open_flags() {
    with_harness_context(|mut context| {
        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Playback,
            backend: AudioBackend::Null,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(
                DEVICE_OPEN_FOLLOW_DEFAULT_ROUTE.0
                    | DEVICE_OPEN_LOW_LATENCY.0
                    | DEVICE_OPEN_REALTIME_THREAD.0,
            ),
        };

        let device_id = harness_string(&mut context, "audio:null:playback");
        let options = harness_device_options(&mut context, options);
        let device = context.destack_audio_device_open(device_id, options)?;
        context.destack_audio_device_close(device)?;

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
        let rows = backend_availability_rows_with_capabilities(&mut context, rows)?;

        for (backend, available, capability_flags) in rows {
            if !available || backend == AudioBackend::Auto {
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
                        let code = error.platform_error().map(|platform| platform.code);
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
                        let code = error.platform_error().map(|platform| platform.code);
                        assert_ne!(
                            code,
                            Some(PlatformErrorCode::NotSupported),
                            "backend {backend:?} advertises loopback but requireLoopback returned notSupported",
                        );
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
            assert_platform_error_code(
                context.destack_audio_device_open(device_id, options),
                PlatformErrorCode::NotSupported,
            )?;
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
        assert_platform_error_code(
            context.destack_audio_device_open(device_id, options),
            PlatformErrorCode::NotSupported,
        )?;

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
