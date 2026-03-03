#[cfg(any(target_os = "linux", target_os = "windows"))]
use parking_lot::Mutex;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::{
    CredentialAccessibility, CredentialAuthenticationPolicy, CredentialAuthenticationRequirement,
};
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::{BindingCallContext, NativeSlice, NativeStringRef};

use super::backend;

/// Operation name for credentials read.
pub(crate) const OS_CREDENTIALS_READ_OPERATION: &str = "destack.os.credentials.read";
/// Operation name for credentials write.
pub(crate) const OS_CREDENTIALS_WRITE_OPERATION: &str = "destack.os.credentials.write";
/// Operation name for credentials delete.
pub(crate) const OS_CREDENTIALS_DELETE_OPERATION: &str = "destack.os.credentials.delete";
/// Operation name for credentials contains.
pub(crate) const OS_CREDENTIALS_CONTAINS_OPERATION: &str = "destack.os.credentials.contains";
/// Operation name for credentials authenticate.
pub(crate) const OS_CREDENTIALS_AUTHENTICATE_OPERATION: &str =
    "destack.os.credentials.authenticate";

/// Decoded credential query payload.
#[derive(Debug, Clone)]
pub(crate) struct CredentialQueryOwned {
    /// Service namespace.
    pub service: String,
    /// Account key within the service.
    pub account: String,
    /// Optional host access group.
    pub access_group: Option<String>,
    /// Whether host authentication is required for reads.
    pub require_authentication: bool,
}

/// Decoded credential write payload.
#[derive(Debug, Clone)]
pub(crate) struct CredentialWriteOptionsOwned {
    /// Service namespace.
    pub service: String,
    /// Account key within the service.
    pub account: String,
    /// Optional host access group.
    pub access_group: Option<String>,
    /// Credential payload bytes.
    pub bytes: Vec<u8>,
    /// Credential accessibility class.
    pub accessibility: CredentialAccessibility,
    /// Authentication policy for credential access.
    pub authentication: CredentialAuthenticationPolicy,
    /// Whether existing records may be replaced.
    pub replace_existing: bool,
}

/// Decoded credential record payload.
#[derive(Debug, Clone)]
pub(crate) struct CredentialRecordOwned {
    /// Service namespace.
    pub service: String,
    /// Account key within the service.
    pub account: String,
    /// Credential payload bytes.
    pub bytes: Vec<u8>,
    /// Optional host creation timestamp in unix nanoseconds.
    pub created_unix_ns: u64,
    /// Optional host modification timestamp in unix nanoseconds.
    pub modified_unix_ns: u64,
}

/// Decoded credential authentication request payload.
#[derive(Debug, Clone)]
pub(crate) struct CredentialAuthenticationOptionsOwned {
    /// Host prompt title.
    pub title: String,
    /// Host prompt subtitle.
    pub subtitle: String,
    /// Host prompt message.
    pub message: String,
    /// Required host authentication policy.
    pub requirement: CredentialAuthenticationRequirement,
}

/// Decode one native string argument into owned text.
pub(crate) fn decode_native_string(
    argument: NativeStringRef,
    field: &str,
) -> RuntimeResult<String> {
    let value = unsafe { argument.as_str() }.map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "string argument was not valid utf-8",
        ))
        .boxed()
    })?;

    Ok(value.to_string())
}

/// Decode one native byte slice argument into owned bytes.
pub(crate) fn decode_native_bytes(bytes: NativeSlice<u8>, field: &str) -> RuntimeResult<Vec<u8>> {
    let bytes = unsafe { bytes.as_slice() }.map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "slice argument was invalid",
        ))
        .boxed()
    })?;

    Ok(bytes.to_vec())
}

/// Normalize one optional string argument.
pub(crate) fn normalize_optional_string(value: String) -> Option<String> {
    if value.is_empty() {
        return None;
    }

    Some(value)
}

/// Read one credential record from the active host backend.
pub(crate) fn read_credentials(
    context: &BindingCallContext,
    query: &CredentialQueryOwned,
) -> RuntimeResult<CredentialRecordOwned> {
    // validate primary key fields
    validate_service_account(
        &query.service,
        &query.account,
        OS_CREDENTIALS_READ_OPERATION,
        "query",
    )?;

    // dispatch to the host backend
    backend::read_credentials(context, query)
}

