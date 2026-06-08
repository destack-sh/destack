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
use destack_repository::PollerBackend;

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

/// Host poller paired with its concrete backend descriptor.
pub(crate) struct HostPollerInstance {
    /// Concrete poller backend.
    backend: PollerBackend,
    /// Platform poller implementation.
    poller: Box<dyn HostPoller>,
}

impl HostPollerInstance {
    /// Create one host poller instance.
    pub(crate) fn new(backend: PollerBackend, poller: Box<dyn HostPoller>) -> Self {
        Self { backend, poller }
    }

    /// Return the concrete poller backend.
    pub(crate) const fn backend(&self) -> PollerBackend {
        self.backend
    }
}

impl std::fmt::Debug for HostPollerInstance {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HostPollerInstance")
            .field("backend", &self.backend)
            .field("poller", &"<host poller>")
            .finish()
    }
}

impl HostPoller for HostPollerInstance {
    fn register(
        &mut self,
        resource_id: crate::host::ResourceId,
        handle: super::HostHandle,
        token: super::PollerToken,
        interests: super::PollInterest,
        flags: super::HostPollerFlags,
    ) -> RuntimeResult<()> {
        self.poller
            .register(resource_id, handle, token, interests, flags)
    }

    fn update(
        &mut self,
        resource_id: crate::host::ResourceId,
        token: super::PollerToken,
        interests: super::PollInterest,
        flags: super::HostPollerFlags,
    ) -> RuntimeResult<()> {
        self.poller.update(resource_id, token, interests, flags)
    }

    fn deregister(&mut self, resource_id: crate::host::ResourceId) -> RuntimeResult<()> {
        self.poller.deregister(resource_id)
    }

    fn wake_handle(&self) -> Option<std::sync::Arc<dyn super::PollerWakeHandle>> {
        self.poller.wake_handle()
    }

    fn wake(&mut self) -> RuntimeResult<()> {
        self.poller.wake()
    }

    fn poll(&mut self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<super::PollerEvent>> {
        self.poller.poll(timeout_nanos)
    }
}

/// Create one host poller from a canonical backend selector.
pub(crate) fn create_host_poller(backend: PollerBackend) -> RuntimeResult<HostPollerInstance> {
    match backend {
        PollerBackend::Auto => create_auto_poller(),
        _ => Ok(HostPollerInstance::new(backend, open_host_poller(backend)?)),
    }
}

/// Return one standardized backend unsupported error.
fn poller_not_supported(backend: PollerBackend) -> RuntimeResult<Box<dyn HostPoller>> {
    let backend = backend_label(backend);

    Err(RuntimeError::from(HostError::not_supported(format!(
        "poller backend {backend} is not supported on this platform"
    )))
    .boxed())
}

/// Return one standardized backend unsupported error for poller instances.
#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly",
    windows
)))]
fn poller_instance_not_supported(backend: PollerBackend) -> RuntimeResult<HostPollerInstance> {
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
fn try_open_host_poller(backend: PollerBackend) -> RuntimeResult<Option<HostPollerInstance>> {
    match open_host_poller(backend) {
        Ok(poller) => Ok(Some(HostPollerInstance::new(backend, poller))),
        Err(error) => {
            if is_not_supported_error(error.as_ref()) {
                Ok(None)
            } else {
                Err(error)
            }
        }
    }
}

/// Open one explicitly selected host poller backend.
fn open_host_poller(backend: PollerBackend) -> RuntimeResult<Box<dyn HostPoller>> {
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

/// Create one automatic host poller for this target.
fn create_auto_poller() -> RuntimeResult<HostPollerInstance> {
    #[cfg(target_os = "linux")]
    {
        // prefer io_uring, then epoll, then poll on Linux
        if let Some(poller) = try_open_host_poller(PollerBackend::IoUring)? {
            return Ok(poller);
        }
        if let Some(poller) = try_open_host_poller(PollerBackend::Epoll)? {
            return Ok(poller);
        }
        let poller = open_host_poller(PollerBackend::Poll)?;

        Ok(HostPollerInstance::new(PollerBackend::Poll, poller))
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    ))]
    {
        // prefer kqueue, then poll on BSD family targets
        if let Some(poller) = try_open_host_poller(PollerBackend::Kqueue)? {
            return Ok(poller);
        }
        let poller = open_host_poller(PollerBackend::Poll)?;

        Ok(HostPollerInstance::new(PollerBackend::Poll, poller))
    }

    #[cfg(windows)]
    {
        // use the Windows backend on Windows hosts
        let poller = open_host_poller(PollerBackend::Windows)?;

        Ok(HostPollerInstance::new(PollerBackend::Windows, poller))
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
        poller_instance_not_supported(PollerBackend::Auto)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use super::*;

    /// Reject runtime poll backend `poll` on Windows hosts.
    #[cfg(windows)]
    #[test]
    fn test_runtime_poll_backend_poll_is_not_supported_on_windows() {
        let result = create_host_poller(PollerBackend::Poll);
        assert!(result.is_err());
        let error = result.err().expect("poll backend should fail");
        let host = error
            .host_error()
            .expect("error should contain one host error");
        assert_eq!(host.code, HostErrorCode::NotSupported);
    }

    /// Wake one blocking io poller wait through one shared wake handle.
    #[test]
    fn test_io_poller_wake_handle_interrupts_blocking_poll() {
        let poller =
            create_host_poller(PollerBackend::Auto).expect("auto poller should initialize");
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
