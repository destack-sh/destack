use crate::diagnostic::{RuntimeResult, RuntimeStatus};
use crate::host::core::error::invalid_argument_value;
use crate::host::macos::{
    MacosApplicationLifecycle, macos_notify_application_lifecycle,
    macos_notify_intent_custom_action, macos_notify_intent_open_file, macos_notify_intent_open_url,
    macos_notify_intent_share_files, macos_notify_intent_share_text,
    macos_notify_interruption_changed, macos_notify_memory_pressure_changed,
    macos_notify_permission_result, macos_notify_power_mode_changed,
    macos_notify_thermal_state_changed, macos_notify_wake, macos_notify_wall_clock_changed,
};
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};
use crate::runtime::{NativeStringRef, NativeStringSlice};

/// macOS lifecycle code for `applicationDidFinishLaunching`.
pub(super) const MACOS_LIFECYCLE_DID_FINISH_LAUNCHING: u32 = 0;
/// macOS lifecycle code for `applicationDidBecomeActive`.
pub(super) const MACOS_LIFECYCLE_DID_BECOME_ACTIVE: u32 = 1;
/// macOS lifecycle code for `applicationWillResignActive`.
pub(super) const MACOS_LIFECYCLE_WILL_RESIGN_ACTIVE: u32 = 2;
/// macOS lifecycle code for `applicationWillTerminate`.
pub(super) const MACOS_LIFECYCLE_WILL_TERMINATE: u32 = 3;
/// macOS memory pressure code for normal state.
pub(super) const MACOS_MEMORY_PRESSURE_NORMAL: u32 = 0;
/// macOS memory pressure code for warning state.
pub(super) const MACOS_MEMORY_PRESSURE_WARNING: u32 = 1;
/// macOS memory pressure code for critical state.
pub(super) const MACOS_MEMORY_PRESSURE_CRITICAL: u32 = 2;
/// macOS thermal code for nominal state.
pub(super) const MACOS_THERMAL_NOMINAL: u32 = 0;
/// macOS thermal code for fair state.
pub(super) const MACOS_THERMAL_FAIR: u32 = 1;
/// macOS thermal code for serious state.
pub(super) const MACOS_THERMAL_SERIOUS: u32 = 2;
/// macOS thermal code for critical state.
pub(super) const MACOS_THERMAL_CRITICAL: u32 = 3;
/// macOS power mode code for normal state.
pub(super) const MACOS_POWER_MODE_NORMAL: u32 = 0;
/// macOS power mode code for low power state.
pub(super) const MACOS_POWER_MODE_LOW_POWER: u32 = 1;

/// Notify the runtime host about one macOS application lifecycle transition.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle_code: u32,
) -> RuntimeStatus {
    let result = decode_macos_application_lifecycle(lifecycle_code)
        .and_then(|lifecycle| macos_notify_application_lifecycle(runtime_id, lifecycle));

    runtime_status(result)
}

/// Notify the runtime host with one macOS permission result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_permission_result(
    runtime_id: u64,
    permission: NativeStringRef,
    granted: bool,
) -> RuntimeStatus {
    let result = decode_permission_name(permission).and_then(|permission| {
        macos_notify_permission_result(runtime_id, permission.as_str(), granted)
    });

    runtime_status(result)
}

/// Notify the runtime host with one macOS open-url intent event.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_intent_open_url(
    runtime_id: u64,
    has_source: bool,
    source: NativeStringRef,
    url: NativeStringRef,
) -> RuntimeStatus {
    let result = decode_optional_string(has_source, source, "source").and_then(|source| {
        decode_string(url, "url")
            .and_then(|url| macos_notify_intent_open_url(runtime_id, source.as_deref(), &url))
    });

    runtime_status(result)
}

/// Notify the runtime host with one macOS open-file intent event.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_intent_open_file(
    runtime_id: u64,
    has_source: bool,
    source: NativeStringRef,
    path: NativeStringRef,
    has_mime_type: bool,
    mime_type: NativeStringRef,
) -> RuntimeStatus {
    let result = decode_optional_string(has_source, source, "source").and_then(|source| {
        decode_string(path, "path").and_then(|path| {
            decode_optional_string(has_mime_type, mime_type, "mime_type").and_then(|mime_type| {
                macos_notify_intent_open_file(
                    runtime_id,
                    source.as_deref(),
                    &path,
                    mime_type.as_deref(),
                )
            })
        })
    });

    runtime_status(result)
}

