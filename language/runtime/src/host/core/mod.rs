#![cfg_attr(not(any(test, unix, windows)), allow(unused_imports))]

mod backend;
pub(crate) mod error;
mod event;
pub(crate) mod observer;
mod queue;
pub(crate) mod registry;
mod runtime;
mod status;

pub(crate) use backend::HostBackend;
pub use backend::HostPollOutcome;
pub(crate) use destack_workspace::Platform;
pub use event::{
    HostEvent, HostEventKind, HostIntentEvent, HostIntentPayload, HostInterruptionEvent,
    HostLifecycleEvent, HostLifecycleState, HostMemoryPressureEvent, HostMemoryPressureLevel,
    HostPermissionEvent, HostPowerMode, HostPowerModeEvent, HostThermalEvent, HostThermalState,
    HostWallClockEvent,
};
pub(crate) use queue::HostQueue;
pub(crate) use registry::HostQueueRegistry;
pub use runtime::Host;
pub use status::{
    HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_FAILED, HOST_STATUS_INVALID_ARGUMENT,
    HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
    HOST_STATUS_PERMISSION_DENIED,
};
