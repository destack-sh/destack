#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
compile_error!("destack_runtime host supports linux, macos, and windows");

pub(crate) mod accessibility;
pub(crate) mod audio;
pub mod binding;
pub mod core;
pub(crate) mod crypto;
pub(crate) mod device;
pub(crate) mod display;
pub(crate) mod fs;
pub(crate) mod gpu;
pub(crate) mod input;
pub(crate) mod io;
pub(crate) mod ipc;
#[cfg(target_os = "linux")]
pub(crate) mod linux;
#[cfg(target_os = "macos")]
pub(crate) mod macos;
pub(crate) mod memory;
pub(crate) mod net;
pub(crate) mod os;
pub(crate) mod poller;
pub(crate) mod process;
pub(crate) mod random;
pub mod resource;
pub mod time;
pub(crate) mod tls;
pub(crate) mod tty;
#[cfg(unix)]
pub(crate) mod unix;
#[cfg(windows)]
pub(crate) mod windows;

pub(crate) use self::core::{Host, compile_target_host};
pub use self::core::{
    HostEvent, HostEventKind, HostPollResult, LifecycleEvent, LifecycleSourceKind, LifecycleState,
    MemoryPressureEvent, MemoryPressureLevel, PowerMode, PowerModeEvent, ThermalEvent,
    ThermalState, WallClockEvent,
};
pub(crate) use self::resource::{
    ResourceBacking, ResourceCapture, ResourceId, ResourceKind, ResourcePortability, ResourceTable,
};
pub use crate::diagnostic::{HostError, HostErrorCode, HostResult};
