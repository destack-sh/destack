#[cfg(target_os = "linux")]
mod epoll;
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
mod kqueue;
mod unix;
#[cfg(target_os = "linux")]
mod uring;

#[cfg(target_os = "linux")]
pub use epoll::EpollPoller;
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
pub use kqueue::KqueuePoller;
pub use unix::UnixPoller;
#[cfg(target_os = "linux")]
pub use uring::IoUringPoller;
