use super::PollerEvent;
use crate::diagnostic::RuntimeResult;
use crate::host::ResourceId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Kernel user-data value reserved for poller wake events.
pub(crate) const WAKE_TOKEN_BITS: u64 = u64::MAX;
/// Kernel user-data value reserved for poller timeout events.
pub(crate) const TIMEOUT_TOKEN_BITS: u64 = u64::MAX - 1;

/// Opaque token used by the poller for event routing.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PollerToken(
    /// Raw token value.
    pub u64,
);

impl PollerToken {
    /// Return whether this token collides with host-poller control events.
    pub const fn is_internal(self) -> bool {
        matches!(self.0, WAKE_TOKEN_BITS | TIMEOUT_TOKEN_BITS)
    }
}

/// Opaque host handle for poller registration.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostHandle(
    /// Raw handle value.
    pub u64,
);

#[cfg(unix)]
impl HostHandle {
    /// Create a host handle from a raw file descriptor.
    pub fn from_raw_fd(fd: std::os::unix::io::RawFd) -> Self {
        Self(fd as u64)
    }

    /// Return the raw file descriptor for this handle.
    pub fn as_raw_fd(self) -> std::os::unix::io::RawFd {
        self.0 as std::os::unix::io::RawFd
    }
}

#[cfg(windows)]
impl HostHandle {
    /// Create a host handle from a raw socket.
    pub fn from_raw_socket(socket: std::os::windows::io::RawSocket) -> Self {
        Self(socket)
    }

    /// Return the raw socket for this handle.
    pub fn as_raw_socket(self) -> std::os::windows::io::RawSocket {
        self.0 as std::os::windows::io::RawSocket
    }
}

/// Interest mask for poller registrations.
///
/// Pollers typically register readability/writability.
/// Other events are emitted based on the resource type.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PollInterest(
    /// Raw interest mask.
    pub u32,
);

impl PollInterest {
    /// No interests.
    pub const NONE: Self = Self(0);
    /// Interested in readable events.
    pub const READABLE: Self = Self(1 << 0);
    /// Interested in writable events.
    pub const WRITABLE: Self = Self(1 << 1);

    /// Return whether the interest mask is empty.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Return whether the interest mask contains the given flags.
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl std::ops::BitOr for PollInterest {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for PollInterest {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Poller configuration flags.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostPollerFlags(
    /// Raw poller flag bits.
    pub u32,
);

impl HostPollerFlags {
    /// No flags.
    pub const NONE: Self = Self(0);
    /// Use edge triggered semantics.
    pub const EDGE: Self = Self(1 << 0);
    /// Use oneshot semantics.
    pub const ONESHOT: Self = Self(1 << 1);
    /// Prefer priority events.
    pub const PRIORITY: Self = Self(1 << 2);

    /// Return whether the flag set is empty.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Return whether the flag set contains the given flags.
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl std::ops::BitOr for HostPollerFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for HostPollerFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Shared wake handle for out-of-band poller wakeups.
pub trait PollerWakeHandle: Send + Sync {
    /// Wake the poller if it is blocked.
    fn wake(&self) -> RuntimeResult<()>;
}

/// Host poller interface for OS-level events.
pub trait HostPoller: Send {
    /// Register a resource handle with the poller.
    fn register(
        &mut self,
        resource_id: ResourceId,
        handle: HostHandle,
        token: PollerToken,
        interests: PollInterest,
        flags: HostPollerFlags,
    ) -> RuntimeResult<()>;

    /// Update the interest mask for a resource.
    fn update(
        &mut self,
        resource_id: ResourceId,
        token: PollerToken,
        interests: PollInterest,
        flags: HostPollerFlags,
    ) -> RuntimeResult<()>;

    /// Remove a resource from the poller.
    fn deregister(&mut self, resource_id: ResourceId) -> RuntimeResult<()>;

    /// Return one shared wake handle for out-of-band wakeups.
    fn wake_handle(&self) -> Option<Arc<dyn PollerWakeHandle>> {
        None
    }

    /// Wake the poller if it is blocked.
    fn wake(&mut self) -> RuntimeResult<()>;

    /// Poll for host events, optionally bounded by a timeout in nanoseconds.
    fn poll(&mut self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<PollerEvent>>;
}
