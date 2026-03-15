#![cfg_attr(not(any(test, unix, windows)), allow(unused_imports))]

mod backend;
pub(crate) mod error;
mod event;
pub(crate) mod observer;
mod queue;
pub(crate) mod registry;
mod runtime;

pub(crate) use backend::HostBackend;
pub use backend::HostPollOutcome;
pub(crate) use destack_workspace::Platform;
pub use event::{
    HostEvent, HostEventKind, HostInterruptionEvent, HostLifecycleEvent, HostLifecycleState,
    HostMemoryPressureEvent, HostMemoryPressureLevel, HostPermissionEvent, HostPowerMode,
    HostPowerModeEvent, HostThermalEvent, HostThermalState, HostWallClockEvent,
};
pub(crate) use queue::HostQueue;
pub(crate) use registry::HostQueueRegistry;
pub use runtime::Host;
