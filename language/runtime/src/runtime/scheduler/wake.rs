use serde::{Deserialize, Serialize};

use crate::host::poller::{PollerEvent, PollerEventMask};
use crate::host::{HostEvent, HostEventKind, ResourceId};

/// Runtime wake consumed by the event loop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Wake {
    /// Timer wake produced by a runtime timer resource.
    Timer(TimerWake),
    /// Resource wake produced by pollers or simulated resources.
    Resource(ResourceWake),
    /// Host wake produced by the embedding host.
    Host(HostWake),
}

impl Wake {
    /// Return the waiter key associated with this wake.
    pub const fn key(&self) -> WakeKey {
        match self {
            Self::Timer(wake) => WakeKey::Timer(wake.resource_id),
            Self::Resource(wake) => WakeKey::Resource {
                resource_id: wake.resource_id,
                readiness: wake.readiness,
            },
            Self::Host(wake) => WakeKey::Host(wake.event.kind()),
        }
    }
}

/// Timer wake selected by the event loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimerWake {
    /// Timer resource that expired.
    pub resource_id: ResourceId,
}

impl TimerWake {
    /// Build one timer expiration wake.
    pub const fn new(resource_id: ResourceId) -> Self {
        Self { resource_id }
    }
}

/// Resource wake selected by the event loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceWake {
    /// Resource that became ready.
    pub resource_id: ResourceId,
    /// Readiness that became available.
    pub readiness: Readiness,
}

impl ResourceWake {
    /// Build one resource wake from a poller event.
    pub const fn poller(event: PollerEvent) -> Self {
        Self {
            resource_id: event.resource_id,
            readiness: Readiness::from_poller_mask(event.mask),
        }
    }
}

/// Host wake selected by the event loop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostWake {
    /// Host event payload.
    pub event: HostEvent,
}

impl HostWake {
    /// Build one host wake.
    pub const fn new(event: HostEvent) -> Self {
        Self { event }
    }
}

/// Resource readiness kind used for waiter matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Readiness {
    /// Resource became readable.
    Readable,
    /// Resource became writable.
    Writable,
    /// Resource reported priority data.
    Priority,
    /// Resource status changed.
    Status,
}

impl Readiness {
    /// Return the readiness represented by one poller event mask.
    pub const fn from_poller_mask(mask: PollerEventMask) -> Self {
        if mask.contains(PollerEventMask::READABLE) {
            Self::Readable
        } else if mask.contains(PollerEventMask::WRITABLE) {
            Self::Writable
        } else if mask.contains(PollerEventMask::PRIORITY) {
            Self::Priority
        } else {
            Self::Status
        }
    }
}

/// Source key that can wake one suspended continuation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WakeKey {
    /// Timer resource deadline expired.
    Timer(ResourceId),
    /// Resource readiness became available.
    Resource {
        /// Resource that can resume a waiter.
        resource_id: ResourceId,
        /// Readiness that can resume a waiter.
        readiness: Readiness,
    },
    /// Host event kind became available.
    Host(HostEventKind),
}

/// Result of one runtime scheduler tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickResult {
    /// One unit of runnable work completed.
    Progress,
    /// Virtual time advanced to the next deadline.
    TimeAdvanced,
    /// Background work is still active outside this tick.
    Background,
    /// No runnable work or future deadlines remained.
    Idle,
}

impl TickResult {
    /// Return whether this tick advanced deterministic execution.
    pub const fn is_progress(self) -> bool {
        matches!(self, Self::Progress | Self::TimeAdvanced)
    }
}
