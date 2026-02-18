use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
#[cfg(any(
    test,
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
use crate::platform::PlatformErrorCode;

#[cfg(target_os = "linux")]
use super::EpollPoller;
#[cfg(target_os = "linux")]
use super::IoUringPoller;
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
use super::KqueuePoller;
use super::PlatformPoller;
#[cfg(unix)]
use super::UnixPoller;
#[cfg(windows)]
use super::WindowsPoller;

/// Canonical platform poller backend selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformPollerBackend {
    /// Select one policy-specific automatic backend.
    Auto,
    /// Select io_uring where supported.
    IoUring,
    /// Select epoll where supported.
    Epoll,
    /// Select kqueue where supported.
    Kqueue,
    /// Select poll-style readiness backend.
    Poll,
    /// Select Windows backend.
    Windows,
}

/// Profile selector for automatic backend choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformPollerProfile {
    /// Runtime scheduler poller policy.
    RuntimeScheduler,
    /// io.poll binding poller policy.
    IoBinding,
}

/// Create one platform poller from a canonical backend selector.
pub fn create_platform_poller(
    backend: PlatformPollerBackend,
    profile: PlatformPollerProfile,
) -> RuntimeResult<Box<dyn PlatformPoller>> {
    match backend {
        PlatformPollerBackend::Auto => match profile {
            PlatformPollerProfile::RuntimeScheduler => create_auto_runtime_poller(),
            PlatformPollerProfile::IoBinding => create_auto_io_poller(),
        },
        _ => create_explicit_poller(backend),
    }
}

/// Create one poller for runtime scheduler use.
pub fn create_platform_poller_for_runtime(
    backend: PlatformPollerBackend,
) -> RuntimeResult<Box<dyn PlatformPoller>> {
    create_platform_poller(backend, PlatformPollerProfile::RuntimeScheduler)
}

/// Create one poller for io.poll binding use.
pub fn create_platform_poller_for_io(
    backend: PlatformPollerBackend,
) -> RuntimeResult<Box<dyn PlatformPoller>> {
    create_platform_poller(backend, PlatformPollerProfile::IoBinding)
}

/// Return one standardized backend-not-supported error.
fn poller_not_supported(backend: PlatformPollerBackend) -> RuntimeResult<Box<dyn PlatformPoller>> {
    let backend = backend_label(backend);

    Err(RuntimeError::from(PlatformError::not_supported(format!(
        "poller backend {backend} is not supported on this platform"
    )))
    .boxed())
}

/// Return one stable backend label for diagnostics.
const fn backend_label(backend: PlatformPollerBackend) -> &'static str {
    match backend {
        PlatformPollerBackend::Auto => "auto",
        PlatformPollerBackend::IoUring => "io_uring",
        PlatformPollerBackend::Epoll => "epoll",
        PlatformPollerBackend::Kqueue => "kqueue",
        PlatformPollerBackend::Poll => "poll",
        PlatformPollerBackend::Windows => "windows",
    }
}

/// Return whether one runtime error reports notSupported.
#[cfg(any(
    test,
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
fn is_not_supported_error(error: &RuntimeError) -> bool {
    match error.platform_error() {
        Some(error) => error.code == PlatformErrorCode::NotSupported,
        None => false,
    }
}

/// Try one explicit backend and only suppress notSupported errors.
#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
fn try_create_explicit_poller(
    backend: PlatformPollerBackend,
) -> RuntimeResult<Option<Box<dyn PlatformPoller>>> {
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
fn create_explicit_poller(
    backend: PlatformPollerBackend,
) -> RuntimeResult<Box<dyn PlatformPoller>> {
    match backend {
        PlatformPollerBackend::Auto => poller_not_supported(PlatformPollerBackend::Auto),
        PlatformPollerBackend::IoUring => {
            #[cfg(target_os = "linux")]
            {
                Ok(Box::new(IoUringPoller::new()?))
            }

            #[cfg(not(target_os = "linux"))]
            {
                poller_not_supported(PlatformPollerBackend::IoUring)
            }
        }
        PlatformPollerBackend::Epoll => {
            #[cfg(target_os = "linux")]
            {
                Ok(Box::new(EpollPoller::new()?))
            }

            #[cfg(not(target_os = "linux"))]
            {
                poller_not_supported(PlatformPollerBackend::Epoll)
            }
        }
        PlatformPollerBackend::Kqueue => {
            #[cfg(any(
                target_os = "macos",
                target_os = "ios",
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
                target_os = "ios",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd",
                target_os = "dragonfly"
            )))]
            {
                poller_not_supported(PlatformPollerBackend::Kqueue)
            }
        }
        PlatformPollerBackend::Poll => {
            #[cfg(unix)]
            {
                Ok(Box::new(UnixPoller::new()?))
            }

            #[cfg(not(unix))]
            {
                poller_not_supported(PlatformPollerBackend::Poll)
            }
        }
        PlatformPollerBackend::Windows => {
            #[cfg(windows)]
            {
                Ok(Box::new(WindowsPoller::new()?))
            }

            #[cfg(not(windows))]
            {
                poller_not_supported(PlatformPollerBackend::Windows)
            }
        }
    }
}

