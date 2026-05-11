mod action;
#[cfg(all(not(test), target_os = "linux"))]
mod linux;
#[cfg(all(test, target_os = "linux"))]
mod test;
#[cfg(any(
    all(not(test), not(target_os = "linux")),
    all(test, not(target_os = "linux"))
))]
mod unsupported;

#[cfg(all(not(test), target_os = "linux"))]
pub(crate) use linux::submit_calendar_request;
#[cfg(all(test, target_os = "linux"))]
pub(crate) use test::submit_calendar_request;
#[cfg(any(
    all(not(test), not(target_os = "linux")),
    all(test, not(target_os = "linux"))
))]
pub(crate) use unsupported::submit_calendar_request;

pub(crate) use action::{desktop_actions, request_actions};
