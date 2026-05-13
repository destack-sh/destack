use std::sync::Arc;

#[cfg(target_os = "linux")]
use crate::host::linux::LinuxHost;
#[cfg(target_os = "macos")]
use crate::host::macos::MacosHost;
#[cfg(windows)]
use crate::host::windows::WindowsHost;

use super::host::Host;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
use super::host::UnsupportedHost;

/// Return the default host integration for the active compile target.
pub(crate) fn default_compile_target_host() -> Arc<dyn Host> {
    #[cfg(target_os = "linux")]
    return Arc::new(LinuxHost::new());

    #[cfg(target_os = "macos")]
    return Arc::new(MacosHost::new());

    #[cfg(windows)]
    return Arc::new(WindowsHost::new());

    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    return Arc::new(UnsupportedHost::new());
}