/// Notify the runtime host with one macOS share-text intent event.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_intent_share_text(
    runtime_id: u64,
    has_source: bool,
    source: NativeStringRef,
    text: NativeStringRef,
    has_mime_type: bool,
    mime_type: NativeStringRef,
) -> RuntimeStatus {
    let result = decode_optional_string(has_source, source, "source").and_then(|source| {
        decode_string(text, "text").and_then(|text| {
            decode_optional_string(has_mime_type, mime_type, "mime_type").and_then(|mime_type| {
                macos_notify_intent_share_text(
                    runtime_id,
                    source.as_deref(),
                    &text,
                    mime_type.as_deref(),
                )
            })
        })
    });

    runtime_status(result)
}

/// Notify the runtime host with one macOS share-files intent event.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_intent_share_files(
    runtime_id: u64,
    has_source: bool,
    source: NativeStringRef,
    paths: NativeStringSlice,
    has_mime_type: bool,
    mime_type: NativeStringRef,
) -> RuntimeStatus {
    let result = decode_optional_string(has_source, source, "source").and_then(|source| {
        decode_string_slice(paths, "paths").and_then(|paths| {
            decode_optional_string(has_mime_type, mime_type, "mime_type").and_then(|mime_type| {
                macos_notify_intent_share_files(
                    runtime_id,
                    source.as_deref(),
                    &paths,
                    mime_type.as_deref(),
                )
            })
        })
    });

    runtime_status(result)
}

/// Notify the runtime host with one macOS custom-action intent event.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_intent_custom_action(
    runtime_id: u64,
    has_source: bool,
    source: NativeStringRef,
    action: NativeStringRef,
    has_url: bool,
    url: NativeStringRef,
    paths: NativeStringSlice,
    has_text: bool,
    text: NativeStringRef,
    has_mime_type: bool,
    mime_type: NativeStringRef,
) -> RuntimeStatus {
    let result = decode_optional_string(has_source, source, "source").and_then(|source| {
        decode_string(action, "action").and_then(|action| {
            decode_optional_string(has_url, url, "url").and_then(|url| {
                decode_string_slice(paths, "paths").and_then(|paths| {
                    decode_optional_string(has_text, text, "text").and_then(|text| {
                        decode_optional_string(has_mime_type, mime_type, "mime_type").and_then(
                            |mime_type| {
                                macos_notify_intent_custom_action(
                                    runtime_id,
                                    source.as_deref(),
                                    &action,
                                    url.as_deref(),
                                    &paths,
                                    text.as_deref(),
                                    mime_type.as_deref(),
                                )
                            },
                        )
                    })
                })
            })
        })
    });

    runtime_status(result)
}

/// Notify the runtime host that interruption state changed on macOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeStatus {
    runtime_status(macos_notify_interruption_changed(runtime_id, interrupted))
}

/// Notify the runtime host that memory pressure changed on macOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_memory_pressure_changed(
    runtime_id: u64,
    level_code: u32,
) -> RuntimeStatus {
    let result = decode_macos_memory_pressure_level(level_code)
        .and_then(|level| macos_notify_memory_pressure_changed(runtime_id, level));

    runtime_status(result)
}

/// Notify the runtime host that thermal state changed on macOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_thermal_state_changed(
    runtime_id: u64,
    thermal_code: u32,
) -> RuntimeStatus {
    let result = decode_macos_thermal_state(thermal_code)
        .and_then(|state| macos_notify_thermal_state_changed(runtime_id, state));

    runtime_status(result)
}

/// Notify the runtime host that power mode changed on macOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_power_mode_changed(
    runtime_id: u64,
    power_mode_code: u32,
) -> RuntimeStatus {
    let result = decode_macos_power_mode(power_mode_code)
        .and_then(|mode| macos_notify_power_mode_changed(runtime_id, mode));

    runtime_status(result)
}

/// Notify the runtime host that wall clock changed on macOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_wall_clock_changed(
    runtime_id: u64,
) -> RuntimeStatus {
    runtime_status(macos_notify_wall_clock_changed(runtime_id))
}

