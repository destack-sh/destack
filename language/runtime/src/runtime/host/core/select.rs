use std::sync::Arc;

use super::adapter::HostAdapter;

/// Return one default host adapter for the active compile target.
pub fn default_host_adapter() -> Arc<dyn HostAdapter> {
    #[cfg(target_os = "android")]
    {
        Arc::new(crate::runtime::host::android::AndroidHostAdapter::new())
    }

    #[cfg(target_os = "dragonfly")]
    {
        Arc::new(crate::runtime::host::dragonfly::DragonflyHostAdapter::new())
    }

    #[cfg(target_os = "freebsd")]
    {
        Arc::new(crate::runtime::host::freebsd::FreeBsdHostAdapter::new())
    }

    #[cfg(target_os = "haiku")]
    {
        Arc::new(crate::runtime::host::haiku::HaikuHostAdapter::new())
    }

    #[cfg(target_os = "illumos")]
    {
        Arc::new(crate::runtime::host::illumos::IllumosHostAdapter::new())
    }

    #[cfg(target_os = "ios")]
    {
        Arc::new(crate::runtime::host::ios::IosHostAdapter::new())
    }

    #[cfg(target_os = "linux")]
    {
        Arc::new(crate::runtime::host::linux::LinuxHostAdapter::new())
    }

    #[cfg(target_os = "macos")]
    {
        Arc::new(crate::runtime::host::macos::MacosHostAdapter::new())
    }

    #[cfg(target_os = "netbsd")]
    {
        Arc::new(crate::runtime::host::netbsd::NetBsdHostAdapter::new())
    }

    #[cfg(target_os = "openbsd")]
    {
        Arc::new(crate::runtime::host::openbsd::OpenBsdHostAdapter::new())
    }

    #[cfg(target_os = "solaris")]
    {
        Arc::new(crate::runtime::host::solaris::SolarisHostAdapter::new())
    }

    #[cfg(windows)]
    {
        Arc::new(crate::runtime::host::windows::WindowsHostAdapter::new())
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
        Arc::new(crate::runtime::host::unsupported::UnsupportedHostAdapter::new())
    }
}
