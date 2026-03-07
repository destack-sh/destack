mod adapter;
mod capability;
mod error;
mod event;
mod ingress;
mod queue;
mod registry;
mod runtime;
mod select;
mod service;
mod state;

pub use adapter::{HostAdapter, HostPlatform, HostPollOutcome};
pub(crate) use capability::default_host_capabilities;
#[allow(unused_imports)]
pub(crate) use error::{invalid_argument_value, missing_host_state, not_supported};
pub use event::{
    HostEvent, HostEventKind, HostInterruptionEvent, HostLifecycleEvent, HostMemoryPressureEvent,
    HostPermissionEvent, HostPowerModeEvent, HostThermalEvent, HostWallClockEvent, HostWindowEvent,
    HostWindowFocusEvent,
};
pub(crate) use ingress::{
    RuntimeIngressObserver, cleanup_runtime_ingress_observers, process_runtime_ingress_observer,
    process_runtime_ingress_observers, register_runtime_ingress_observer,
};
pub(crate) use queue::HostEventQueue;
#[allow(unused_imports)]
pub(crate) use registry::{
    HostStateCleanup, HostStateRegistration, host_state_for_runtime, register_host_state,
};
pub use runtime::Host;
pub use select::default_host;
pub use service::{HostLifecycleState, HostMemoryPressureLevel, HostPowerMode, HostThermalState};
pub(crate) use state::HostAdapterState;
pub use state::HostState;