/// Wake one blocked host event poll operation for macOS.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_macos_notify_wake(runtime_id: u64) -> RuntimeStatus {
    runtime_status(macos_notify_wake(runtime_id))
}

// lifecycle decode
pub(super) fn decode_macos_application_lifecycle(
    lifecycle_code: u32,
) -> RuntimeResult<MacosApplicationLifecycle> {
    let lifecycle = match lifecycle_code {
        MACOS_LIFECYCLE_DID_FINISH_LAUNCHING => MacosApplicationLifecycle::DidFinishLaunching,
        MACOS_LIFECYCLE_DID_BECOME_ACTIVE => MacosApplicationLifecycle::DidBecomeActive,
        MACOS_LIFECYCLE_WILL_RESIGN_ACTIVE => MacosApplicationLifecycle::WillResignActive,
        MACOS_LIFECYCLE_WILL_TERMINATE => MacosApplicationLifecycle::WillTerminate,
        _ => {
            return Err(invalid_argument_value(
                "lifecycle_code",
                "invalid macos lifecycle code",
            ));
        }
    };

    Ok(lifecycle)
}

// memory pressure decode
pub(super) fn decode_macos_memory_pressure_level(
    level_code: u32,
) -> RuntimeResult<HostMemoryPressureLevel> {
    let level = match level_code {
        MACOS_MEMORY_PRESSURE_NORMAL => HostMemoryPressureLevel::Normal,
        MACOS_MEMORY_PRESSURE_WARNING => HostMemoryPressureLevel::Warning,
        MACOS_MEMORY_PRESSURE_CRITICAL => HostMemoryPressureLevel::Critical,
        _ => {
            return Err(invalid_argument_value(
                "level_code",
                "invalid macos memory pressure level code",
            ));
        }
    };

    Ok(level)
}

// thermal decode
pub(super) fn decode_macos_thermal_state(thermal_code: u32) -> RuntimeResult<HostThermalState> {
    let state = match thermal_code {
        MACOS_THERMAL_NOMINAL => HostThermalState::Nominal,
        MACOS_THERMAL_FAIR => HostThermalState::Fair,
        MACOS_THERMAL_SERIOUS => HostThermalState::Serious,
        MACOS_THERMAL_CRITICAL => HostThermalState::Critical,
        _ => {
            return Err(invalid_argument_value(
                "thermal_code",
                "invalid macos thermal state code",
            ));
        }
    };

    Ok(state)
}

// power mode decode
pub(super) fn decode_macos_power_mode(power_mode_code: u32) -> RuntimeResult<HostPowerMode> {
    let mode = match power_mode_code {
        MACOS_POWER_MODE_NORMAL => HostPowerMode::Normal,
        MACOS_POWER_MODE_LOW_POWER => HostPowerMode::LowPower,
        _ => {
            return Err(invalid_argument_value(
                "power_mode_code",
                "invalid macos power mode code",
            ));
        }
    };

    Ok(mode)
}

// permission decode
fn decode_permission_name(permission: NativeStringRef) -> RuntimeResult<String> {
    decode_string(permission, "permission")
}

/// Decode one optional host string.
fn decode_optional_string(
    is_present: bool,
    value: NativeStringRef,
    argument: &'static str,
) -> RuntimeResult<Option<String>> {
    if !is_present {
        return Ok(None);
    }

    decode_string(value, argument).map(Some)
}

/// Decode one required host string.
fn decode_string(value: NativeStringRef, argument: &'static str) -> RuntimeResult<String> {
    unsafe { value.as_str() }
        .map(str::to_string)
        .map_err(|_| invalid_argument_value(argument, format!("invalid {argument} string")))
}

/// Decode one host string slice.
fn decode_string_slice(
    values: NativeStringSlice,
    argument: &'static str,
) -> RuntimeResult<Vec<String>> {
    let mut decoded_values = Vec::new();

    for value in unsafe { values.as_slice() }
        .map_err(|_| invalid_argument_value(argument, format!("invalid {argument} slice")))?
    {
        decoded_values.push(decode_string(*value, argument)?);
    }

    Ok(decoded_values)
}

// ffi result conversion
fn runtime_status(result: RuntimeResult<()>) -> RuntimeStatus {
    RuntimeStatus::from_result(result, None)
}
