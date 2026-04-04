use super::{HostOperation, decode};

use crate::host::HostRequest;

/// Build one open-settings operation.
pub(crate) fn open_settings() -> HostOperation<()> {
    HostOperation::new(HostRequest::OsPermissionOpenSettings, decode::none)
}
