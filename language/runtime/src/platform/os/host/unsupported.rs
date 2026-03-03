use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::runtime::BindingCallContext;

use super::core::{HostIdentityOwned, OS_HOST_IDENTITY_OPERATION};

/// Read one host identity payload from unsupported backends.
pub(super) fn read_host_identity(
    _context: &BindingCallContext,
) -> RuntimeResult<HostIdentityOwned> {
    Err(core_platform::not_supported(OS_HOST_IDENTITY_OPERATION))
}
