mod adapter;
#[allow(dead_code)]
pub(crate) mod callback;
pub(crate) mod error;
mod event;
mod queue;
pub(crate) mod registry;
pub(crate) mod request;
mod runtime;
mod status;

#[allow(unused_imports)]
pub(crate) use crate::host::abi::core::{HostSessionHandle, HostStatus};
pub(crate) use adapter::HostAdapter;
pub use adapter::HostPollOutcome;
pub(crate) use destack_artifact::Platform;
#[cfg(any(test, feature = "execution"))]
pub(crate) use adapter::without_native_ingress;
pub use event::{
    HostBackgroundEvent, HostDocumentEvent, HostEvent, HostEventKind, HostIntentEvent,
    HostIntentPayload, HostInterruptionEvent, HostLifecycleEvent, HostLifecycleSourceKind,
    HostLifecycleState, HostLocationEvent, HostMemoryPressureEvent, HostMemoryPressureLevel,
    HostNotificationEvent, HostPermissionEvent, HostPowerMode, HostPowerModeEvent,
    HostThermalEvent, HostThermalState, HostWallClockEvent,
};
pub(crate) use queue::HostQueue;
pub(crate) use registry::{
    HostEventObserver, HostSessionId, HostSessionRegistry, RuntimeIngressHandler,
};
pub use request::HostRequestId;
pub(crate) use request::{
    HostRequest, HostRequestCompletion, HostRequestContext, HostRequestOutcome, HostRequestResult,
    HostSessionContext,
};
pub use runtime::HostSession;
pub use status::{
    HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_FAILED, HOST_STATUS_INVALID_ARGUMENT,
    HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
    HOST_STATUS_PERMISSION_DENIED,
};
