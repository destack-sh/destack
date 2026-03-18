use crate::diagnostic::{RuntimeResult, RuntimeStatus};
use crate::host::core::error::invalid_argument_value;
use crate::host::windows::{
    WindowsApplicationLifecycle, windows_notify_application_lifecycle,
    windows_notify_intent_custom_action, windows_notify_intent_open_file,
    windows_notify_intent_open_url, windows_notify_intent_share_files,
    windows_notify_intent_share_text, windows_notify_interruption_changed,
    windows_notify_location_sample, windows_notify_memory_pressure_changed,
    windows_notify_permission_result, windows_notify_power_mode_changed,
    windows_notify_thermal_state_changed, windows_notify_wake, windows_notify_wall_clock_changed,
};
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};
use crate::platform::os::abi_generated::LocationSampleValue;
use crate::runtime::{NativeStringRef, NativeStringSlice};

pub(crate) const WINDOWS_LIFECYCLE_CREATED: u32 = 0;
pub(crate) const WINDOWS_LIFECYCLE_ACTIVATED: u32 = 1;
pub(crate) const WINDOWS_LIFECYCLE_RESUMED: u32 = 2;
pub(crate) const WINDOWS_LIFECYCLE_SUSPENDED: u32 = 3;
pub(crate) const WINDOWS_LIFECYCLE_STOPPING: u32 = 4;
pub(crate) const WINDOWS_LIFECYCLE_DESTROYED: u32 = 5;
pub(crate) const WINDOWS_MEMORY_PRESSURE_NORMAL: u32 = 0;
pub(crate) const WINDOWS_MEMORY_PRESSURE_WARNING: u32 = 1;
pub(crate) const WINDOWS_MEMORY_PRESSURE_CRITICAL: u32 = 2;
pub(crate) const WINDOWS_THERMAL_NOMINAL: u32 = 0;
pub(crate) const WINDOWS_THERMAL_FAIR: u32 = 1;
pub(crate) const WINDOWS_THERMAL_SERIOUS: u32 = 2;
pub(crate) const WINDOWS_THERMAL_CRITICAL: u32 = 3;
pub(crate) const WINDOWS_POWER_MODE_NORMAL: u32 = 0;
pub(crate) const WINDOWS_POWER_MODE_LOW_POWER: u32 = 1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_windows_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle_code: u32,
) -> RuntimeStatus {
    let result = decode_windows_application_lifecycle(lifecycle_code)
        .and_then(|lifecycle| windows_notify_application_lifecycle(runtime_id, lifecycle));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_windows_notify_permission_result(
    runtime_id: u64,
    permission: NativeStringRef,
    granted: bool,
) -> RuntimeStatus {
    let result = decode_permission_name(permission).and_then(|permission| {
        windows_notify_permission_result(runtime_id, permission.as_str(), granted)
    });

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_windows_notify_location_sample(
    runtime_id: u64,
    watch_id: NativeStringRef,
    sample: LocationSampleValue,
) -> RuntimeStatus {
    let result = decode_string(watch_id, "watch_id")
        .and_then(|watch_id| windows_notify_location_sample(runtime_id, &watch_id, sample));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_windows_notify_intent_open_url(
    runtime_id: u64,
    has_source: bool,
    source: NativeStringRef,
    url: NativeStringRef,
) -> RuntimeStatus {
    let result = decode_optional_string(has_source, source, "source").and_then(|source| {
        decode_string(url, "url")
            .and_then(|url| windows_notify_intent_open_url(runtime_id, source.as_deref(), &url))
    });

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_windows_notify_intent_open_file(
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
                windows_notify_intent_open_file(
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
pub unsafe extern "C" fn destack_host_windows_notify_intent_share_text(
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
                windows_notify_intent_share_text(
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
pub unsafe extern "C" fn destack_host_windows_notify_intent_share_files(
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
                windows_notify_intent_share_files(
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
pub unsafe extern "C" fn destack_host_windows_notify_intent_custom_action(
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
                                windows_notify_intent_custom_action(
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
pub unsafe extern "C" fn destack_host_windows_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeStatus {
    runtime_status(windows_notify_interruption_changed(runtime_id, interrupted))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_windows_notify_memory_pressure_changed(
    runtime_id: u64,
    level_code: u32,
) -> RuntimeStatus {
    let result = decode_windows_memory_pressure_level(level_code)
        .and_then(|level| windows_notify_memory_pressure_changed(runtime_id, level));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_windows_notify_thermal_state_changed(
    runtime_id: u64,
    thermal_code: u32,
) -> RuntimeStatus {
    let result = decode_windows_thermal_state(thermal_code)
        .and_then(|state| windows_notify_thermal_state_changed(runtime_id, state));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_windows_notify_power_mode_changed(
    runtime_id: u64,
    power_mode_code: u32,
) -> RuntimeStatus {
    let result = decode_windows_power_mode(power_mode_code)
        .and_then(|mode| windows_notify_power_mode_changed(runtime_id, mode));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_windows_notify_wall_clock_changed(
    runtime_id: u64,
) -> RuntimeStatus {
    runtime_status(windows_notify_wall_clock_changed(runtime_id))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_windows_notify_wake(runtime_id: u64) -> RuntimeStatus {
    runtime_status(windows_notify_wake(runtime_id))
}

pub(crate) fn decode_windows_application_lifecycle(
    lifecycle_code: u32,
) -> RuntimeResult<WindowsApplicationLifecycle> {
    let lifecycle = match lifecycle_code {
        WINDOWS_LIFECYCLE_CREATED => WindowsApplicationLifecycle::Created,
        WINDOWS_LIFECYCLE_ACTIVATED => WindowsApplicationLifecycle::Activated,
        WINDOWS_LIFECYCLE_RESUMED => WindowsApplicationLifecycle::Resumed,
        WINDOWS_LIFECYCLE_SUSPENDED => WindowsApplicationLifecycle::Suspended,
        WINDOWS_LIFECYCLE_STOPPING => WindowsApplicationLifecycle::Stopping,
        WINDOWS_LIFECYCLE_DESTROYED => WindowsApplicationLifecycle::Destroyed,
        _ => {
            return Err(invalid_argument_value(
                "lifecycle_code",
                "invalid windows lifecycle code",
            ));
        }
    };

    Ok(lifecycle)
}

pub(crate) fn decode_windows_memory_pressure_level(
    level_code: u32,
) -> RuntimeResult<HostMemoryPressureLevel> {
    let level = match level_code {
        WINDOWS_MEMORY_PRESSURE_NORMAL => HostMemoryPressureLevel::Normal,
        WINDOWS_MEMORY_PRESSURE_WARNING => HostMemoryPressureLevel::Warning,
        WINDOWS_MEMORY_PRESSURE_CRITICAL => HostMemoryPressureLevel::Critical,
        _ => {
            return Err(invalid_argument_value(
                "level_code",
                "invalid windows memory pressure level code",
            ));
        }
    };

    Ok(level)
}

pub(crate) fn decode_windows_thermal_state(thermal_code: u32) -> RuntimeResult<HostThermalState> {
    let state = match thermal_code {
        WINDOWS_THERMAL_NOMINAL => HostThermalState::Nominal,
        WINDOWS_THERMAL_FAIR => HostThermalState::Fair,
        WINDOWS_THERMAL_SERIOUS => HostThermalState::Serious,
        WINDOWS_THERMAL_CRITICAL => HostThermalState::Critical,
        _ => {
            return Err(invalid_argument_value(
                "thermal_code",
                "invalid windows thermal state code",
            ));
        }
    };

    Ok(state)
}

pub(crate) fn decode_windows_power_mode(power_mode_code: u32) -> RuntimeResult<HostPowerMode> {
    let mode = match power_mode_code {
        WINDOWS_POWER_MODE_NORMAL => HostPowerMode::Normal,
        WINDOWS_POWER_MODE_LOW_POWER => HostPowerMode::LowPower,
        _ => {
            return Err(invalid_argument_value(
                "power_mode_code",
                "invalid windows power mode code",
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
    field: &'static str,
) -> RuntimeResult<Option<String>> {
    if !is_present {
        return Ok(None);
    }

    decode_string(value, field).map(Some)
}

fn decode_string(value: NativeStringRef, field: &'static str) -> RuntimeResult<String> {
    unsafe { value.as_str() }
        .map(|value| value.to_string())
        .map_err(|_| invalid_argument_value(field, format!("invalid utf8 {field} payload")))
}

fn decode_string_slice(
    values: NativeStringSlice,
    field: &'static str,
) -> RuntimeResult<Vec<String>> {
    let values = unsafe { values.as_slice() }
        .map_err(|_| invalid_argument_value(field, format!("invalid {field} slice payload")))?;
    let mut decoded = Vec::with_capacity(values.len());

    for value in values {
        let value = decode_string(*value, field)?;
        decoded.push(value);
    }

    Ok(decoded)
}

fn runtime_status(result: RuntimeResult<()>) -> RuntimeStatus {
    RuntimeStatus::from_result(result, None)
}
