mod adapter;
#[cfg(any(test, target_os = "android", target_os = "macos", windows))]
mod bridge;
mod event;
mod queue;
#[cfg(any(test, target_os = "android", target_os = "macos", windows))]
mod registry;
mod runtime;
mod select;
mod service;
mod state;

pub use adapter::{HostAdapter, HostPlatform};
#[cfg(any(test, target_os = "android", target_os = "macos", windows))]
pub(crate) use bridge::HostBridge;
pub use event::{
    HostEvent, HostEventKind, HostInterruptionEvent, HostLifecycleEvent, HostPermissionEvent,
    HostWindowEvent,
};
pub(crate) use queue::HostEventQueue;
#[cfg(any(test, target_os = "android", target_os = "macos", windows))]
pub(crate) use registry::{HostBridgeRegistration, host_bridge_for_runtime, register_host_bridge};
pub use runtime::HostRuntime;
pub use select::default_host_adapter;
pub use service::{
    HostInterruptionService, HostLifecycleService, HostLifecycleState, HostPermissionService,
    HostServices, HostWindowService,
};
pub(crate) use state::HostServiceState;
