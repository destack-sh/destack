mod adapter;
pub(crate) mod error;
mod event;
mod queue;
pub(crate) mod registry;
pub(crate) mod request;
mod runtime;
mod status;

pub(crate) use adapter::HostAdapter;
pub use adapter::HostPollOutcome;
pub(crate) use destack_workspace::Platform;
pub use event::{
    HostBackgroundEvent, HostEvent, HostEventKind, HostIntentEvent, HostIntentPayload,
    HostInterruptionEvent, HostLifecycleEvent, HostLifecycleState, HostLocationEvent,
    HostMemoryPressureEvent, HostMemoryPressureLevel, HostNotificationEvent, HostPermissionEvent,
    HostPowerMode, HostPowerModeEvent, HostThermalEvent, HostThermalState, HostWallClockEvent,
};
pub(crate) use queue::HostQueue;
pub(crate) use registry::{
    HostEventObserver, HostRuntimeId, HostRuntimeRegistry, RuntimeIngressHandler,
};
pub(crate) use request::{HostRequest, HostRequestContext, HostRequestOutcome, HostRequestResult};
pub use runtime::HostSession;
pub use status::{
    HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_FAILED, HOST_STATUS_INVALID_ARGUMENT,
    HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
    HOST_STATUS_PERMISSION_DENIED,
};
