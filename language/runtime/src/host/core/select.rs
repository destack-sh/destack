use std::sync::Arc;

use super::HostPlatform;
use super::adapter::Host;

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
pub fn default_host() -> Arc<dyn Host> {
    #[cfg(target_os = "android")]
    return Arc::new(crate::host::android::AndroidHost::new());

    #[cfg(target_os = "dragonfly")]
    return Arc::new(crate::host::dragonfly::DragonflyHost::new());

    #[cfg(target_os = "freebsd")]
    return Arc::new(crate::host::freebsd::FreeBsdHost::new());

    #[cfg(target_os = "haiku")]
    return Arc::new(crate::host::haiku::HaikuHost::new());

    #[cfg(target_os = "illumos")]
    return Arc::new(crate::host::illumos::IllumosHost::new());

    #[cfg(target_os = "ios")]
    return Arc::new(crate::host::ios::IosHost::new());

    #[cfg(target_os = "linux")]
    return Arc::new(crate::host::linux::LinuxHost::new());

    #[cfg(target_os = "macos")]
    return Arc::new(crate::host::macos::MacosHost::new());

    #[cfg(target_os = "netbsd")]
    return Arc::new(crate::host::netbsd::NetBsdHost::new());

    #[cfg(target_os = "openbsd")]
    return Arc::new(crate::host::openbsd::OpenBsdHost::new());

    #[cfg(target_os = "solaris")]
    return Arc::new(crate::host::solaris::SolarisHost::new());

    #[cfg(windows)]
    return Arc::new(crate::host::windows::WindowsHost::new());

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
    return Arc::new(crate::host::unsupported::UnsupportedHost::new());
}
