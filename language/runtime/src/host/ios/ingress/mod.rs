mod background;
mod core;
mod intent;
pub(crate) mod lifecycle;
mod location;
mod notification;
mod permission;
mod system;

pub(crate) use background::ios_notify_background_event;
pub(crate) use intent::{
    ios_notify_intent_custom_action, ios_notify_intent_open_file, ios_notify_intent_open_url,
    ios_notify_intent_share_files, ios_notify_intent_share_text,
};
pub(crate) use lifecycle::{IosApplicationLifecycle, ios_notify_application_lifecycle};
pub(crate) use location::ios_notify_location_sample;
pub(crate) use notification::ios_notify_notification_event;
pub(crate) use permission::ios_notify_permission_result;
pub(crate) use system::{
    ios_notify_interruption_changed, ios_notify_memory_pressure_changed,
    ios_notify_power_mode_changed, ios_notify_thermal_state_changed, ios_notify_wake,
    ios_notify_wall_clock_changed,
};
