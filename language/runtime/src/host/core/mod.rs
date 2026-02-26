mod adapter;
mod bridge;
mod capability;
mod event;
mod queue;
mod registry;
mod runtime;
mod select;
mod service;
mod state;

pub use adapter::{Host, HostPlatform, HostPollOutcome};
pub(crate) use bridge::HostBridge;
pub(crate) use capability::default_host_capabilities;
pub use event::{
    HostEvent, HostEventKind, HostInterruptionEvent, HostLifecycleEvent, HostMemoryPressureEvent,
    HostPermissionEvent, HostPowerModeEvent, HostThermalEvent, HostWallClockEvent, HostWindowEvent,
    HostWindowFocusEvent,
};
pub(crate) use queue::HostEventQueue;
#[allow(unused_imports)]
pub(crate) use registry::{HostBridgeRegistration, host_bridge_for_runtime, register_host_bridge};
pub use runtime::HostRuntime;
pub use select::default_host;
pub use service::{HostLifecycleState, HostMemoryPressureLevel, HostPowerMode, HostThermalState};
pub use state::HostState;
