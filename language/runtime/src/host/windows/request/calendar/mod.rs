#[cfg(all(not(test), windows))]
mod service;
#[cfg(all(not(test), windows))]
mod submit;
#[cfg(all(test, windows))]
mod test;
#[cfg(all(test, not(windows)))]
mod unsupported;

/// Submit one Windows calendar request through the active host lane.
#[cfg(all(not(test), windows))]
pub(crate) use submit::submit_calendar_request;
#[cfg(all(test, windows))]
pub(crate) use test::{
    WindowsCalendarHooks, set_windows_calendar_test_hooks, submit_calendar_request,
};
#[cfg(all(test, not(windows)))]
pub(crate) use unsupported::submit_calendar_request;
