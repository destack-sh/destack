use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(unix)]
use crate::platform::UnixPoller;
#[cfg(windows)]
use crate::platform::WindowsPoller;
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
use crate::platform::poller::KqueuePoller;
#[cfg(target_os = "linux")]
use crate::platform::poller::{EpollPoller, IoUringPoller};
use crate::platform::{PlatformError, PlatformPoller};
use destack_workspace::{PollerBackend, RuntimeOptions};

/// Build a poller instance from runtime options.
pub(super) fn poller_for_options(
    options: &RuntimeOptions,
) -> RuntimeResult<Option<Box<dyn PlatformPoller>>> {
    // select the requested poller backend
    let backend = options.scheduler.poller_backend;

    // create the poller instance for the selected backend
    let poller = match backend {
        PollerBackend::Auto => auto_poller()?,
        PollerBackend::IoUring => poller_iouring()?,
        PollerBackend::Epoll => poller_epoll()?,
        PollerBackend::Kqueue => poller_kqueue()?,
        PollerBackend::Poll => poller_poll()?,
        PollerBackend::Windows => poller_windows()?,
    };

    Ok(Some(poller))
}

/// Return a not-supported error for one poller backend.
fn poller_not_supported(name: &str) -> RuntimeResult<Box<dyn PlatformPoller>> {
    // report unsupported backend for this target
    Err(RuntimeError::from(PlatformError::not_supported(format!(
        "poller backend {name} is not supported on this platform"
    )))
    .boxed())
}

/// Build an io_uring poller.
#[cfg(target_os = "linux")]
fn poller_iouring() -> RuntimeResult<Box<dyn PlatformPoller>> {
    // create Linux io_uring poller
    Ok(Box::new(IoUringPoller::new()?))
}

/// Build an io_uring poller.
#[cfg(not(target_os = "linux"))]
fn poller_iouring() -> RuntimeResult<Box<dyn PlatformPoller>> {
    // reject io_uring on unsupported targets
    poller_not_supported("io_uring")
}

/// Build an epoll poller.
#[cfg(target_os = "linux")]
fn poller_epoll() -> RuntimeResult<Box<dyn PlatformPoller>> {
    // create Linux epoll poller
    Ok(Box::new(EpollPoller::new()?))
}

/// Build an epoll poller.
#[cfg(not(target_os = "linux"))]
fn poller_epoll() -> RuntimeResult<Box<dyn PlatformPoller>> {
    // reject epoll on unsupported targets
    poller_not_supported("epoll")
}

/// Build a kqueue poller.
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
fn poller_kqueue() -> RuntimeResult<Box<dyn PlatformPoller>> {
    // create BSD kqueue poller
    Ok(Box::new(KqueuePoller::new()?))
}

/// Build a kqueue poller.
#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
)))]
fn poller_kqueue() -> RuntimeResult<Box<dyn PlatformPoller>> {
    // reject kqueue on unsupported targets
    poller_not_supported("kqueue")
}

/// Build a poll(2)-style poller.
#[cfg(unix)]
fn poller_poll() -> RuntimeResult<Box<dyn PlatformPoller>> {
    // create Unix poll backend
    Ok(Box::new(UnixPoller::new()?))
}

/// Build a poll(2)-style poller.
#[cfg(not(unix))]
fn poller_poll() -> RuntimeResult<Box<dyn PlatformPoller>> {
    // reject poll on unsupported targets
    poller_not_supported("poll")
}

/// Build a Windows poller.
#[cfg(windows)]
fn poller_windows() -> RuntimeResult<Box<dyn PlatformPoller>> {
    // create Windows poller
    Ok(Box::new(WindowsPoller::new()?))
}

/// Build a Windows poller.
#[cfg(not(windows))]
fn poller_windows() -> RuntimeResult<Box<dyn PlatformPoller>> {
    // reject Windows backend on unsupported targets
    poller_not_supported("windows")
}

/// Build the automatic best-available poller.
fn auto_poller() -> RuntimeResult<Box<dyn PlatformPoller>> {
    #[cfg(target_os = "linux")]
    {
        // prefer io_uring on Linux when available
        if let Ok(poller) = IoUringPoller::new() {
            return Ok(Box::new(poller));
        }

        // fall back to epoll on Linux
        if let Ok(poller) = EpollPoller::new() {
            return Ok(Box::new(poller));
        }

        // final Linux fallback is poll
        poller_poll()
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    ))]
    {
        // prefer kqueue on BSD-family targets
        if let Ok(poller) = KqueuePoller::new() {
            return Ok(Box::new(poller));
        }

        // final BSD-family fallback is poll
        poller_poll()
    }

    #[cfg(windows)]
    {
        // use Windows backend directly
        return poller_windows();
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly",
        windows
    )))]
    {
        // reject auto backend on unsupported targets
        poller_not_supported("auto")
    }
}
