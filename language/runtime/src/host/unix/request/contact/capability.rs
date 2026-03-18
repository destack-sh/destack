use crate::runtime::capability::PlatformCapability;

/// Return the static desktop contact capability ids for the Unix host family.
pub(crate) fn desktop_capabilities() -> [PlatformCapability; 2] {
    [
        PlatformCapability::OsContactRead,
        PlatformCapability::OsContactWrite,
    ]
}
