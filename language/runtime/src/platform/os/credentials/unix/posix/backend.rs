use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::os::CredentialAuthenticationResult;
use crate::runtime::BindingCallContext;

use super::super::core::{
    CredentialAuthenticationOptionsOwned, CredentialQueryOwned, CredentialRecordOwned,
    CredentialWriteOptionsOwned, OS_CREDENTIALS_AUTHENTICATE_OPERATION,
    OS_CREDENTIALS_CONTAINS_OPERATION, OS_CREDENTIALS_DELETE_OPERATION,
    OS_CREDENTIALS_READ_OPERATION, OS_CREDENTIALS_WRITE_OPERATION,
};

/// Read one credential record from unsupported unix host backends.
pub(crate) fn read_credentials(
    binding: &BindingCallContext,
    _query: &CredentialQueryOwned,
) -> RuntimeResult<CredentialRecordOwned> {
    Err(core_platform::not_supported(OS_CREDENTIALS_READ_OPERATION))
}

/// Write one credential record to unsupported unix host backends.
pub(crate) fn write_credentials(
    binding: &BindingCallContext,
    _options: &CredentialWriteOptionsOwned,
) -> RuntimeResult<()> {
    Err(core_platform::not_supported(OS_CREDENTIALS_WRITE_OPERATION))
}

/// Delete one credential record from unsupported unix host backends.
pub(crate) fn delete_credentials(
    binding: &BindingCallContext,
    _service: &str,
    _account: &str,
    _access_group: Option<&str>,
) -> RuntimeResult<()> {
    Err(core_platform::not_supported(
        OS_CREDENTIALS_DELETE_OPERATION,
    ))
}

/// Return whether one credential record exists in unsupported unix host backends.
pub(crate) fn contains_credentials(
    binding: &BindingCallContext,
    _service: &str,
    _account: &str,
    _access_group: Option<&str>,
) -> RuntimeResult<bool> {
    Err(core_platform::not_supported(
        OS_CREDENTIALS_CONTAINS_OPERATION,
    ))
}

/// Run one host authentication challenge on unsupported unix host backends.
pub(crate) fn authenticate_credentials(
    binding: &BindingCallContext,
    _options: &CredentialAuthenticationOptionsOwned,
) -> RuntimeResult<CredentialAuthenticationResult> {
    Err(core_platform::not_supported(
        OS_CREDENTIALS_AUTHENTICATE_OPERATION,
    ))
}
