use super::super::core::{
    BACKEND_CAPABILITY_EXCLUSIVE_MODE, BACKEND_CAPABILITY_LOOPBACK, BACKEND_CAPABILITY_SHARED_MODE,
    DEVICE_CAPABILITY_LOOPBACK,
};
use super::super::{
    AudioBackend, AudioBackendOpenFlags, AudioBackendSelectionPolicy, AudioDeviceDirection,
    AudioDeviceListFlags, AudioDeviceListRequest, AudioDeviceOpenFlags, AudioDeviceOpenOptions,
    AudioShareMode,
};
use super::core::{
    backend_availability_rows, backend_availability_rows_with_capabilities, descriptor_count,
    device_descriptor_direction_from_value, device_direction_capability_rows,
    harness_device_options, harness_list_request, harness_string, string_from_harness_value,
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

            if backend == AudioBackend::CoreAudio {
                assert_ne!(capability_flags.0 & BACKEND_CAPABILITY_EXCLUSIVE_MODE.0, 0);
                assert_ne!(capability_flags.0 & BACKEND_CAPABILITY_LOOPBACK.0, 0);
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
        assert_platform_error_code(
            context.destack_audio_device_rescan(
                AudioBackend::Asio,
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
            backend_flags: AudioBackendOpenFlags(0),
            backend_hint: context.call_context.store_string(""),
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
fn test_audio_device_open_rejects_backend_specific_tuning_without_support() {
    with_harness_context(|mut context| {
        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Playback,
            backend: AudioBackend::Null,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
            backend_flags: AudioBackendOpenFlags(0x1),
            backend_hint: context.call_context.store_string("driver-a"),
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
fn test_audio_device_open_rejects_loopback_direction_without_loopback_capability() {
    with_harness_context(|mut context| {
        let options = AudioDeviceOpenOptions {
            direction: AudioDeviceDirection::Loopback,
            backend: AudioBackend::Null,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            share_mode: AudioShareMode::Shared,
            flags: AudioDeviceOpenFlags(0),
            backend_flags: AudioBackendOpenFlags(0),
            backend_hint: context.call_context.store_string(""),
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
            backend_flags: AudioBackendOpenFlags(0),
            backend_hint: context.call_context.store_string(""),
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
