#[cfg(windows)]
mod backend;
#[cfg(windows)]
pub(crate) mod core;
mod ffi;
mod notify;

#[cfg(windows)]
pub(crate) use core::{
    WindowsCalendarHooks, WindowsContactHooks, WindowsLocationHooks, pick_documents,
    request_location_permission, set_windows_calendar_test_hooks, set_windows_contact_test_hooks,
    set_windows_document_test_pick_hook, set_windows_location_test_hooks, submit_calendar_request,
    submit_contact_request, submit_location_request, unregister_location_runtime,
};
