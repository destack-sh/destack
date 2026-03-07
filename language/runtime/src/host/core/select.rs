use std::sync::Arc;

use super::HostPlatform;
use super::adapter::HostAdapter;
use crate::runtime::world::RuntimeId;

#[cfg(target_os = "android")]
use crate::host::android::AndroidHost;
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
#[cfg(target_os = "linux")]
use crate::host::linux::LinuxHost;
#[cfg(target_os = "macos")]
use crate::host::macos::MacosHost;
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

/// Return the host platform for the active compile target.
pub(super) const fn compile_target_host_platform() -> HostPlatform {
    #[cfg(target_os = "android")]
    {
        HostPlatform::Android
    }

    #[cfg(target_os = "dragonfly")]
    {
        HostPlatform::DragonFly
    }

    #[cfg(target_os = "freebsd")]
    {
        HostPlatform::FreeBsd
    }

    #[cfg(target_os = "haiku")]
    {
        HostPlatform::Haiku
    }

    #[cfg(target_os = "illumos")]
    {
        HostPlatform::Illumos
    }

    #[cfg(target_os = "ios")]
    {
        HostPlatform::IOS
    }

    #[cfg(target_os = "linux")]
    {
        HostPlatform::Linux
    }

    #[cfg(target_os = "macos")]
    {
        HostPlatform::MacOS
    }

    #[cfg(target_os = "netbsd")]
    {
        HostPlatform::NetBsd
    }

    #[cfg(target_os = "openbsd")]
    {
        HostPlatform::OpenBsd
    }

    #[cfg(target_os = "solaris")]
    {
        HostPlatform::Solaris
    }

    #[cfg(windows)]
    {
        HostPlatform::Windows
    }

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
    {
        HostPlatform::Universal
    }
}

/// Return one default host for the active compile target.
pub fn default_host(runtime_id: RuntimeId) -> Arc<dyn HostAdapter> {
    #[cfg(target_os = "android")]
    return Arc::new(AndroidHost::new(runtime_id));

    #[cfg(target_os = "dragonfly")]
    return Arc::new(DragonflyHost::new(runtime_id));

    #[cfg(target_os = "freebsd")]
    return Arc::new(FreeBsdHost::new(runtime_id));

    #[cfg(target_os = "haiku")]
    return Arc::new(HaikuHost::new(runtime_id));

    #[cfg(target_os = "illumos")]
    return Arc::new(IllumosHost::new(runtime_id));

    #[cfg(target_os = "ios")]
    return Arc::new(IosHost::new(runtime_id));

    #[cfg(target_os = "linux")]
    return Arc::new(LinuxHost::new(runtime_id));

    #[cfg(target_os = "macos")]
    return Arc::new(MacosHost::new(runtime_id));

    #[cfg(target_os = "netbsd")]
    return Arc::new(NetBsdHost::new(runtime_id));

    #[cfg(target_os = "openbsd")]
    return Arc::new(OpenBsdHost::new(runtime_id));

    #[cfg(target_os = "solaris")]
    return Arc::new(SolarisHost::new(runtime_id));

    #[cfg(windows)]
    return Arc::new(WindowsHost::new(runtime_id));

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
    return Arc::new(UnsupportedHost::new(runtime_id));
}
