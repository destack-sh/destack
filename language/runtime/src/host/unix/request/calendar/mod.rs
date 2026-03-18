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
pub(crate) use linux::*;
#[cfg(all(test, target_os = "linux"))]
pub(crate) use test::*;
#[cfg(any(
    all(not(test), not(target_os = "linux")),
    all(test, not(target_os = "linux"))
))]
pub(crate) use unsupported::*;

pub(crate) use capability::*;
