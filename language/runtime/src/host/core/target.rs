use std::sync::Arc;

#[cfg(target_os = "linux")]
use crate::host::linux::LinuxHost;
#[cfg(target_os = "macos")]
use crate::host::macos::MacosHost;
use crate::host::time::HostClockSource;
#[cfg(windows)]
use crate::host::windows::WindowsHost;

use super::host::Host;

/// Return the host integration for the active compile target.
#[cfg(target_os = "linux")]
pub(crate) fn compile_target_host(clock_source: Option<Arc<dyn HostClockSource>>) -> Arc<dyn Host> {
    Arc::new(LinuxHost::new(clock_source))
}

/// Return the host integration for the active compile target.
#[cfg(target_os = "macos")]
pub(crate) fn compile_target_host(clock_source: Option<Arc<dyn HostClockSource>>) -> Arc<dyn Host> {
    Arc::new(MacosHost::new(clock_source))
}

/// Return the host integration for the active compile target.
#[cfg(windows)]
pub(crate) fn compile_target_host(clock_source: Option<Arc<dyn HostClockSource>>) -> Arc<dyn Host> {
    Arc::new(WindowsHost::new(clock_source))
}
