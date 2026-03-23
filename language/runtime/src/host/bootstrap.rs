use std::sync::Arc;

use destack_artifact::Platform;
use destack_workspace::{PlatformHostOptions, RuntimeOptions};

#[cfg(target_os = "android")]
use crate::host::android::AndroidHost;
#[cfg(target_os = "android")]
use crate::host::android::abi::registry::unregister_android_bindings;
use crate::host::core::HostAdapter;
use crate::host::core::registry::HostCleanup;
#[cfg(target_os = "dragonfly")]
use crate::host::dragonfly::DragonflyHost;
#[cfg(target_os = "freebsd")]
use crate::host::freebsd::FreeBsdHost;
#[cfg(target_os = "haiku")]
use crate::host::haiku::HaikuHost;
#[cfg(target_os = "illumos")]
use crate::host::illumos::IllumosHost;
#[cfg(target_os = "ios")]
use crate::host::ios::IosHost;
#[cfg(target_os = "ios")]
use crate::host::ios::abi::registry::unregister_ios_bindings;
#[cfg(target_os = "linux")]
use crate::host::linux::LinuxHost;
#[cfg(target_os = "linux")]
use crate::host::linux::ingress::unregister_linux_runtime;
#[cfg(target_os = "macos")]
use crate::host::macos::MacosHost;
#[cfg(target_os = "macos")]
use crate::host::macos::ingress::unregister_macos_runtime;
#[cfg(target_os = "netbsd")]
use crate::host::netbsd::NetBsdHost;
#[cfg(target_os = "openbsd")]
use crate::host::openbsd::OpenBsdHost;
#[cfg(target_os = "solaris")]
use crate::host::solaris::SolarisHost;
#[cfg(not(any(
    target_os = "android",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "haiku",
    target_os = "illumos",
    target_os = "ios",
    target_os = "linux",
    target_os = "macos",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "solaris",
    windows,
)))]
use crate::host::unsupported::UnsupportedHost;
#[cfg(windows)]
use crate::host::windows::WindowsHost;
#[cfg(windows)]
use crate::host::windows::ingress::unregister_windows_runtime;

/// Return the default host integration parts for the active compile target.
pub(crate) fn default_compile_target_parts() -> (Platform, Arc<dyn HostAdapter>, Option<HostCleanup>)
{
    #[cfg(target_os = "android")]
    return (
        Platform::Android,
        Arc::new(AndroidHost::new()),
        Some(unregister_android_bindings),
    );

    #[cfg(target_os = "dragonfly")]
    return (Platform::DragonFly, Arc::new(DragonflyHost::new()), None);

    #[cfg(target_os = "freebsd")]
    return (Platform::FreeBsd, Arc::new(FreeBsdHost::new()), None);

    #[cfg(target_os = "haiku")]
    return (Platform::Haiku, Arc::new(HaikuHost::new()), None);

    #[cfg(target_os = "illumos")]
    return (Platform::Illumos, Arc::new(IllumosHost::new()), None);

    #[cfg(target_os = "ios")]
    return (
        Platform::IOS,
        Arc::new(IosHost::new()),
        Some(unregister_ios_bindings),
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

    #[cfg(target_os = "netbsd")]
    return (Platform::NetBsd, Arc::new(NetBsdHost::new()), None);

    #[cfg(target_os = "openbsd")]
    return (Platform::OpenBsd, Arc::new(OpenBsdHost::new()), None);

    #[cfg(target_os = "solaris")]
    return (Platform::Solaris, Arc::new(SolarisHost::new()), None);

    #[cfg(windows)]
    return (
        Platform::Windows,
        Arc::new(WindowsHost::new()),
        Some(unregister_windows_runtime),
    );

    #[cfg(not(any(
        target_os = "android",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "haiku",
        target_os = "illumos",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "solaris",
        windows,
    )))]
    return (Platform::Universal, Arc::new(UnsupportedHost::new()), None);
}

/// Return host event queue options for one compile target platform.
pub(crate) fn host_options_for_target(
    platform: Platform,
    options: &RuntimeOptions,
) -> PlatformHostOptions {
    match platform {
        Platform::Android => options.platform.android.clone(),
        Platform::DragonFly => options.platform.dragonfly.clone(),
        Platform::FreeBsd => options.platform.freebsd.clone(),
        Platform::Haiku => options.platform.haiku.clone(),
        Platform::Illumos => options.platform.illumos.clone(),
        Platform::IOS => options.platform.ios.clone(),
        Platform::Linux => options.platform.linux.clone(),
        Platform::MacOS => options.platform.macos.clone(),
        Platform::NetBsd => options.platform.netbsd.clone(),
        Platform::OpenBsd => options.platform.openbsd.clone(),
        Platform::Solaris => options.platform.solaris.clone(),
        Platform::Windows => options.platform.windows.host_options(),
        _ => PlatformHostOptions::default(),
    }
}
