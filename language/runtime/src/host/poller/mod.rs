mod event;
mod host;
#[allow(
    dead_code,
    reason = "registration is consumed by generated host bindings"
)]
mod poller;
#[cfg(unix)]
#[allow(
    dead_code,
    reason = "registration is consumed by generated host bindings"
)]
mod unix;
#[cfg(windows)]
#[allow(
    dead_code,
    reason = "registration is consumed by generated host bindings"
)]
mod windows;

pub(crate) use event::*;
pub(crate) use host::*;
pub(crate) use poller::*;
#[cfg(target_os = "linux")]
pub(crate) use unix::EpollPoller;
#[cfg(target_os = "linux")]
pub(crate) use unix::IoUringPoller;
#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
pub(crate) use unix::KqueuePoller;
#[cfg(unix)]
pub(crate) use unix::UnixPoller;
#[cfg(windows)]
pub(crate) use windows::WindowsPoller;
