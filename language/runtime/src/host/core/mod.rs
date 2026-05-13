mod clock;
mod errno;
mod error;
mod event;
mod host;
mod queue;
mod session;
mod target;

#[cfg(target_os = "macos")]
pub(crate) use crate::host::unix::apple_process_monotonic_nanos;
#[cfg(unix)]
pub(crate) use crate::host::unix::io_error;
#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) use crate::host::unix::unix_process_monotonic_nanos;
#[cfg(windows)]
pub(crate) use crate::host::windows::io_error;
#[cfg(windows)]
pub(crate) use crate::host::windows::qpc_process_monotonic_nanos;
pub(crate) use clock::monotonic_now_ns;
#[cfg(unix)]
pub(crate) use clock::timeout_deadline;
#[cfg(unix)]
pub(crate) use errno::get_errno;
pub(crate) use error::io_would_block;
pub use event::{
    HostEvent, HostEventKind, LifecycleEvent, LifecycleSourceKind, LifecycleState,
    MemoryPressureEvent, MemoryPressureLevel, PowerMode, PowerModeEvent, ThermalEvent,
    ThermalState, WallClockEvent,
};
pub(crate) use host::Host;
pub use host::HostPollResult;
pub use session::HostSession;
pub(crate) use target::default_compile_target_host;
