use crate::diagnostic::RuntimeResult;
use crate::platform::os::CredentialAuthenticationResult;
use crate::runtime::BindingCallContext;

use super::core::{
    CredentialAuthenticationOptionsOwned, CredentialQueryOwned, CredentialRecordOwned,
    CredentialWriteOptionsOwned,
};

/// Read one credential record from the host backend.
pub(super) fn read_credentials(
    context: &BindingCallContext,
    query: &CredentialQueryOwned,
) -> RuntimeResult<CredentialRecordOwned> {
    // dispatch to the platform backend
    platform_backend::read_credentials(context, query)
}

/// Write one credential record through the host backend.
pub(super) fn write_credentials(
    context: &BindingCallContext,
    options: &CredentialWriteOptionsOwned,
) -> RuntimeResult<()> {
    // dispatch to the platform backend
    platform_backend::write_credentials(context, options)
}

/// Delete one credential record through the host backend.
pub(super) fn delete_credentials(
    context: &BindingCallContext,
    service: &str,
    account: &str,
    access_group: Option<&str>,
) -> RuntimeResult<()> {
    // dispatch to the platform backend
    platform_backend::delete_credentials(context, service, account, access_group)
}

/// Return whether one credential record exists in the host backend.
pub(super) fn contains_credentials(
    context: &BindingCallContext,
    service: &str,
    account: &str,
    access_group: Option<&str>,
) -> RuntimeResult<bool> {
    // dispatch to the platform backend
    platform_backend::contains_credentials(context, service, account, access_group)
}

/// Run one host authentication challenge.
pub(super) fn authenticate_credentials(
    context: &BindingCallContext,
    options: &CredentialAuthenticationOptionsOwned,
) -> RuntimeResult<CredentialAuthenticationResult> {
    // dispatch to the platform backend
    platform_backend::authenticate_credentials(context, options)
}

#[cfg(unix)]
use super::unix as platform_backend;
#[cfg(not(any(unix, windows)))]
use super::unsupported as platform_backend;
#[cfg(windows)]
use super::windows as platform_backend;
