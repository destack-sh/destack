use crate::platform::ResourceId;

/// Flags attached to poller events.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformEventFlags(
    /// Raw flag bits.
    pub u32,
);

impl PlatformEventFlags {
    /// No flags.
    pub const NONE: Self = Self(0);
    /// Event was edge triggered.
    pub const EDGE: Self = Self(1 << 0);
    /// Event should be delivered once.
    pub const ONESHOT: Self = Self(1 << 1);

    /// Return whether the flag set is empty.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Return whether the flag set contains the given flags.
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl std::ops::BitOr for PlatformEventFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for PlatformEventFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Bitmask describing the event state.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformEventMask(
    /// Raw mask bits.
    pub u32,
);

impl PlatformEventMask {
    /// No event bits.
    pub const NONE: Self = Self(0);
    /// Resource is readable.
    pub const READABLE: Self = Self(1 << 0);
    /// Resource is writable.
    pub const WRITABLE: Self = Self(1 << 1);
    /// Resource reported an error condition.
    pub const ERROR: Self = Self(1 << 2);
    /// Resource was closed or hung up.
    pub const HANGUP: Self = Self(1 << 3);
    /// Resource reported priority data.
    pub const PRIORITY: Self = Self(1 << 4);

    /// Return whether the mask is empty.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Return whether the mask contains the given bits.
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl std::ops::BitOr for PlatformEventMask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for PlatformEventMask {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Source category for the event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformEventSource {
    /// Event originated from I/O readiness.
    Io,
    /// Event originated from a signal watch.
    Signal,
    /// Event originated from a process watch.
    Process,
    /// Event originated from a timer watch.
    Timer,
}

/// Payload data attached to an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformEventPayload {
    /// I/O readiness payload from the underlying poller.
    Io {
        /// Raw event bits from the platform poller.
        data: u64,
    },
    /// Signal delivery payload.
    Signal {
        /// Signal number.
        signal: u32,
    },
    /// Process state change payload.
    Process {
        /// Process id.
        pid: u32,
        /// Process status information.
        status: ProcessStatus,
    },
    /// Timer expiration payload.
    Timer {
        /// Deadline associated with the timer, in nanoseconds.
        deadline_nanos: u64,
    },
}

/// Process termination or state change status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessStatus {
    /// Process exited normally with a code.
    Exited {
        /// Exit status code.
        code: i32,
    },
    /// Process terminated due to a signal.
    Signaled {
        /// Signal number.
        signal: u32,
        /// Whether a core dump was produced.
        core_dump: bool,
    },
    /// Process stopped by a signal.
    Stopped {
        /// Signal number.
        signal: u32,
    },
    /// Process resumed after being stopped.
    Continued,
}

/// Platform event emitted by the poller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformEvent {
    /// Resource associated with the event.
    pub resource_id: ResourceId,
    /// Event source category.
    pub source: PlatformEventSource,
    /// Event state mask.
    pub mask: PlatformEventMask,
    /// Event flags associated with this event.
    pub flags: PlatformEventFlags,
    /// Opaque user token from registration.
    pub token: u64,
    /// Payload information associated with the event.
    pub payload: PlatformEventPayload,
}
