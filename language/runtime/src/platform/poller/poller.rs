use super::PlatformEvent;
use crate::diagnostic::RuntimeResult;
use crate::platform::ResourceId;

/// Opaque platform handle for poller registration.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformHandle(
    /// Raw handle value.
    pub u64,
);

#[cfg(unix)]
impl PlatformHandle {
    /// Create a platform handle from a raw file descriptor.
    pub fn from_raw_fd(fd: std::os::unix::io::RawFd) -> Self {
        Self(fd as u64)
    }

    /// Return the raw file descriptor for this handle.
    pub fn as_raw_fd(self) -> std::os::unix::io::RawFd {
        self.0 as std::os::unix::io::RawFd
    }
}

#[cfg(windows)]
impl PlatformHandle {
    /// Create a platform handle from a raw socket.
    pub fn from_raw_socket(socket: std::os::windows::io::RawSocket) -> Self {
        Self(socket as u64)
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
pub struct PlatformInterest(
    /// Raw interest mask.
    pub u32,
);

impl PlatformInterest {
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

impl std::ops::BitOr for PlatformInterest {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for PlatformInterest {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Poller configuration flags.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformPollerFlags(
    /// Raw poller flag bits.
    pub u32,
);

impl PlatformPollerFlags {
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

impl std::ops::BitOr for PlatformPollerFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for PlatformPollerFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Platform poller interface for OS-level events.
pub trait PlatformPoller: Send {
    /// Register a resource handle with the poller.
    fn register(
        &mut self,
        resource_id: ResourceId,
        handle: PlatformHandle,
        token: u64,
        interests: PlatformInterest,
        flags: PlatformPollerFlags,
    ) -> RuntimeResult<()>;

    /// Update the interest mask for a resource.
    fn update(
        &mut self,
        resource_id: ResourceId,
        token: u64,
        interests: PlatformInterest,
        flags: PlatformPollerFlags,
    ) -> RuntimeResult<()>;

    /// Remove a resource from the poller.
    fn deregister(&mut self, resource_id: ResourceId) -> RuntimeResult<()>;

    /// Wake the poller if it is blocked.
    fn wake(&mut self) -> RuntimeResult<()>;

    /// Poll for platform events, optionally bounded by a timeout in nanoseconds.
    fn poll(&mut self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<PlatformEvent>>;
}
