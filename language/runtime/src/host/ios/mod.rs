#[cfg(target_os = "ios")]
mod backend;
#[cfg(any(test, target_os = "ios"))]
mod callback;
#[cfg(any(test, target_os = "ios"))]
mod ffi;
#[cfg(test)]
mod tests;

#[cfg(target_os = "ios")]
pub(crate) use backend::IosHost;
#[cfg(any(test, target_os = "ios"))]
pub use callback::{
    IosApplicationLifecycle, ios_notify_application_lifecycle, ios_notify_intent_custom_action,
    ios_notify_intent_open_file, ios_notify_intent_open_url, ios_notify_intent_share_files,
    ios_notify_intent_share_text, ios_notify_interruption_changed,
    ios_notify_memory_pressure_changed, ios_notify_permission_result,
    ios_notify_power_mode_changed, ios_notify_thermal_state_changed, ios_notify_wake,
    ios_notify_wall_clock_changed,
};
#[cfg(any(test, target_os = "ios"))]
pub use ffi::{
    destack_host_ios_notify_application_lifecycle, destack_host_ios_notify_intent_custom_action,
    destack_host_ios_notify_intent_open_file, destack_host_ios_notify_intent_open_url,
    destack_host_ios_notify_intent_share_files, destack_host_ios_notify_intent_share_text,
    destack_host_ios_notify_interruption_changed, destack_host_ios_notify_memory_pressure_changed,
    destack_host_ios_notify_permission_result, destack_host_ios_notify_power_mode_changed,
    destack_host_ios_notify_thermal_state_changed, destack_host_ios_notify_wake,
    destack_host_ios_notify_wall_clock_changed,
};
