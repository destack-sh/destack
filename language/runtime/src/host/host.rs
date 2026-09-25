use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::host::HostEvent;
#[cfg(target_os = "linux")]
use crate::host::linux::LinuxHost;
#[cfg(target_os = "macos")]
use crate::host::macos::MacosHost;
use crate::host::time::HostClockSource;
#[cfg(windows)]
use crate::host::windows::WindowsHost;

/// Runtime boundary for process-local host integration.
pub(crate) trait Host: std::fmt::Debug + Send + Sync {
    /// Return whether the current execution context is the process main context.
    fn is_process_main_context(&self) -> bool {
        false
    }

    /// Advance immediately ready host events without blocking.
    fn advance_events(&self) -> RuntimeResult<()> {
        Ok(())
    }

    /// Collect host events that are ready to enter the runtime.
    fn collect_events(&self) -> RuntimeResult<Vec<HostEvent>> {
        Ok(Vec::new())
    }

    /// Return one host wall-clock sample in nanoseconds.
    fn wall_nanos(&self) -> u64;

    /// Return one host monotonic-clock sample in nanoseconds.
    fn mono_nanos(&self) -> u64;

    /// Sleep on the host for one duration in nanoseconds.
    fn sleep_nanos(&self, duration_nanos: u64);

    /// Sleep on the host until one wall-clock deadline in nanoseconds.
    fn sleep_until_wall_nanos(&self, deadline_nanos: u64);

    /// Fill one buffer with host entropy.
    fn fill_random_bytes(&self, buffer: &mut [u8]) -> RuntimeResult<()>;

    /// Try to fill one buffer with host entropy without blocking.
    fn try_fill_random_bytes(&self, buffer: &mut [u8]) -> RuntimeResult<()>;

    /// Return one host entropy u64.
    fn random_u64(&self) -> RuntimeResult<u64>;
}

/// Create the host integration for the active compile target.
#[cfg(target_os = "linux")]
pub(crate) fn compile_target(clock_source: Option<Arc<dyn HostClockSource>>) -> Arc<dyn Host> {
    Arc::new(LinuxHost::new(clock_source))
}

/// Create the host integration for the active compile target.
#[cfg(target_os = "macos")]
pub(crate) fn compile_target(clock_source: Option<Arc<dyn HostClockSource>>) -> Arc<dyn Host> {
    Arc::new(MacosHost::new(clock_source))
}

/// Create the host integration for the active compile target.
#[cfg(windows)]
pub(crate) fn compile_target(clock_source: Option<Arc<dyn HostClockSource>>) -> Arc<dyn Host> {
    Arc::new(WindowsHost::new(clock_source))
}

/// Return the compile target platform name.
pub(crate) const fn platform_name() -> &'static str {
    #[cfg(target_os = "windows")]
    return "windows";

    #[cfg(target_os = "linux")]
    return "linux";

    #[cfg(target_os = "macos")]
    return "macos";

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    compile_error!("tspp_runtime host supports linux, macos, and windows");
}

/// Return the compile target platform-family name.
pub(crate) const fn family_name() -> &'static str {
    #[cfg(windows)]
    return "windows";

    #[cfg(unix)]
    return "unix";
}

/// Return the compile target host name.
pub(crate) const fn host_name() -> &'static str {
    "native"
}