/// Create one automatic backend for runtime scheduler usage.
fn create_auto_runtime_poller() -> RuntimeResult<Box<dyn PlatformPoller>> {
    #[cfg(target_os = "linux")]
    {
        // prefer io_uring, then epoll, then poll on Linux
        if let Some(poller) = try_create_explicit_poller(PlatformPollerBackend::IoUring)? {
            return Ok(poller);
        }
        if let Some(poller) = try_create_explicit_poller(PlatformPollerBackend::Epoll)? {
            return Ok(poller);
        }
        create_explicit_poller(PlatformPollerBackend::Poll)
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
        // prefer kqueue, then poll on BSD-family targets
        if let Some(poller) = try_create_explicit_poller(PlatformPollerBackend::Kqueue)? {
            return Ok(poller);
        }
        create_explicit_poller(PlatformPollerBackend::Poll)
    }

    #[cfg(windows)]
    {
        // use the Windows backend on Windows hosts
        create_explicit_poller(PlatformPollerBackend::Windows)
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
        poller_not_supported(PlatformPollerBackend::Auto)
    }
}

/// Create one automatic backend for io.poll binding usage.
fn create_auto_io_poller() -> RuntimeResult<Box<dyn PlatformPoller>> {
    #[cfg(target_os = "linux")]
    {
        // prefer epoll, then poll for io.poll on Linux
        if let Some(poller) = try_create_explicit_poller(PlatformPollerBackend::Epoll)? {
            return Ok(poller);
        }
        create_explicit_poller(PlatformPollerBackend::Poll)
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
        // prefer kqueue, then poll for io.poll on BSD-family targets
        if let Some(poller) = try_create_explicit_poller(PlatformPollerBackend::Kqueue)? {
            return Ok(poller);
        }
        create_explicit_poller(PlatformPollerBackend::Poll)
    }

    #[cfg(windows)]
    {
        // use the poll-style backend on Windows hosts
        create_explicit_poller(PlatformPollerBackend::Windows)
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
        poller_not_supported(PlatformPollerBackend::Auto)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use super::*;

    use crate::platform::PlatformError;

    /// Treat not-supported platform errors as fallback candidates.
    #[test]
    fn test_is_not_supported_error_matches_not_supported_code() {
        let error = RuntimeError::from(PlatformError::not_supported("backend"));
        assert!(is_not_supported_error(&error));
    }

    /// Do not treat other platform errors as fallback candidates.
    #[test]
    fn test_is_not_supported_error_rejects_other_codes() {
        let error = RuntimeError::from(PlatformError::io("backend setup failed"));
        assert!(!is_not_supported_error(&error));
    }

    /// Reject runtime poll backend `poll` on Windows hosts.
    #[cfg(windows)]
    #[test]
    fn test_runtime_poll_backend_poll_is_not_supported_on_windows() {
        let result = create_platform_poller_for_runtime(PlatformPollerBackend::Poll);
        assert!(result.is_err());
        let error = result.err().expect("poll backend should fail");
        let platform = error
            .platform_error()
            .expect("error should contain one platform error");
        assert_eq!(platform.code, PlatformErrorCode::NotSupported);
    }

    /// Wake one blocking io poller wait through one shared wake handle.
    #[test]
    fn test_io_poller_wake_handle_interrupts_blocking_poll() {
        let poller = create_platform_poller_for_io(PlatformPollerBackend::Auto)
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
