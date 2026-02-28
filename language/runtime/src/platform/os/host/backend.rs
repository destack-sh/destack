use crate::diagnostic::RuntimeResult;
use crate::runtime::BindingCallContext;

use super::core::HostIdentityOwned;

/// Read one host identity payload from the active backend.
pub(super) fn read_host_identity(context: &BindingCallContext) -> RuntimeResult<HostIdentityOwned> {
    // dispatch to the platform backend
    platform_backend::read_host_identity(context)
}

#[cfg(unix)]
use super::unix as platform_backend;
#[cfg(not(any(unix, windows)))]
use super::unsupported as platform_backend;
#[cfg(windows)]
use super::windows as platform_backend;
