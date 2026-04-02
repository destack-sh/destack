#[path = "background.generated.rs"]
mod background;
mod core;
#[path = "document.generated.rs"]
mod document;
#[path = "intent.generated.rs"]
mod intent;
pub(crate) mod lifecycle;
#[path = "location.generated.rs"]
mod location;
#[cfg(target_os = "android")]
pub(crate) mod message;
#[path = "notification.generated.rs"]
mod notification;
#[path = "permission.generated.rs"]
mod permission;
mod system;
#[path = "text.generated.rs"]
mod text;

pub(crate) use background::android_notify_background_event;
pub(crate) use document::android_notify_document_result;
pub(crate) use intent::android_notify_intent_event;
pub(crate) use lifecycle::{AndroidActivityLifecycle, android_notify_activity_lifecycle};
pub(crate) use location::android_notify_location_sample;
pub(crate) use notification::android_notify_notification_event;
pub(crate) use permission::android_notify_permission_result;
pub(crate) use system::{
    android_notify_interruption_changed, android_notify_memory_pressure_changed,
    android_notify_power_mode_changed, android_notify_thermal_state_changed, android_notify_wake,
    android_notify_wall_clock_changed,
};
pub(crate) use text::android_notify_text_input_state;
