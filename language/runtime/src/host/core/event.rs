use serde::{Deserialize, Serialize};

use super::request::HostRequestId;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::fs::abi_generated::OsPathValue;
use crate::platform::input::InputTextSessionStateValue;
use crate::platform::os::Permission;
use crate::platform::os::abi_generated::{
    BackgroundEventValue, DocumentDescriptorValue, IntentEventValue, LocationSampleValue,
    NotificationEventValue,
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

/// Host lifecycle source attachment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HostLifecycleSourceKind {
    /// Event originated from one application attachment.
    Application,
    /// Event originated from one activity attachment.
    Activity,
    /// Event originated from one scene attachment.
    Scene,
    /// Event originated from one window attachment.
    Window,
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

/// Host semantic event kind for queue coalescing and routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HostEventKind {
    /// Lifecycle transitions.
    Lifecycle,
    /// Intent activation or share ingress events.
    Intent,
    /// Background task readiness or expiration events.
    Background,
    /// Document picker completion events.
    Document,
    /// Notification delivery or interaction events.
    Notification,
    /// Location watch sample events.
    Location,
    /// Permission result events.
    Permission,
    /// Text session state events.
    Text,
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

/// Runtime-visible host ingress payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostEvent {
    /// Host lifecycle transition event.
    Lifecycle(HostLifecycleEvent),
    /// Host intent activation or share ingress event.
    Intent(HostIntentEvent),
    /// Host background task readiness or expiration event.
    Background(Box<HostBackgroundEvent>),
    /// Host document picker completion event.
    Document(Box<HostDocumentEvent>),
    /// Host notification delivery or interaction event.
    Notification(Box<HostNotificationEvent>),
    /// Host location watch sample event.
    Location(Box<HostLocationEvent>),
    /// Host permission flow result event.
    Permission(HostPermissionEvent),
    /// Host text session state event.
    Text(Box<HostTextEvent>),
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
            HostEvent::Document(_) => HostEventKind::Document,
            HostEvent::Notification(_) => HostEventKind::Notification,
            HostEvent::Location(_) => HostEventKind::Location,
            HostEvent::Permission(_) => HostEventKind::Permission,
            HostEvent::Text(_) => HostEventKind::Text,
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
    /// Source attachment that originated this transition.
    pub source_kind: HostLifecycleSourceKind,
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

/// Host document picker completion payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostDocumentEvent {
    /// Stable request identifier for the interactive document flow.
    pub request_id: HostRequestId,
    /// Selected document descriptors returned by the host.
    pub documents: Vec<DocumentDescriptorValue>,
}

impl Eq for HostDocumentEvent {}

/// Host location watch sample payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostLocationEvent {
    /// Runtime-scoped watch identifier.
    pub watch_id: String,
    /// Location sample delivered by the host.
    pub sample: LocationSampleValue,
}

impl Eq for HostLocationEvent {}

/// Host text session state payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostTextEvent {
    /// Runtime-scoped text session identifier.
    pub session_id: u64,
    /// Host-authoritative text session state.
    pub state: InputTextSessionStateValue,
}

impl Eq for HostTextEvent {}

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
        /// Normalized content type when provided by the host.
        content_type: Option<String>,
    },
    /// Host delivered one shared text payload.
    ShareText {
        /// Shared text payload from the host.
        text: String,
        /// Normalized content type when provided by the host.
        content_type: Option<String>,
    },
    /// Host delivered one shared file list.
    ShareFiles {
        /// Shared file path payloads from the host.
        paths: Vec<String>,
        /// Normalized content type when provided by the host.
        content_type: Option<String>,
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
        /// Normalized content type when provided by the host.
        content_type: Option<String>,
    },
}

/// Decode one semantic path value into one queue-owned utf8 path string.
fn utf8_path_from_value(value: OsPathValue, label: &'static str) -> RuntimeResult<String> {
    #[cfg(unix)]
    {
        match value {
            OsPathValue::OsPathBytes(value) => String::from_utf8(value.bytes.0).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    label,
                    "path bytes are not valid utf8",
                ))
                .boxed()
            }),
            OsPathValue::OsPathUtf16(value) => String::from_utf16(&value.utf16.0).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    label,
                    "path utf16 is not valid",
                ))
                .boxed()
            }),
        }
    }

    #[cfg(windows)]
    {
        match value {
            OsPathValue::OsPathBytes(value) => String::from_utf8(value.bytes.0).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    label,
                    "path bytes are not valid utf8",
                ))
                .boxed()
            }),
            OsPathValue::OsPathUtf16(value) => Ok(String::from_utf16_lossy(&value.utf16.0)),
        }
    }
}

/// Build one core host intent event from one semantic intent value payload.
pub(crate) fn host_intent_event_from_value(
    event: IntentEventValue,
) -> RuntimeResult<HostIntentEvent> {
    // decode one semantic intent payload into the queue-facing host event
    let (source, payload) = match event {
        IntentEventValue::IntentOpenUrlEvent(value) => (
            value.metadata.source,
            HostIntentPayload::OpenUrl {
                url: value.payload.url,
            },
        ),
        IntentEventValue::IntentOpenFileEvent(value) => {
            let path = utf8_path_from_value(value.payload.path, "path")?;

            (
                value.metadata.source,
                HostIntentPayload::OpenFile {
                    path,
                    content_type: value.payload.mime_type,
                },
            )
        }
        IntentEventValue::IntentShareTextEvent(value) => (
            value.metadata.source,
            HostIntentPayload::ShareText {
                text: value.payload.text,
                content_type: value.payload.mime_type,
            },
        ),
        IntentEventValue::IntentShareFilesEvent(value) => {
            // lower each shared path into one queue-owned utf8 string
            let mut paths = Vec::with_capacity(value.payload.paths.len());

            for path in value.payload.paths {
                let path = utf8_path_from_value(path, "paths")?;
                paths.push(path);
            }

            (
                value.metadata.source,
                HostIntentPayload::ShareFiles {
                    paths,
                    content_type: value.payload.mime_type,
                },
            )
        }
        IntentEventValue::IntentCustomActionEvent(value) => {
            // lower each shared path into one queue-owned utf8 string
            let mut paths = Vec::with_capacity(value.payload.paths.len());

            for path in value.payload.paths {
                let path = utf8_path_from_value(path, "paths")?;
                paths.push(path);
            }

            (
                value.metadata.source,
                HostIntentPayload::CustomAction {
                    action: value.payload.action,
                    url: value.payload.url,
                    paths,
                    text: value.payload.text,
                    content_type: value.payload.mime_type,
                },
            )
        }
    };

    Ok(HostIntentEvent { source, payload })
}

/// Host permission flow payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostPermissionEvent {
    /// Stable request identifier when this result completes one explicit host request.
    pub request_id: Option<HostRequestId>,
    /// Permission selector associated with this result.
    pub permission: Permission,
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
