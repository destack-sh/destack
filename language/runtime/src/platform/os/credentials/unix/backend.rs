use crate::diagnostic::RuntimeResult;
use crate::platform::os::CredentialAuthenticationResult;
use crate::runtime::BindingCallContext;

use super::super::core::{
    CredentialAuthenticationOptionsOwned, CredentialQueryOwned, CredentialRecordOwned,
    CredentialWriteOptionsOwned,
};

/// Read one credential record from the unix backend.
pub(crate) fn read_credentials(
    context: &BindingCallContext,
    query: &CredentialQueryOwned,
) -> RuntimeResult<CredentialRecordOwned> {
    platform_backend::read_credentials(context, query)
}

/// Write one credential record through the unix backend.
pub(crate) fn write_credentials(
    context: &BindingCallContext,
    options: &CredentialWriteOptionsOwned,
) -> RuntimeResult<()> {
    platform_backend::write_credentials(context, options)
}

/// Delete one credential record through the unix backend.
pub(crate) fn delete_credentials(
    context: &BindingCallContext,
    service: &str,
    account: &str,
    access_group: Option<&str>,
) -> RuntimeResult<()> {
    platform_backend::delete_credentials(context, service, account, access_group)
}

/// Return whether one credential record exists in the unix backend.
pub(crate) fn contains_credentials(
    context: &BindingCallContext,
    service: &str,
    account: &str,
    access_group: Option<&str>,
) -> RuntimeResult<bool> {
    platform_backend::contains_credentials(context, service, account, access_group)
}

/// Run one host authentication challenge on unix.
pub(crate) fn authenticate_credentials(
    context: &BindingCallContext,
    options: &CredentialAuthenticationOptionsOwned,
) -> RuntimeResult<CredentialAuthenticationResult> {
    platform_backend::authenticate_credentials(context, options)
}

#[cfg(target_os = "android")]
use super::android as platform_backend;
#[cfg(any(target_os = "macos", target_os = "ios"))]
use super::apple as platform_backend;
#[cfg(target_os = "linux")]
use super::linux as platform_backend;
#[cfg(not(any(
    target_os = "android",
    target_os = "ios",
    target_os = "linux",
    target_os = "macos",
)))]
use super::posix as platform_backend;
