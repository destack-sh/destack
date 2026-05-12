use std::sync::Arc;

use destack_artifact::Platform;

use crate::host::HostAdapter;
use crate::host::core::registry::HostCleanup;
#[cfg(target_os = "android")]
use crate::host::os::android::AndroidHost;
#[cfg(target_os = "android")]
use crate::host::os::android::abi::registry::unregister_android_bindings;
#[cfg(target_os = "ios")]
use crate::host::os::apple::abi::registry::unregister_ios_bindings;
#[cfg(target_os = "ios")]
use crate::host::os::ios::IosHost;
#[cfg(target_os = "linux")]
use crate::host::os::linux::LinuxHost;
#[cfg(target_os = "linux")]
use crate::host::os::linux::ingress::unregister_linux_runtime;
#[cfg(target_os = "macos")]
use crate::host::os::macos::MacosHost;
#[cfg(target_os = "macos")]
use crate::host::os::macos::ingress::unregister_macos_runtime;
#[cfg(not(any(
    target_os = "android",
    target_os = "ios",
    target_os = "linux",
    target_os = "macos",
    windows,
)))]
use crate::host::os::unsupported::UnsupportedHost;
#[cfg(windows)]
use crate::host::os::windows::WindowsHost;
#[cfg(windows)]
use crate::host::os::windows::ingress::unregister_windows_runtime;

/// Return the default host integration parts for the active compile target.
pub(crate) fn default_compile_target_parts() -> (Platform, Arc<dyn HostAdapter>, Option<HostCleanup>)
{
    #[cfg(target_os = "android")]
    return (
        Platform::Android,
        Arc::new(AndroidHost::new()),
        Some(|host_session_id| unregister_android_bindings(host_session_id.handle())),
    );

    #[cfg(target_os = "ios")]
    return (
        Platform::IOS,
        Arc::new(IosHost::new()),
        Some(|host_session_id| unregister_ios_bindings(host_session_id.handle())),
    );

    #[cfg(target_os = "linux")]
    return (
        Platform::Linux,
        Arc::new(LinuxHost::new()),
        Some(unregister_linux_runtime),
    );

    #[cfg(target_os = "macos")]
    return (
        Platform::MacOS,
        Arc::new(MacosHost::new()),
        Some(unregister_macos_runtime),
    );

    #[cfg(windows)]
    return (
        Platform::Windows,
        Arc::new(WindowsHost::new()),
        Some(unregister_windows_runtime),
    );

    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos",
        windows,
    )))]
    return (Platform::Unknown, Arc::new(UnsupportedHost::new()), None);
}
