mod capability;
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
pub(crate) use linux::{request_capabilities, submit_contact_request};
#[cfg(all(test, target_os = "linux"))]
pub(crate) use test::{request_capabilities, submit_contact_request};
#[cfg(any(
    all(not(test), not(target_os = "linux")),
    all(test, not(target_os = "linux"))
))]
pub(crate) use unsupported::{request_capabilities, submit_contact_request};

pub(crate) use capability::desktop_capabilities;