/// Write one credential record to the active host backend.
pub(crate) fn write_credentials(
    context: &BindingCallContext,
    options: &CredentialWriteOptionsOwned,
) -> RuntimeResult<()> {
    // validate primary key fields
    validate_service_account(
        &options.service,
        &options.account,
        OS_CREDENTIALS_WRITE_OPERATION,
        "options",
    )?;

    // reject empty payload writes because this lane stores binary secrets
    if options.bytes.is_empty() {
        return Err(core_platform::invalid_argument(
            "options.bytes",
            "credential payload cannot be empty",
        ));
    }

    // dispatch to the host backend
    backend::write_credentials(context, options)
}

/// Delete one credential record from the active host backend.
pub(crate) fn delete_credentials(
    context: &BindingCallContext,
    service: &str,
    account: &str,
    access_group: Option<&str>,
) -> RuntimeResult<()> {
    // validate primary key fields
    validate_service_account(
        service,
        account,
        OS_CREDENTIALS_DELETE_OPERATION,
        "credential",
    )?;

    // dispatch to the host backend
    backend::delete_credentials(context, service, account, access_group)
}

/// Return whether one credential record exists on the active host backend.
pub(crate) fn contains_credentials(
    context: &BindingCallContext,
    service: &str,
    account: &str,
    access_group: Option<&str>,
) -> RuntimeResult<bool> {
    // validate primary key fields
    validate_service_account(
        service,
        account,
        OS_CREDENTIALS_CONTAINS_OPERATION,
        "credential",
    )?;

    // dispatch to the host backend
    backend::contains_credentials(context, service, account, access_group)
}

/// Run one host authentication challenge.
pub(crate) fn authenticate_credentials(
    context: &BindingCallContext,
    options: &CredentialAuthenticationOptionsOwned,
) -> RuntimeResult<crate::platform::os::CredentialAuthenticationResult> {
    // validate prompt fields for host APIs that require text
    validate_authentication_prompt(options)?;

    // dispatch to the host backend
    backend::authenticate_credentials(context, options)
}

/// Build one ioInvalidData runtime error.
pub(crate) fn invalid_data(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Build one ioAlreadyExists runtime error.
#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    target_os = "windows"
))]
pub(crate) fn already_exists(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoAlreadyExists),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Build one ioPermissionDenied runtime error.
pub(crate) fn permission_denied(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoPermissionDenied),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Build one ioWouldBlock runtime error.
#[cfg(any(target_os = "ios", target_os = "macos", target_os = "windows"))]
pub(crate) fn would_block(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoWouldBlock),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Build one ioInterrupted runtime error.
#[cfg(any(target_os = "ios", target_os = "macos"))]
pub(crate) fn interrupted(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInterrupted),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Runtime-local guard state for no-replace credential writes.
#[cfg(any(target_os = "linux", target_os = "windows"))]
#[derive(Debug, Default)]
pub(crate) struct NoReplaceWriteRuntimeState {
    /// In-process mutex for check-then-write serialization.
    lock: Mutex<()>,
}

/// Execute one closure under one runtime-local no-replace write guard.
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub(crate) fn with_no_replace_write_guard<R>(
    context: &BindingCallContext,
    execute: impl FnOnce() -> RuntimeResult<R>,
) -> RuntimeResult<R> {
    // resolve one runtime-local lock holder for credential writes
    let runtime_state = context
        .runtime()
        .platform_state
        .os
        .no_replace_write_runtime_state(NoReplaceWriteRuntimeState::default);

    // serialize one check-then-write sequence for this runtime
    let _guard = runtime_state.lock.lock();

    execute()
}

/// Validate one service and account pair.
fn validate_service_account(
    service: &str,
    account: &str,
    operation: &'static str,
    field_scope: &str,
) -> RuntimeResult<()> {
    // reject empty services because host credential managers index by service namespace
    if service.is_empty() {
        return Err(core_platform::invalid_argument(
            format!("{field_scope}.service"),
            format!("{operation}: service cannot be empty"),
        ));
    }

    // reject empty accounts because host credential managers index by account key
    if account.is_empty() {
        return Err(core_platform::invalid_argument(
            format!("{field_scope}.account"),
            format!("{operation}: account cannot be empty"),
        ));
    }

    Ok(())
}

/// Validate one authentication prompt payload.
fn validate_authentication_prompt(
    options: &CredentialAuthenticationOptionsOwned,
) -> RuntimeResult<()> {
    // read requirement here so prompt validation owns the full option contract
    let _requirement = options.requirement;

    // normalize each prompt field for validation
    let title = options.title.trim();
    let subtitle = options.subtitle.trim();
    let message = options.message.trim();

    // reject fully empty prompts to keep host ui intent explicit
    if title.is_empty() && subtitle.is_empty() && message.is_empty() {
        return Err(core_platform::invalid_argument(
            "options",
            "at least one authentication prompt field must be non-empty",
        ));
    }

    Ok(())
}
