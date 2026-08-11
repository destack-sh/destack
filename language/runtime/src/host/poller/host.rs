use std::fmt;
use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::host::ResourceId;

#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly",
    windows
)))]
use crate::diagnostic::RuntimeError;
#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly",
    windows
)))]
use crate::host::HostError;

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
use crate::host::HostErrorCode;

use super::{
    HostHandle, HostPollerFlags, PollInterest, Poller, PollerEvent, PollerToken, PollerWakeHandle,
};

#[cfg(target_os = "linux")]
use super::{EpollPoller, IoUringPoller};

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

/// One platform host poller.
pub(crate) struct HostPoller {
    /// Platform poller implementation.
    poller: Box<dyn Poller>,
}

impl HostPoller {
    /// Create one host poller.
    fn new(poller: Box<dyn Poller>) -> Self {
        Self { poller }
    }

    /// Open the preferred host poller for this platform.
    pub(crate) fn open() -> RuntimeResult<Self> {
        PollerBackend::open_preferred()
    }
}

impl fmt::Debug for HostPoller {
    /// Format one host poller without traversing backend state.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HostPoller")
            .field("poller", &"<host poller>")
            .finish()
    }
}

impl Poller for HostPoller {
    /// Register one host resource interest.
    fn register(
        &mut self,
        resource_id: ResourceId,
        handle: HostHandle,
        token: PollerToken,
        interests: PollInterest,
        flags: HostPollerFlags,
    ) -> RuntimeResult<()> {
        self.poller
            .register(resource_id, handle, token, interests, flags)
    }

    /// Update one registered host resource interest.
    fn update(
        &mut self,
        resource_id: ResourceId,
        token: PollerToken,
        interests: PollInterest,
        flags: HostPollerFlags,
    ) -> RuntimeResult<()> {
        self.poller.update(resource_id, token, interests, flags)
    }

    /// Remove one registered host resource.
    fn deregister(&mut self, resource_id: ResourceId) -> RuntimeResult<()> {
        self.poller.deregister(resource_id)
    }

    /// Return the handle that wakes one blocking poll.
    fn wake_handle(&self) -> Option<Arc<dyn PollerWakeHandle>> {
        self.poller.wake_handle()
    }

    /// Wake one blocking poll.
    fn wake(&mut self) -> RuntimeResult<()> {
        self.poller.wake()
    }

    /// Poll for ready host events.
    fn poll(&mut self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<PollerEvent>> {
        self.poller.poll(timeout_nanos)
    }
}

/// One concrete host poller backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PollerBackend {
    /// Linux io_uring.
    #[cfg(target_os = "linux")]
    IoUring,
    /// Linux epoll.
    #[cfg(target_os = "linux")]
    Epoll,
    /// BSD or macOS kqueue.
    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    ))]
    Kqueue,
    /// Unix poll.
    #[cfg(unix)]
    Poll,
    /// Windows readiness polling.
    #[cfg(windows)]
    Windows,
}

impl PollerBackend {
    /// Open the preferred host poller for this platform.
    fn open_preferred() -> RuntimeResult<HostPoller> {
        #[cfg(target_os = "linux")]
        {
            // prefer io_uring, then epoll, then poll on Linux
            if let Some(poller) = Self::IoUring.open_maybe()? {
                return Ok(poller);
            }
            if let Some(poller) = Self::Epoll.open_maybe()? {
                return Ok(poller);
            }

            Self::Poll.open()
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
            if let Some(poller) = Self::Kqueue.open_maybe()? {
                return Ok(poller);
            }

            Self::Poll.open()
        }

        #[cfg(windows)]
        {
            Self::Windows.open()
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
            Err(RuntimeError::from(HostError::not_supported(
                "host polling is not supported on this platform".to_string(),
            ))
            .boxed())
        }
    }

    /// Open this exact host poller backend.
    fn open(self) -> RuntimeResult<HostPoller> {
        let poller: Box<dyn Poller> = match self {
            #[cfg(target_os = "linux")]
            Self::IoUring => Box::new(IoUringPoller::new()?),
            #[cfg(target_os = "linux")]
            Self::Epoll => Box::new(EpollPoller::new()?),
            #[cfg(any(
                target_os = "macos",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd",
                target_os = "dragonfly"
            ))]
            Self::Kqueue => Box::new(KqueuePoller::new()?),
            #[cfg(unix)]
            Self::Poll => Box::new(UnixPoller::new()?),
            #[cfg(windows)]
            Self::Windows => Box::new(WindowsPoller::new()?),
        };

        Ok(HostPoller::new(poller))
    }

    /// Try this backend and suppress only an unsupported-backend failure.
    #[cfg(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    ))]
    fn open_maybe(self) -> RuntimeResult<Option<HostPoller>> {
        match self.open() {
            Ok(poller) => Ok(Some(poller)),
            Err(error)
                if error
                    .host_error()
                    .is_some_and(|error| error.code == HostErrorCode::NotSupported) =>
            {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex, mpsc};
    use std::time::{Duration, Instant};

    use super::*;

    /// Wake one blocking io poller wait through one shared wake handle.
    #[test]
    fn test_io_poller_wake_handle_interrupts_blocking_poll() {
        let poller = HostPoller::open().expect("host poller should initialize");
        let wake_handle = poller
            .wake_handle()
            .expect("io poller should expose one wake handle");

        let poller = Arc::new(Mutex::new(poller));
        let poller_for_thread = Arc::clone(&poller);
        let (started, wait_for_start) = mpsc::sync_channel(0);
        let started_at = Instant::now();

        let waiter = std::thread::spawn(move || {
            let mut poller = poller_for_thread
                .lock()
                .expect("poller lock should not be poisoned");
            started.send(()).expect("poll waiter should signal startup");
            poller
                .poll(Some(2_000_000_000))
                .expect("poll should unblock after wake");
        });

        wait_for_start
            .recv()
            .expect("poll waiter should signal startup");
        wake_handle.wake().expect("wake should succeed");
        waiter.join().expect("wait thread should join");

        let elapsed = started_at.elapsed();
        assert!(
            elapsed < Duration::from_millis(1_000),
            "poll wake should interrupt quickly, elapsed={elapsed:?}"
        );
    }
}
