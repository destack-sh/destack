use serde::{Deserialize, Serialize};

use crate::platform::os::abi_generated::{
    BackgroundEventValue, LocationSampleValue, MediaAssetSummaryValue, NotificationEventValue,
};

/// Host lifecycle state.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HostLifecycleState {
    /// Runtime has not received start events yet.
    Initializing,
    /// Runtime is active and can process host interactions.
    Running,
    /// Runtime is paused by the host.
    Paused,
    /// Runtime is stopped by the host.
    Stopped,
    /// Runtime host process is being destroyed.
    Destroyed,
}

/// Host memory pressure state.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HostMemoryPressureLevel {
    /// Memory pressure is normal.
    Normal,
    /// Memory pressure is elevated.
    Warning,
    /// Memory pressure is critical.
    Critical,
}

/// Host thermal state.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HostThermalState {
    /// Thermal state is nominal.
    Nominal,
    /// Thermal state is fair.
    Fair,
    /// Thermal state is serious.
    Serious,
    /// Thermal state is critical.
    Critical,
}

/// Host power mode state.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HostPowerMode {
    /// Normal power mode.
    Normal,
    /// Low power mode.
    LowPower,
}

/// Host semantic event kind key for scheduler watches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HostEventKind {
    /// Lifecycle transitions.
    Lifecycle,
    /// Intent activation or share ingress events.
    Intent,
    /// Background task readiness or expiration events.
    Background,
    /// Notification delivery or interaction events.
    Notification,
    /// Location watch sample events.
    Location,
    /// Media watch add or update or remove events.
    Media,
    /// Permission result events.
    Permission,
    /// Interruption events.
    Interruption,
    /// Memory pressure state events.
    MemoryPressure,
    /// Thermal state events.
    ThermalState,
    /// Power mode events.
    PowerMode,
    /// Wall clock change events.
    WallClock,
}

/// Runtime-visible host event payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostEvent {
    /// Host lifecycle transition event.
    Lifecycle(HostLifecycleEvent),
    /// Host intent activation or share ingress event.
    Intent(HostIntentEvent),
    /// Host background task readiness or expiration event.
    Background(Box<HostBackgroundEvent>),
    /// Host notification delivery or interaction event.
    Notification(Box<HostNotificationEvent>),
    /// Host location watch sample event.
    Location(Box<HostLocationEvent>),
    /// Host media watch asset event.
    Media(Box<HostMediaEvent>),
    /// Host permission flow result event.
    Permission(HostPermissionEvent),
    /// Host interruption event.
    Interruption(HostInterruptionEvent),
    /// Host memory pressure state event.
    MemoryPressure(HostMemoryPressureEvent),
    /// Host thermal state event.
    ThermalState(HostThermalEvent),
    /// Host power mode event.
    PowerMode(HostPowerModeEvent),
    /// Host wall clock change event.
    WallClock(HostWallClockEvent),
}

impl HostEvent {
    /// Return the host semantic event kind for this event.
    pub const fn kind(&self) -> HostEventKind {
        match self {
            HostEvent::Lifecycle(_) => HostEventKind::Lifecycle,
            HostEvent::Intent(_) => HostEventKind::Intent,
            HostEvent::Background(_) => HostEventKind::Background,
            HostEvent::Notification(_) => HostEventKind::Notification,
            HostEvent::Location(_) => HostEventKind::Location,
            HostEvent::Media(_) => HostEventKind::Media,
            HostEvent::Permission(_) => HostEventKind::Permission,
            HostEvent::Interruption(_) => HostEventKind::Interruption,
            HostEvent::MemoryPressure(_) => HostEventKind::MemoryPressure,
            HostEvent::ThermalState(_) => HostEventKind::ThermalState,
            HostEvent::PowerMode(_) => HostEventKind::PowerMode,
            HostEvent::WallClock(_) => HostEventKind::WallClock,
        }
    }
}

/// Host lifecycle state change payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostLifecycleEvent {
    /// Next lifecycle state after this transition.
    pub state: HostLifecycleState,
}

/// Host intent ingress payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostIntentEvent {
    /// Source package, bundle, or process identifier when available.
    pub source: Option<String>,
    /// Intent payload delivered by the host.
    pub payload: HostIntentPayload,
}

/// Host notification ingress payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostNotificationEvent {
    /// Notification payload delivered by the host.
    pub event: NotificationEventValue,
}

impl Eq for HostNotificationEvent {}

/// Host background-task ingress payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostBackgroundEvent {
    /// Background-task payload delivered by the host.
    pub event: BackgroundEventValue,
}

impl Eq for HostBackgroundEvent {}

/// Host location watch sample payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostLocationEvent {
    /// Runtime-scoped watch identifier.
    pub watch_id: String,
    /// Location sample delivered by the host.
    pub sample: LocationSampleValue,
}

impl Eq for HostLocationEvent {}

/// Host media watch event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostMediaEventKind {
    /// One media asset was added.
    Added,
    /// One media asset was updated.
    Updated,
    /// One media asset was removed.
    Removed,
}

/// Host media watch event payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostMediaEvent {
    /// Runtime-scoped watch identifier.
    pub watch_id: String,
    /// Host media watch event kind.
    pub kind: HostMediaEventKind,
    /// Media asset summary delivered by the host.
    pub asset: MediaAssetSummaryValue,
}

impl Eq for HostMediaEvent {}

/// Host intent ingress variants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostIntentPayload {
    /// Host requested that one URL be opened.
    OpenUrl {
        /// URL payload from the host.
        url: String,
    },
    /// Host requested that one file be opened.
    OpenFile {
        /// Path payload from the host.
        path: String,
        /// MIME type when provided by the host.
        mime_type: Option<String>,
    },
    /// Host delivered one shared text payload.
    ShareText {
        /// Shared text payload from the host.
        text: String,
        /// MIME type when provided by the host.
        mime_type: Option<String>,
    },
    /// Host delivered one shared file list.
    ShareFiles {
        /// Shared file path payloads from the host.
        paths: Vec<String>,
        /// MIME type when provided by the host.
        mime_type: Option<String>,
    },
    /// Host delivered one custom action payload.
    CustomAction {
        /// Action identifier from the host.
        action: String,
        /// URL payload when provided by the host.
        url: Option<String>,
        /// File path payloads when provided by the host.
        paths: Vec<String>,
        /// Shared text payload when provided by the host.
        text: Option<String>,
        /// MIME type when provided by the host.
        mime_type: Option<String>,
    },
}

/// Host permission flow payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostPermissionEvent {
    /// Permission name associated with this result.
    pub permission: String,
    /// Whether the permission was granted.
    pub granted: bool,
}

/// Host interruption payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostInterruptionEvent {
    /// Whether the host is currently interrupted.
    pub interrupted: bool,
}

/// Host memory pressure payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostMemoryPressureEvent {
    /// Memory pressure level reported by the host.
    pub level: HostMemoryPressureLevel,
}

/// Host thermal payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostThermalEvent {
    /// Thermal state reported by the host.
    pub state: HostThermalState,
}

/// Host power mode payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostPowerModeEvent {
    /// Power mode reported by the host.
    pub mode: HostPowerMode,
}

/// Host wall clock payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostWallClockEvent;
