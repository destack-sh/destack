#[cfg(target_os = "linux")]
mod adapter;
#[cfg(any(test, target_os = "linux"))]
mod ingress;
#[cfg(test)]
mod tests;

#[cfg(target_os = "linux")]
pub(crate) use adapter::LinuxHost;
#[cfg(any(test, target_os = "linux"))]
pub(crate) use ingress::*;
#[cfg(all(test, target_os = "linux"))]
pub(crate) use tests::{
    LinuxCalendarHooks, LinuxContactHooks, LinuxLocationHooks, publish_location_sample,
    set_linux_calendar_test_hooks, set_linux_contact_test_hooks, set_linux_location_test_hooks,
    submit_calendar_request as submit_test_calendar_request,
    submit_contact_request as submit_test_contact_request,
    submit_location_request as submit_test_location_request,
    unregister_location_runtime as unregister_test_location_runtime,
};
