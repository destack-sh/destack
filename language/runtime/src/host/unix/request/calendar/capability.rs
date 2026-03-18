use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};

/// Return the static desktop calendar capability ids for the Unix host family.
pub(crate) fn desktop_capabilities() -> [PlatformCapability; 2] {
    [
        PlatformCapability::OsCalendarRead,
        PlatformCapability::OsCalendarWrite,
    ]
}

/// Return dynamic Unix calendar request capabilities.
pub(crate) fn request_capabilities() -> PlatformCapabilitySet {
    #[cfg(all(not(test), target_os = "linux"))]
    {
        return super::linux::request_capabilities();
    }

    #[cfg(all(test, target_os = "linux"))]
    {
        return super::test::request_capabilities();
    }

    #[cfg(any(
        all(not(test), not(target_os = "linux")),
        all(test, not(target_os = "linux"))
    ))]
    {
        super::unsupported::request_capabilities()
    }
}
