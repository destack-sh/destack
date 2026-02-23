use super::service::HostLifecycleState;
use crate::runtime::poller::PollerEvent;

/// Host semantic event kind key for scheduler watches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostEventKind {
    /// Lifecycle transitions.
    Lifecycle,
    /// Window and surface events.
    Window,
    /// Permission result events.
    Permission,
    /// Interruption events.
    Interruption,
}

/// Runtime-visible host event payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostEvent {
    /// Event routed from the runtime poller layer.
    Poller(PollerEvent),
    /// Host lifecycle transition event.
    Lifecycle(HostLifecycleEvent),
    /// Host window and surface event.
    Window(HostWindowEvent),
    /// Host permission flow result event.
    Permission(HostPermissionEvent),
    /// Host interruption event.
    Interruption(HostInterruptionEvent),
}

impl HostEvent {
    /// Return the host semantic event kind for this event when one exists.
    pub const fn kind(&self) -> Option<HostEventKind> {
        match self {
            HostEvent::Poller(_) => None,
            HostEvent::Lifecycle(_) => Some(HostEventKind::Lifecycle),
            HostEvent::Window(_) => Some(HostEventKind::Window),
            HostEvent::Permission(_) => Some(HostEventKind::Permission),
            HostEvent::Interruption(_) => Some(HostEventKind::Interruption),
        }
    }

    /// Return whether this event must be handled losslessly.
    pub const fn is_lossless(&self) -> bool {
        matches!(self.kind(), Some(HostEventKind::Permission))
    }

    /// Return whether this event uses latest-state coalescing semantics.
    pub const fn is_coalescing(&self) -> bool {
        matches!(
            self.kind(),
            Some(HostEventKind::Lifecycle | HostEventKind::Window | HostEventKind::Interruption)
        )
    }
}

/// Host lifecycle state change payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostLifecycleEvent {
    /// Next lifecycle state after this transition.
    pub state: HostLifecycleState,
}

/// Host window and surface payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostWindowEvent {
    /// Host window became available.
    /// On Android this aligns to surface creation events such as `InitWindow`.
    WindowAvailable,
    /// Host window was torn down.
    /// On Android this aligns to surface teardown events such as `TerminateWindow`.
    WindowTerminated,
    /// Host window dimensions changed.
    /// This aligns to window resize events in common host event systems.
    WindowResized {
        /// Window width in physical pixels.
        width_px: u32,
        /// Window height in physical pixels.
        height_px: u32,
    },
}

/// Host permission flow payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostPermissionEvent {
    /// Permission name associated with this result.
    pub permission: String,
    /// Whether the permission was granted.
    pub granted: bool,
}

/// Host interruption payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostInterruptionEvent {
    /// Whether the host is currently interrupted.
    pub interrupted: bool,
}
