#[cfg(target_os = "macos")]
mod backend;
pub(crate) mod core;
mod ffi;
mod notify;

pub(crate) use core::{
    MacosCalendarHooks, MacosContactHooks, MacosLocationHooks, pick_documents,
    request_location_permission, set_macos_calendar_test_hooks, set_macos_contact_test_hooks,
    set_macos_document_test_pick_hook, set_macos_location_test_hooks, submit_calendar_request,
    submit_contact_request, submit_location_request, unregister_location_runtime,
};
