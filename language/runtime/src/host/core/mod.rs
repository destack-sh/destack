mod backend;
mod capability;
pub(crate) mod error;
mod event;
pub(crate) mod observer;
mod queue;
pub(crate) mod registry;
mod runtime;
mod select;
mod state;

pub(crate) use backend::HostBackend;
pub use backend::HostPollOutcome;
pub(crate) use destack_workspace::Platform;
pub use event::{
    HostEvent, HostEventKind, HostInterruptionEvent, HostLifecycleEvent, HostLifecycleState,
    HostMemoryPressureEvent, HostMemoryPressureLevel, HostPermissionEvent, HostPowerMode,
    HostPowerModeEvent, HostThermalEvent, HostThermalState, HostWallClockEvent,
};
pub(crate) use queue::HostEventQueue;
pub use runtime::Host;
pub(crate) use state::HostState;
