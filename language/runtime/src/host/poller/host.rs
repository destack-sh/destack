use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::HostError;
#[cfg(any(
    test,
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
use crate::host::HostErrorCode;
use destack_workspace::PollerBackend;

#[cfg(target_os = "linux")]
use super::EpollPoller;
use super::HostPoller;
#[cfg(target_os = "linux")]
use super::IoUringPoller;
#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
use super::KqueuePoller;
#[cfg(unix)]
use super::UnixPoller;
#[cfg(windows)]
use super::WindowsPoller;

/// Profile selector for automatic backend choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum HostPollerProfile {
    /// Runtime scheduler poller policy.
    RuntimeScheduler,
    /// io.poll binding poller policy.
    #[cfg_attr(not(test), allow(dead_code))]
    IoBinding,
}

/// Create one host poller from a canonical backend selector.
pub(crate) fn create_host_poller(
    backend: PollerBackend,
    profile: HostPollerProfile,
) -> RuntimeResult<Box<dyn HostPoller>> {
    match backend {
        PollerBackend::Auto => match profile {
            HostPollerProfile::RuntimeScheduler => create_auto_runtime_poller(),
            HostPollerProfile::IoBinding => create_auto_io_poller(),
        },
        _ => create_explicit_poller(backend),
    }
}

/// Create one poller for runtime scheduler use.
pub(crate) fn create_host_poller_for_runtime(
    backend: PollerBackend,
) -> RuntimeResult<Box<dyn HostPoller>> {
    create_host_poller(backend, HostPollerProfile::RuntimeScheduler)
}

/// Create one poller for io.poll binding use.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn create_host_poller_for_io(
    backend: PollerBackend,
) -> RuntimeResult<Box<dyn HostPoller>> {
    create_host_poller(backend, HostPollerProfile::IoBinding)
}

/// Return one standardized backend-not-supported error.
fn poller_not_supported(backend: PollerBackend) -> RuntimeResult<Box<dyn HostPoller>> {
    let backend = backend_label(backend);

    Err(RuntimeError::from(HostError::not_supported(format!(
        "poller backend {backend} is not supported on this platform"
    )))
    .boxed())
}

/// Return one stable backend label for diagnostics.
const fn backend_label(backend: PollerBackend) -> &'static str {
    match backend {
        PollerBackend::Auto => "auto",
        PollerBackend::IoUring => "io_uring",
        PollerBackend::Epoll => "epoll",
        PollerBackend::Kqueue => "kqueue",
        PollerBackend::Poll => "poll",
        PollerBackend::Windows => "windows",
    }
}

/// Return whether one runtime error reports notSupported.
#[cfg(any(
    test,
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
fn is_not_supported_error(error: &RuntimeError) -> bool {
    match error.host_error() {
        Some(error) => error.code == HostErrorCode::NotSupported,
        None => false,
    }
}

/// Try one explicit backend and only suppress notSupported errors.
#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
fn try_create_explicit_poller(
    backend: PollerBackend,
) -> RuntimeResult<Option<Box<dyn HostPoller>>> {
    match create_explicit_poller(backend) {
        Ok(poller) => Ok(Some(poller)),
        Err(error) => {
            if is_not_supported_error(error.as_ref()) {
                Ok(None)
            } else {
                Err(error)
            }
        }
    }
}

/// Create one explicit backend without fallback policy.
fn create_explicit_poller(backend: PollerBackend) -> RuntimeResult<Box<dyn HostPoller>> {
    match backend {
        PollerBackend::Auto => poller_not_supported(PollerBackend::Auto),
        PollerBackend::IoUring => {
            #[cfg(target_os = "linux")]
            {
                Ok(Box::new(IoUringPoller::new()?))
            }

            #[cfg(not(target_os = "linux"))]
            {
                poller_not_supported(PollerBackend::IoUring)
            }
        }
        PollerBackend::Epoll => {
            #[cfg(target_os = "linux")]
            {
                Ok(Box::new(EpollPoller::new()?))
            }

            #[cfg(not(target_os = "linux"))]
            {
                poller_not_supported(PollerBackend::Epoll)
            }
        }
        PollerBackend::Kqueue => {
            #[cfg(any(
                target_os = "macos",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd",
                target_os = "dragonfly"
            ))]
            {
                Ok(Box::new(KqueuePoller::new()?))
            }

            #[cfg(not(any(
                target_os = "macos",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd",
                target_os = "dragonfly"
            )))]
            {
                poller_not_supported(PollerBackend::Kqueue)
            }
        }
        PollerBackend::Poll => {
            #[cfg(unix)]
            {
                Ok(Box::new(UnixPoller::new()?))
            }

            #[cfg(not(unix))]
            {
                poller_not_supported(PollerBackend::Poll)
            }
        }
        PollerBackend::Windows => {
            #[cfg(windows)]
            {
                Ok(Box::new(WindowsPoller::new()?))
            }

            #[cfg(not(windows))]
            {
                poller_not_supported(PollerBackend::Windows)
            }
        }
    }
}

