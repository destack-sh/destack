#[cfg(target_os = "macos")]
mod backend;
mod callback;
mod ffi;
#[cfg(test)]
mod tests;

#[cfg(target_os = "macos")]
pub(crate) use backend::MacosHost;
pub use callback::{
    MacosApplicationLifecycle, macos_notify_application_lifecycle,
    macos_notify_intent_custom_action, macos_notify_intent_open_file, macos_notify_intent_open_url,
    macos_notify_intent_share_files, macos_notify_intent_share_text,
    macos_notify_interruption_changed, macos_notify_memory_pressure_changed,
    macos_notify_permission_result, macos_notify_power_mode_changed,
    macos_notify_thermal_state_changed, macos_notify_wake, macos_notify_wall_clock_changed,
};
pub use ffi::{
    destack_host_macos_notify_application_lifecycle,
    destack_host_macos_notify_intent_custom_action, destack_host_macos_notify_intent_open_file,
    destack_host_macos_notify_intent_open_url, destack_host_macos_notify_intent_share_files,
    destack_host_macos_notify_intent_share_text, destack_host_macos_notify_interruption_changed,
    destack_host_macos_notify_memory_pressure_changed, destack_host_macos_notify_permission_result,
    destack_host_macos_notify_power_mode_changed, destack_host_macos_notify_thermal_state_changed,
    destack_host_macos_notify_wake, destack_host_macos_notify_wall_clock_changed,
};
