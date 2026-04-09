pub(crate) mod r#loop;

#[cfg(target_os = "ios")]
#[path = "background.generated.rs"]
mod background;
#[cfg(target_os = "ios")]
mod core;
#[cfg(target_os = "ios")]
#[path = "document.generated.rs"]
mod document;
#[cfg(target_os = "ios")]
#[path = "intent.generated.rs"]
mod intent;
#[cfg(target_os = "ios")]
pub(crate) mod lifecycle;
#[cfg(target_os = "ios")]
#[path = "location.generated.rs"]
mod location;
#[cfg(target_os = "ios")]
#[path = "notification.generated.rs"]
mod notification;
#[cfg(target_os = "ios")]
#[path = "permission.generated.rs"]
mod permission;
#[cfg(target_os = "ios")]
mod system;
#[cfg(target_os = "ios")]
#[path = "text.generated.rs"]
mod text;

#[cfg(target_os = "ios")]
pub(crate) use background::ios_notify_background_event;
#[cfg(target_os = "ios")]
pub(crate) use document::ios_notify_document_result;
#[cfg(target_os = "ios")]
pub(crate) use intent::ios_notify_intent_event;
#[cfg(target_os = "ios")]
pub(crate) use lifecycle::{IosApplicationLifecycle, ios_notify_application_lifecycle};
#[cfg(target_os = "ios")]
pub(crate) use location::ios_notify_location_sample;
#[cfg(target_os = "ios")]
pub(crate) use notification::ios_notify_notification_event;
#[cfg(target_os = "ios")]
pub(crate) use permission::ios_notify_permission_result;
#[cfg(target_os = "ios")]
pub(crate) use system::{
    ios_notify_interruption_changed, ios_notify_memory_pressure_changed,
    ios_notify_power_mode_changed, ios_notify_thermal_state_changed, ios_notify_wake,
    ios_notify_wall_clock_changed,
};
#[cfg(target_os = "ios")]
pub(crate) use text::ios_notify_text_input_state;
