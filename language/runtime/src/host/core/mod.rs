mod adapter;
mod bridge;
mod event;
mod queue;
mod registry;
mod runtime;
mod select;
mod service;
mod state;

pub use adapter::{HostAdapter, HostPlatform};
pub(crate) use bridge::HostBridge;
pub use event::{
    HostEvent, HostEventKind, HostInterruptionEvent, HostLifecycleEvent, HostMemoryPressureEvent,
    HostPermissionEvent, HostPowerModeEvent, HostThermalEvent, HostWallClockEvent, HostWindowEvent,
    HostWindowFocusEvent,
};
pub(crate) use queue::HostEventQueue;
#[allow(unused_imports)]
pub(crate) use registry::{HostBridgeRegistration, host_bridge_for_runtime, register_host_bridge};
pub use runtime::HostRuntime;
pub use select::default_host_adapter;
pub use service::{
    HostLifecycleState, HostMemoryPressureLevel, HostPermissionService, HostPowerMode,
    HostServices, HostStateReader, HostThermalState,
};
pub(crate) use state::HostStateStore;
