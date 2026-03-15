#![allow(unused_variables)]

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::os::host::core::{HostIdentityOwned, OS_HOST_IDENTITY_OPERATION};
use crate::runtime::BindingCallContext;

/// Read one host identity payload from unsupported backends.
pub(super) fn read_host_identity(binding: &BindingCallContext) -> RuntimeResult<HostIdentityOwned> {
    Err(core_platform::not_supported(OS_HOST_IDENTITY_OPERATION))
}
