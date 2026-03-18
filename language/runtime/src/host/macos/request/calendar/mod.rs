#[cfg(all(not(test), target_os = "macos"))]
mod core;
#[cfg(all(not(test), target_os = "macos"))]
mod native;
#[cfg(all(not(test), target_os = "macos"))]
mod query;
#[cfg(all(not(test), target_os = "macos"))]
mod recurrence;
#[cfg(all(not(test), target_os = "macos"))]
mod submit;
#[cfg(all(test, target_os = "macos"))]
mod test;
#[cfg(all(test, not(target_os = "macos")))]
mod unsupported;
#[cfg(all(not(test), target_os = "macos"))]
mod write;

/// Submit one macOS calendar request when the native backend is available.
#[cfg(all(not(test), target_os = "macos"))]
pub(crate) use submit::submit_calendar_request;
#[cfg(all(test, target_os = "macos"))]
pub(crate) use test::{MacosCalendarHooks, set_macos_calendar_test_hooks, submit_calendar_request};
#[cfg(all(test, not(target_os = "macos")))]
pub(crate) use unsupported::submit_calendar_request;
