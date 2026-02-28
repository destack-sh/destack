use crate::diagnostic::RuntimeResult;
use crate::runtime::BindingCallContext;

use super::core::{HostIdentityOwned, OS_HOST_IDENTITY_OPERATION, not_supported};

/// Read one host identity payload from unsupported backends.
pub(super) fn read_host_identity(
    _context: &BindingCallContext,
) -> RuntimeResult<HostIdentityOwned> {
    Err(not_supported(OS_HOST_IDENTITY_OPERATION))
}
