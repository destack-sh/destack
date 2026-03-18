use crate::diagnostic::{RuntimeResult, RuntimeStatus};
use crate::host::android::{
    AndroidActivityLifecycle, android_notify_activity_lifecycle, android_notify_background_event,
    android_notify_intent_custom_action, android_notify_intent_open_file,
    android_notify_intent_open_url, android_notify_intent_share_files,
    android_notify_intent_share_text, android_notify_interruption_changed,
    android_notify_location_sample, android_notify_media_event,
    android_notify_memory_pressure_changed, android_notify_notification_event,
    android_notify_permission_result, android_notify_power_mode_changed,
    android_notify_thermal_state_changed, android_notify_wake, android_notify_wall_clock_changed,
};
use crate::host::callback::decode_callback_host_json;
use crate::host::core::error::invalid_argument_value;
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};
use crate::platform::os::{
    BackgroundEventValue, LocationSampleValue, MediaEventValue, NotificationEventValue,
};
use crate::runtime::{NativeSlice, NativeStringRef, NativeStringSlice};

pub(super) const ANDROID_LIFECYCLE_CREATED: u32 = 0;
pub(super) const ANDROID_LIFECYCLE_STARTED: u32 = 1;
pub(super) const ANDROID_LIFECYCLE_RESUMED: u32 = 2;
pub(super) const ANDROID_LIFECYCLE_PAUSED: u32 = 3;
pub(super) const ANDROID_LIFECYCLE_STOPPED: u32 = 4;
pub(super) const ANDROID_LIFECYCLE_DESTROYED: u32 = 5;
pub(super) const ANDROID_MEMORY_PRESSURE_NORMAL: u32 = 0;
pub(super) const ANDROID_MEMORY_PRESSURE_WARNING: u32 = 1;
pub(super) const ANDROID_MEMORY_PRESSURE_CRITICAL: u32 = 2;
pub(super) const ANDROID_THERMAL_NOMINAL: u32 = 0;
pub(super) const ANDROID_THERMAL_FAIR: u32 = 1;
pub(super) const ANDROID_THERMAL_SERIOUS: u32 = 2;
pub(super) const ANDROID_THERMAL_CRITICAL: u32 = 3;
pub(super) const ANDROID_POWER_MODE_NORMAL: u32 = 0;
pub(super) const ANDROID_POWER_MODE_LOW_POWER: u32 = 1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_notify_activity_lifecycle(
    runtime_id: u64,
    lifecycle_code: u32,
) -> RuntimeStatus {
    let result = decode_android_activity_lifecycle(lifecycle_code)
        .and_then(|lifecycle| android_notify_activity_lifecycle(runtime_id, lifecycle));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_notify_permission_result(
    runtime_id: u64,
    permission: NativeStringRef,
    granted: bool,
) -> RuntimeStatus {
    let result = decode_permission_name(permission).and_then(|permission| {
        android_notify_permission_result(runtime_id, permission.as_str(), granted)
    });

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_notify_intent_open_url(
    runtime_id: u64,
    has_source: bool,
    source: NativeStringRef,
    url: NativeStringRef,
) -> RuntimeStatus {
    let result = decode_optional_string(has_source, source, "source").and_then(|source| {
        decode_string(url, "url")
            .and_then(|url| android_notify_intent_open_url(runtime_id, source.as_deref(), &url))
    });

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_notify_intent_open_file(
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
                android_notify_intent_open_file(
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
pub unsafe extern "C" fn destack_host_android_notify_intent_share_text(
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
                android_notify_intent_share_text(
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
pub unsafe extern "C" fn destack_host_android_notify_intent_share_files(
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
                android_notify_intent_share_files(
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
pub unsafe extern "C" fn destack_host_android_notify_intent_custom_action(
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
                                android_notify_intent_custom_action(
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
pub unsafe extern "C" fn destack_host_android_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeStatus {
    runtime_status(android_notify_interruption_changed(runtime_id, interrupted))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_notify_notification_event(
    runtime_id: u64,
    payload: NativeSlice<u8>,
) -> RuntimeStatus {
    let result =
        decode_notification_event_payload(payload, "destack.host.android.notifyNotificationEvent")
            .and_then(|event| android_notify_notification_event(runtime_id, event));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_notify_background_event(
    runtime_id: u64,
    payload: NativeSlice<u8>,
) -> RuntimeStatus {
    let result =
        decode_background_event_payload(payload, "destack.host.android.notifyBackgroundEvent")
            .and_then(|event| android_notify_background_event(runtime_id, event));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_notify_location_sample(
    runtime_id: u64,
    watch_id: NativeStringRef,
    sample: *const LocationSampleValue,
) -> RuntimeStatus {
    let result = decode_string(watch_id, "watch_id").and_then(|watch_id| {
        if sample.is_null() {
            return Err(invalid_argument_value(
                "sample",
                "sample pointer must not be null",
            ));
        }

        let sample = unsafe { *sample };

        android_notify_location_sample(runtime_id, &watch_id, sample)
    });

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_notify_media_event(
    runtime_id: u64,
    watch_id: NativeStringRef,
    payload: NativeSlice<u8>,
) -> RuntimeStatus {
    let result = decode_string(watch_id, "watch_id").and_then(|watch_id| {
        decode_media_event_payload(payload, "destack.host.android.notifyMediaEvent")
            .and_then(|event| android_notify_media_event(runtime_id, &watch_id, event))
    });

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_notify_memory_pressure_changed(
    runtime_id: u64,
    level_code: u32,
) -> RuntimeStatus {
    let result = decode_android_memory_pressure_level(level_code)
        .and_then(|level| android_notify_memory_pressure_changed(runtime_id, level));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_notify_thermal_state_changed(
    runtime_id: u64,
    thermal_code: u32,
) -> RuntimeStatus {
    let result = decode_android_thermal_state(thermal_code)
        .and_then(|state| android_notify_thermal_state_changed(runtime_id, state));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_notify_power_mode_changed(
    runtime_id: u64,
    power_mode_code: u32,
) -> RuntimeStatus {
    let result = decode_android_power_mode(power_mode_code)
        .and_then(|mode| android_notify_power_mode_changed(runtime_id, mode));

    runtime_status(result)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_notify_wall_clock_changed(
    runtime_id: u64,
) -> RuntimeStatus {
    runtime_status(android_notify_wall_clock_changed(runtime_id))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_notify_wake(runtime_id: u64) -> RuntimeStatus {
    runtime_status(android_notify_wake(runtime_id))
}

pub(super) fn decode_android_activity_lifecycle(
    lifecycle_code: u32,
) -> RuntimeResult<AndroidActivityLifecycle> {
    let lifecycle = match lifecycle_code {
        ANDROID_LIFECYCLE_CREATED => AndroidActivityLifecycle::Created,
        ANDROID_LIFECYCLE_STARTED => AndroidActivityLifecycle::Started,
        ANDROID_LIFECYCLE_RESUMED => AndroidActivityLifecycle::Resumed,
        ANDROID_LIFECYCLE_PAUSED => AndroidActivityLifecycle::Paused,
        ANDROID_LIFECYCLE_STOPPED => AndroidActivityLifecycle::Stopped,
        ANDROID_LIFECYCLE_DESTROYED => AndroidActivityLifecycle::Destroyed,
        _ => {
            return Err(invalid_argument_value(
                "lifecycle_code",
                "invalid android lifecycle code",
            ));
        }
    };

    Ok(lifecycle)
}

pub(super) fn decode_android_memory_pressure_level(
    level_code: u32,
) -> RuntimeResult<HostMemoryPressureLevel> {
    let level = match level_code {
        ANDROID_MEMORY_PRESSURE_NORMAL => HostMemoryPressureLevel::Normal,
        ANDROID_MEMORY_PRESSURE_WARNING => HostMemoryPressureLevel::Warning,
        ANDROID_MEMORY_PRESSURE_CRITICAL => HostMemoryPressureLevel::Critical,
        _ => {
            return Err(invalid_argument_value(
                "level_code",
                "invalid android memory pressure level code",
            ));
        }
    };

    Ok(level)
}

pub(super) fn decode_android_thermal_state(thermal_code: u32) -> RuntimeResult<HostThermalState> {
    let state = match thermal_code {
        ANDROID_THERMAL_NOMINAL => HostThermalState::Nominal,
        ANDROID_THERMAL_FAIR => HostThermalState::Fair,
        ANDROID_THERMAL_SERIOUS => HostThermalState::Serious,
        ANDROID_THERMAL_CRITICAL => HostThermalState::Critical,
        _ => {
            return Err(invalid_argument_value(
                "thermal_code",
                "invalid android thermal state code",
            ));
        }
    };

    Ok(state)
}

pub(super) fn decode_android_power_mode(power_mode_code: u32) -> RuntimeResult<HostPowerMode> {
    let mode = match power_mode_code {
        ANDROID_POWER_MODE_NORMAL => HostPowerMode::Normal,
        ANDROID_POWER_MODE_LOW_POWER => HostPowerMode::LowPower,
        _ => {
            return Err(invalid_argument_value(
                "power_mode_code",
                "invalid android power mode code",
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

fn decode_notification_event_payload(
    payload: NativeSlice<u8>,
    operation: &'static str,
) -> RuntimeResult<NotificationEventValue> {
    let payload = unsafe { payload.as_slice() }
        .map_err(|_| invalid_argument_value("payload", "invalid payload slice"))?;

    decode_callback_host_json(payload, operation, "notification event")
}

fn decode_background_event_payload(
    payload: NativeSlice<u8>,
    operation: &'static str,
) -> RuntimeResult<BackgroundEventValue> {
    let payload = unsafe { payload.as_slice() }
        .map_err(|_| invalid_argument_value("payload", "invalid payload slice"))?;

    decode_callback_host_json(payload, operation, "background event")
}

fn decode_media_event_payload(
    payload: NativeSlice<u8>,
    operation: &'static str,
) -> RuntimeResult<MediaEventValue> {
    let payload = unsafe { payload.as_slice() }
        .map_err(|_| invalid_argument_value("payload", "invalid payload slice"))?;

    decode_callback_host_json(payload, operation, "media event")
}

fn runtime_status(result: RuntimeResult<()>) -> RuntimeStatus {
    RuntimeStatus::from_result(result, None)
}
