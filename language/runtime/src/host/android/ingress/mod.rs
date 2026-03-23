mod background;
mod core;
mod intent;
pub(crate) mod lifecycle;
mod location;
#[cfg(target_os = "android")]
pub(crate) mod message;
mod notification;
mod permission;
mod system;

pub(crate) use background::android_notify_background_event;
pub(crate) use intent::{
    android_notify_intent_custom_action, android_notify_intent_open_file,
    android_notify_intent_open_url, android_notify_intent_share_files,
    android_notify_intent_share_text,
};
pub(crate) use lifecycle::{AndroidActivityLifecycle, android_notify_activity_lifecycle};
pub(crate) use location::android_notify_location_sample;
pub(crate) use notification::android_notify_notification_event;
pub(crate) use permission::android_notify_permission_result;
pub(crate) use system::{
    android_notify_interruption_changed, android_notify_memory_pressure_changed,
    android_notify_power_mode_changed, android_notify_thermal_state_changed, android_notify_wake,
    android_notify_wall_clock_changed,
};
