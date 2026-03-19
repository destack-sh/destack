#[cfg(windows)]
mod adapter;
#[cfg(any(test, windows))]
mod ingress;
#[cfg(windows)]
mod request;
#[cfg(test)]
pub(crate) mod tests;

#[cfg(windows)]
pub(crate) use adapter::WindowsHost;
#[cfg(windows)]
pub(crate) use ingress::process_ingress_loop;
#[cfg(any(test, windows))]
pub(crate) use ingress::*;
#[cfg(all(test, windows))]
pub(crate) use request::{
    WindowsCalendarHooks, WindowsContactHooks, WindowsLocationHooks,
    set_windows_calendar_test_hooks, set_windows_contact_test_hooks,
    set_windows_document_test_pick_hook, set_windows_location_test_hooks,
};
#[cfg(windows)]
pub(crate) use request::{
    request_capabilities as windows_request_capabilities, submit_request as submit_windows_request,
    unregister_location_runtime,
};
