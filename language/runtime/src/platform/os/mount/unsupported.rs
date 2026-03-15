use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::os::mount::core::MountEntryOwned;

/// Read the current host mount table on unsupported targets.
pub(crate) fn read_mount_entries() -> RuntimeResult<Vec<MountEntryOwned>> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.os.mount.list")).boxed())
}
