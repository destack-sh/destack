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
#[path = "notification.generated.rs"]
mod notification;
#[path = "permission.generated.rs"]
mod permission;
mod system;
#[path = "text.generated.rs"]
mod text;

pub(crate) use background::ios_notify_background_event;
pub(crate) use document::ios_notify_document_result;
pub(crate) use intent::ios_notify_intent_event;
pub(crate) use lifecycle::{IosApplicationLifecycle, ios_notify_application_lifecycle};
pub(crate) use location::ios_notify_location_sample;
pub(crate) use notification::ios_notify_notification_event;
pub(crate) use permission::ios_notify_permission_result;
pub(crate) use system::{
    ios_notify_interruption_changed, ios_notify_memory_pressure_changed,
    ios_notify_power_mode_changed, ios_notify_thermal_state_changed, ios_notify_wake,
    ios_notify_wall_clock_changed,
};
pub(crate) use text::ios_notify_text_input_state;
