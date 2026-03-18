mod calendar;
mod contact;
mod document;
mod intent;
mod location;
mod submit;

#[cfg(all(test, windows))]
pub(crate) use crate::host::windows::request::{
    calendar::{WindowsCalendarHooks, set_windows_calendar_test_hooks},
    contact::{WindowsContactHooks, set_windows_contact_test_hooks},
    document::set_windows_document_test_pick_hook,
};
#[cfg(all(test, windows))]
pub(crate) use crate::host::windows::tests::{
    WindowsLocationHooks, set_windows_location_test_hooks,
};
#[cfg(windows)]
pub(crate) use location::unregister_location_runtime;
pub(crate) use submit::*;
