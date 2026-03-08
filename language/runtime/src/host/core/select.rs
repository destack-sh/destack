use std::sync::Arc;

use super::Platform;
use super::backend::HostBackend;
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
pub(super) const fn compile_target_host_platform() -> Platform {
    #[cfg(target_os = "android")]
    {
        Platform::Android
    }

    #[cfg(target_os = "dragonfly")]
    {
        Platform::DragonFly
    }

    #[cfg(target_os = "freebsd")]
    {
        Platform::FreeBsd
    }

    #[cfg(target_os = "haiku")]
    {
        Platform::Haiku
    }

    #[cfg(target_os = "illumos")]
    {
        Platform::Illumos
    }

    #[cfg(target_os = "ios")]
    {
        Platform::IOS
    }

    #[cfg(target_os = "linux")]
    {
        Platform::Linux
    }

    #[cfg(target_os = "macos")]
    {
        Platform::MacOS
    }

    #[cfg(target_os = "netbsd")]
    {
        Platform::NetBsd
    }

    #[cfg(target_os = "openbsd")]
    {
        Platform::OpenBsd
    }

    #[cfg(target_os = "solaris")]
    {
        Platform::Solaris
    }

    #[cfg(windows)]
    {
        Platform::Windows
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
        Platform::Universal
    }
}

/// Return one default host for the active compile target.
pub(crate) fn default_host() -> Arc<dyn HostBackend> {
    #[cfg(target_os = "android")]
    return Arc::new(AndroidHost::new());

    #[cfg(target_os = "dragonfly")]
    return Arc::new(DragonflyHost::new());

    #[cfg(target_os = "freebsd")]
    return Arc::new(FreeBsdHost::new());

    #[cfg(target_os = "haiku")]
    return Arc::new(HaikuHost::new());

    #[cfg(target_os = "illumos")]
    return Arc::new(IllumosHost::new());

    #[cfg(target_os = "ios")]
    return Arc::new(IosHost::new());

    #[cfg(target_os = "linux")]
    return Arc::new(LinuxHost::new());

    #[cfg(target_os = "macos")]
    return Arc::new(MacosHost::new());

    #[cfg(target_os = "netbsd")]
    return Arc::new(NetBsdHost::new());

    #[cfg(target_os = "openbsd")]
    return Arc::new(OpenBsdHost::new());

    #[cfg(target_os = "solaris")]
    return Arc::new(SolarisHost::new());

    #[cfg(windows)]
    return Arc::new(WindowsHost::new());

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
    return Arc::new(UnsupportedHost::new());
}
