#![allow(dead_code, unused_imports)]
#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

pub mod abi;
pub(crate) mod core;
pub(crate) mod operation;
pub mod os;
pub(crate) mod policy;

pub(crate) use crate::host::abi::core::HostSessionHandle;
#[cfg(any(test, target_os = "android", target_os = "ios"))]
pub(crate) use crate::host::abi::core::HostStatus;
pub(crate) use destack_artifact::Platform;

pub(crate) use core::adapter::HostAdapter;
pub use core::adapter::PollResult;

#[cfg(any(test, target_os = "android"))]
pub use os::android;
#[cfg(target_os = "ios")]
pub use os::apple;

#[cfg(any(test, target_os = "android", target_os = "ios"))]
pub(crate) use core::event::host_intent_event_from_value;
pub use core::event::{
    HostBackgroundEvent, HostEvent, HostEventKind, HostIntentEvent, HostIntentPayload,
    HostInterruptionEvent, HostLifecycleEvent, HostLifecycleSourceKind, HostLifecycleState,
    HostLocationEvent, HostMemoryPressureEvent, HostMemoryPressureLevel, HostNotificationEvent,
    HostPermissionEvent, HostPowerMode, HostPowerModeEvent, HostRequestCompletionEvent,
    HostTextEvent, HostThermalEvent, HostThermalState, HostWallClockEvent,
};

pub(crate) use core::queue::HostQueue;
pub(crate) use core::registry::{
    HostEventObserver, HostSessionId, HostSessionRegistry, RuntimeIngressHandler,
};
pub(crate) use core::request::{
    HostRequest, HostRequestCompletion, HostRequestOutcome, RequestContext, SessionContext,
};
pub use core::request::{HostRequestId, HostRequestResult};
pub use core::session::Session;
pub use core::status::{
    HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_FAILED, HOST_STATUS_INVALID_ARGUMENT,
    HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
    HOST_STATUS_PERMISSION_DENIED,
};
#[cfg(windows)]
pub(crate) use os::windows::ingress::run_ingress_loop;
