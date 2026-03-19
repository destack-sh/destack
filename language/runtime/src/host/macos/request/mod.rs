mod calendar;
mod contact;
mod dispatch;
mod document;
mod location;
#[cfg(target_os = "macos")]
pub(crate) use crate::host::macos::request::location::unregister_location_runtime;
#[cfg(all(test, target_os = "macos"))]
pub(crate) use crate::host::macos::request::{
    calendar::{MacosCalendarHooks, set_macos_calendar_test_hooks},
    contact::{MacosContactHooks, set_macos_contact_test_hooks},
    document::set_macos_document_test_pick_hook,
    location::{MacosLocationHooks, set_macos_location_test_hooks},
};
pub(crate) use dispatch::*;
