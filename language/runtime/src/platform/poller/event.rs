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
    /// Event has priority.
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

/// Kind of platform event reported by the poller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformEventKind {
    /// Resource is readable.
    Readable,
    /// Resource is writable.
    Writable,
    /// Resource encountered an error.
    Error,
    /// Resource was closed or hung up.
    Closed,
    /// External signal was delivered.
    Signal,
    /// Process exited or changed state.
    ProcessExit,
    /// Timer event fired.
    Timer,
}

/// Platform event emitted by the poller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformEvent {
    /// Resource associated with the event.
    pub resource_id: ResourceId,
    /// Event classification for the resource.
    pub kind: PlatformEventKind,
    /// Event flags associated with this event.
    pub flags: PlatformEventFlags,
    /// Optional data payload or platform specific info.
    pub data: u64,
}
