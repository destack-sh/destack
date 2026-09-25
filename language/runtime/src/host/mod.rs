#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
compile_error!("tspp_runtime host supports linux, macos, and windows");

pub(crate) mod accessibility;
pub(crate) mod audio;
pub(crate) mod crypto;
pub(crate) mod device;
pub(crate) mod display;
mod event;
pub(crate) mod fs;
pub(crate) mod gpu;
mod host;
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

pub(crate) use self::event::HostQueue;
pub use self::event::{
    HostEvent, HostEventKind, LifecycleEvent, LifecycleSourceKind, LifecycleState,
    MemoryPressureEvent, MemoryPressureLevel, PowerMode, PowerModeEvent, ThermalEvent,
    ThermalState, WallClockEvent,
};
pub(crate) use self::host::{Host, compile_target, family_name, host_name, platform_name};
pub(crate) use self::resource::{ResourceId, ResourceKind, ResourceTable};
pub(crate) use self::time::monotonic_now_ns;
#[cfg(unix)]
pub(crate) use self::time::timeout_deadline;
#[cfg(unix)]
pub(crate) use self::unix::{get_errno, io_error};
#[cfg(windows)]
pub(crate) use self::windows::io_error;
pub use crate::diagnostic::{HostError, HostErrorCode, HostResult};
