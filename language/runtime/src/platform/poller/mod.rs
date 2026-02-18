mod event;
mod host;
mod poller;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

pub use event::*;
pub use host::*;
pub use poller::*;
#[cfg(target_os = "linux")]
pub use unix::EpollPoller;
#[cfg(target_os = "linux")]
pub use unix::IoUringPoller;
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
pub use unix::KqueuePoller;
#[cfg(unix)]
pub use unix::UnixPoller;
#[cfg(windows)]
pub use windows::WindowsPoller;
