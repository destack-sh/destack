#[cfg(target_os = "macos")]
mod adapter;
#[cfg(any(test, target_os = "macos"))]
mod ingress;
#[cfg(any(test, target_os = "macos"))]
mod request;
#[cfg(test)]
mod tests;

#[cfg(target_os = "macos")]
pub(crate) use adapter::MacosHost;
#[cfg(any(test, target_os = "macos"))]
pub(crate) use ingress::*;
#[cfg(target_os = "macos")]
pub(crate) use request::unregister_location_runtime;
#[cfg(all(test, target_os = "macos"))]
pub(crate) use request::{
    MacosCalendarHooks, MacosContactHooks, MacosLocationHooks, set_macos_calendar_test_hooks,
    set_macos_contact_test_hooks, set_macos_document_test_pick_hook, set_macos_location_test_hooks,
};
#[cfg(target_os = "macos")]
pub(crate) use request::{
    request_capabilities as macos_request_capabilities, submit_request as submit_macos_request,
};
