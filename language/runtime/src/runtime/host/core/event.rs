use super::service::HostLifecycleState;
use crate::runtime::poller::PollerEvent;

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
    WindowAvailable,
    /// Host window was torn down.
    WindowTerminated,
    /// Host window dimensions changed.
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