/// Create one automatic backend for runtime scheduler usage.
fn create_auto_runtime_poller() -> RuntimeResult<Box<dyn HostPoller>> {
    #[cfg(target_os = "linux")]
    {
        // prefer io_uring, then epoll, then poll on Linux
        if let Some(poller) = try_create_explicit_poller(PollerBackend::IoUring)? {
            return Ok(poller);
        }
        if let Some(poller) = try_create_explicit_poller(PollerBackend::Epoll)? {
            return Ok(poller);
        }
        create_explicit_poller(PollerBackend::Poll)
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    ))]
    {
        // prefer kqueue, then poll on BSD-family targets
        if let Some(poller) = try_create_explicit_poller(PollerBackend::Kqueue)? {
            return Ok(poller);
        }
        create_explicit_poller(PollerBackend::Poll)
    }

    #[cfg(windows)]
    {
        // use the Windows backend on Windows hosts
        create_explicit_poller(PollerBackend::Windows)
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly",
        windows
    )))]
    {
        poller_not_supported(PollerBackend::Auto)
    }
}

/// Create one automatic backend for io.poll binding usage.
fn create_auto_io_poller() -> RuntimeResult<Box<dyn HostPoller>> {
    #[cfg(target_os = "linux")]
    {
        // prefer epoll, then poll for io.poll on Linux
        if let Some(poller) = try_create_explicit_poller(PollerBackend::Epoll)? {
            return Ok(poller);
        }
        create_explicit_poller(PollerBackend::Poll)
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    ))]
    {
        // prefer kqueue, then poll for io.poll on BSD-family targets
        if let Some(poller) = try_create_explicit_poller(PollerBackend::Kqueue)? {
            return Ok(poller);
        }
        create_explicit_poller(PollerBackend::Poll)
    }

    #[cfg(windows)]
    {
        // use the poll-style backend on Windows hosts
        create_explicit_poller(PollerBackend::Windows)
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly",
        windows
    )))]
    {
        poller_not_supported(PollerBackend::Auto)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use super::*;

    use crate::host::HostError;

    /// Treat not-supported host errors as fallback candidates.
    #[test]
    fn test_is_not_supported_error_matches_not_supported_code() {
        let error = RuntimeError::from(HostError::not_supported("backend"));
        assert!(is_not_supported_error(&error));
    }

    /// Do not treat other host errors as fallback candidates.
    #[test]
    fn test_is_not_supported_error_rejects_other_codes() {
        let error = RuntimeError::from(HostError::io("backend setup failed"));
        assert!(!is_not_supported_error(&error));
    }

    /// Reject runtime poll backend `poll` on Windows hosts.
    #[cfg(windows)]
    #[test]
    fn test_runtime_poll_backend_poll_is_not_supported_on_windows() {
        let result = create_host_poller_for_runtime(PollerBackend::Poll);
        assert!(result.is_err());
        let error = result.err().expect("poll backend should fail");
        let platform = error
            .host_error()
            .expect("error should contain one host error");
        assert_eq!(platform.code, HostErrorCode::NotSupported);
    }

    /// Wake one blocking io poller wait through one shared wake handle.
    #[test]
    fn test_io_poller_wake_handle_interrupts_blocking_poll() {
        let poller = create_host_poller_for_io(PollerBackend::Auto)
            .expect("auto io poller should initialize");
        let wake_handle = poller
            .wake_handle()
            .expect("io poller should expose one wake handle");

        let poller = Arc::new(Mutex::new(poller));
        let poller_for_thread = Arc::clone(&poller);
        let started_at = Instant::now();

        let waiter = std::thread::spawn(move || {
            let mut poller = poller_for_thread
                .lock()
                .expect("poller lock should not be poisoned");
            poller
                .poll(Some(2_000_000_000))
                .expect("poll should unblock after wake");
        });

        std::thread::sleep(Duration::from_millis(25));
        wake_handle.wake().expect("wake should succeed");
        waiter.join().expect("wait thread should join");

        let elapsed = started_at.elapsed();
        assert!(
            elapsed < Duration::from_millis(1_000),
            "poll wake should interrupt quickly, elapsed={elapsed:?}"
        );
    }
}
