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
pub(crate) use linux::{request_actions, submit_location_request, unregister_location_runtime};
#[cfg(all(test, target_os = "linux"))]
pub(crate) use test::{request_actions, submit_location_request, unregister_location_runtime};
#[cfg(any(
    all(not(test), not(target_os = "linux")),
    all(test, not(target_os = "linux"))
))]
pub(crate) use unsupported::{request_actions, submit_location_request};
