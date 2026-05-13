#[cfg(target_os = "linux")]
mod epoll;
#[cfg(any(
    target_os = "macos",
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
pub(crate) use epoll::EpollPoller;
#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
pub(crate) use kqueue::KqueuePoller;
pub(crate) use unix::UnixPoller;
#[cfg(target_os = "linux")]
pub(crate) use uring::IoUringPoller;
