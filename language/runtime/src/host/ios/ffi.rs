use crate::diagnostic::{RuntimeResult, RuntimeStatus};
use crate::host::core::error::invalid_argument_value;
use crate::host::ios::{
    IosApplicationLifecycle, ios_notify_application_lifecycle, ios_notify_intent_custom_action,
    ios_notify_intent_open_file, ios_notify_intent_open_url, ios_notify_intent_share_files,
    ios_notify_intent_share_text, ios_notify_interruption_changed,
    ios_notify_memory_pressure_changed, ios_notify_permission_result,
    ios_notify_power_mode_changed, ios_notify_thermal_state_changed, ios_notify_wake,
    ios_notify_wall_clock_changed,
};
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};
use crate::runtime::{NativeStringRef, NativeStringSlice};

/// iOS lifecycle code for `applicationDidFinishLaunching`.
pub(super) const IOS_LIFECYCLE_DID_FINISH_LAUNCHING: u32 = 0;
/// iOS lifecycle code for `applicationDidBecomeActive`.
pub(super) const IOS_LIFECYCLE_DID_BECOME_ACTIVE: u32 = 1;
/// iOS lifecycle code for `applicationWillResignActive`.
pub(super) const IOS_LIFECYCLE_WILL_RESIGN_ACTIVE: u32 = 2;
/// iOS lifecycle code for `applicationDidEnterBackground`.
pub(super) const IOS_LIFECYCLE_DID_ENTER_BACKGROUND: u32 = 3;
/// iOS lifecycle code for `applicationWillEnterForeground`.
pub(super) const IOS_LIFECYCLE_WILL_ENTER_FOREGROUND: u32 = 4;
/// iOS lifecycle code for `applicationWillTerminate`.
pub(super) const IOS_LIFECYCLE_WILL_TERMINATE: u32 = 5;
/// iOS memory pressure code for normal state.
pub(super) const IOS_MEMORY_PRESSURE_NORMAL: u32 = 0;
/// iOS memory pressure code for warning state.
pub(super) const IOS_MEMORY_PRESSURE_WARNING: u32 = 1;
/// iOS memory pressure code for critical state.
pub(super) const IOS_MEMORY_PRESSURE_CRITICAL: u32 = 2;
/// iOS thermal code for nominal state.
pub(super) const IOS_THERMAL_NOMINAL: u32 = 0;
/// iOS thermal code for fair state.
pub(super) const IOS_THERMAL_FAIR: u32 = 1;
/// iOS thermal code for serious state.
pub(super) const IOS_THERMAL_SERIOUS: u32 = 2;
/// iOS thermal code for critical state.
pub(super) const IOS_THERMAL_CRITICAL: u32 = 3;
/// iOS power mode code for normal state.
pub(super) const IOS_POWER_MODE_NORMAL: u32 = 0;
/// iOS power mode code for low power state.
pub(super) const IOS_POWER_MODE_LOW_POWER: u32 = 1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle_code: u32,
) -> RuntimeStatus {
    let result = decode_ios_application_lifecycle(lifecycle_code)
        .and_then(|lifecycle| ios_notify_application_lifecycle(runtime_id, lifecycle));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_permission_result(
    runtime_id: u64,
    permission: NativeStringRef,
    granted: bool,
) -> RuntimeStatus {
    let result = decode_permission_name(permission).and_then(|permission| {
        ios_notify_permission_result(runtime_id, permission.as_str(), granted)
    });

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_intent_open_url(
    runtime_id: u64,
    has_source: bool,
    source: NativeStringRef,
    url: NativeStringRef,
) -> RuntimeStatus {
    let result = decode_optional_string(has_source, source, "source").and_then(|source| {
        decode_string(url, "url")
            .and_then(|url| ios_notify_intent_open_url(runtime_id, source.as_deref(), &url))
    });

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_intent_open_file(
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
                ios_notify_intent_open_file(
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_intent_share_text(
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
                ios_notify_intent_share_text(
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_intent_share_files(
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
                ios_notify_intent_share_files(
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_intent_custom_action(
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
                                ios_notify_intent_custom_action(
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeStatus {
    runtime_status(ios_notify_interruption_changed(runtime_id, interrupted))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_memory_pressure_changed(
    runtime_id: u64,
    level_code: u32,
) -> RuntimeStatus {
    let result = decode_ios_memory_pressure_level(level_code)
        .and_then(|level| ios_notify_memory_pressure_changed(runtime_id, level));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_thermal_state_changed(
    runtime_id: u64,
    thermal_code: u32,
) -> RuntimeStatus {
    let result = decode_ios_thermal_state(thermal_code)
        .and_then(|state| ios_notify_thermal_state_changed(runtime_id, state));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_power_mode_changed(
    runtime_id: u64,
    power_mode_code: u32,
) -> RuntimeStatus {
    let result = decode_ios_power_mode(power_mode_code)
        .and_then(|mode| ios_notify_power_mode_changed(runtime_id, mode));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_wall_clock_changed(
    runtime_id: u64,
) -> RuntimeStatus {
    runtime_status(ios_notify_wall_clock_changed(runtime_id))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_wake(runtime_id: u64) -> RuntimeStatus {
    runtime_status(ios_notify_wake(runtime_id))
}

pub(super) fn decode_ios_application_lifecycle(
    lifecycle_code: u32,
) -> RuntimeResult<IosApplicationLifecycle> {
    let lifecycle = match lifecycle_code {
        IOS_LIFECYCLE_DID_FINISH_LAUNCHING => IosApplicationLifecycle::DidFinishLaunching,
        IOS_LIFECYCLE_DID_BECOME_ACTIVE => IosApplicationLifecycle::DidBecomeActive,
        IOS_LIFECYCLE_WILL_RESIGN_ACTIVE => IosApplicationLifecycle::WillResignActive,
        IOS_LIFECYCLE_DID_ENTER_BACKGROUND => IosApplicationLifecycle::DidEnterBackground,
        IOS_LIFECYCLE_WILL_ENTER_FOREGROUND => IosApplicationLifecycle::WillEnterForeground,
        IOS_LIFECYCLE_WILL_TERMINATE => IosApplicationLifecycle::WillTerminate,
        _ => {
            return Err(invalid_argument_value(
                "lifecycle_code",
                "invalid ios lifecycle code",
            ));
        }
    };

    Ok(lifecycle)
}

pub(super) fn decode_ios_memory_pressure_level(
    level_code: u32,
) -> RuntimeResult<HostMemoryPressureLevel> {
    let level = match level_code {
        IOS_MEMORY_PRESSURE_NORMAL => HostMemoryPressureLevel::Normal,
        IOS_MEMORY_PRESSURE_WARNING => HostMemoryPressureLevel::Warning,
        IOS_MEMORY_PRESSURE_CRITICAL => HostMemoryPressureLevel::Critical,
        _ => {
            return Err(invalid_argument_value(
                "level_code",
                "invalid ios memory pressure level code",
            ));
        }
    };

    Ok(level)
}

pub(super) fn decode_ios_thermal_state(thermal_code: u32) -> RuntimeResult<HostThermalState> {
    let state = match thermal_code {
        IOS_THERMAL_NOMINAL => HostThermalState::Nominal,
        IOS_THERMAL_FAIR => HostThermalState::Fair,
        IOS_THERMAL_SERIOUS => HostThermalState::Serious,
        IOS_THERMAL_CRITICAL => HostThermalState::Critical,
        _ => {
            return Err(invalid_argument_value(
                "thermal_code",
                "invalid ios thermal state code",
            ));
        }
    };

    Ok(state)
}

pub(super) fn decode_ios_power_mode(power_mode_code: u32) -> RuntimeResult<HostPowerMode> {
    let mode = match power_mode_code {
        IOS_POWER_MODE_NORMAL => HostPowerMode::Normal,
        IOS_POWER_MODE_LOW_POWER => HostPowerMode::LowPower,
        _ => {
            return Err(invalid_argument_value(
                "power_mode_code",
                "invalid ios power mode code",
            ));
        }
    };

    Ok(mode)
}

fn decode_permission_name(permission: NativeStringRef) -> RuntimeResult<String> {
    decode_string(permission, "permission")
}

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

fn decode_string(value: NativeStringRef, argument: &'static str) -> RuntimeResult<String> {
    unsafe { value.as_str() }
        .map(str::to_string)
        .map_err(|_| invalid_argument_value(argument, format!("invalid {argument} string")))
}

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

fn runtime_status(result: RuntimeResult<()>) -> RuntimeStatus {
    RuntimeStatus::from_result(result, None)
}
