#[cfg(target_os = "linux")]
mod epoll;
mod event;
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
mod kqueue;
mod poller;
#[cfg(unix)]
#[path = "unix.rs"]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
pub use epoll::EpollPoller;
pub use event::*;
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
pub use kqueue::KqueuePoller;
pub use poller::*;
#[cfg(unix)]
pub use unix::UnixPoller;
#[cfg(windows)]
pub use windows::WindowsPoller;
