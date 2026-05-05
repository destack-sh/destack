#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::collections::HashMap;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::sync::{Arc, OnceLock};

use destack_vm as vm;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use parking_lot::Mutex;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::{
    CredentialAccessibility, CredentialAuthenticationOptions, CredentialAuthenticationOptionsVm,
    CredentialAuthenticationPolicy, CredentialAuthenticationRequirement,
    CredentialAuthenticationResult, CredentialQuery, CredentialQueryVm, CredentialRecord,
    CredentialRecordVm, CredentialWriteOptions, CredentialWriteOptionsVm,
};
use crate::platform::{PlatformError, VmSlice, core as core_platform};
use crate::runtime::BindingCallContext;

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
pub(crate) fn normalize_optional_string(value: Option<String>) -> Option<String> {
    let value = value?;
    if value.is_empty() {
        return None;
    }

    Some(value)
}

/// Decode one VM string argument into owned text.
pub(crate) fn decode_vm_string(
    context: &mut vm::BindingContext<'_>,
    argument: vm::StringHandle,
    field: &str,
) -> RuntimeResult<String> {
    let value = context.string_ref(argument).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "string argument was invalid",
        ))
        .boxed()
    })?;

    Ok(value.as_str().to_string())
}

/// Decode one VM byte slice argument into owned bytes.
pub(crate) fn decode_vm_bytes(
    context: &mut vm::BindingContext<'_>,
    bytes: VmSlice<u8>,
    field: &str,
) -> RuntimeResult<Vec<u8>> {
    let bytes = bytes.read_bytes(&context.read()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "slice argument was invalid",
        ))
        .boxed()
    })?;

    Ok(bytes.to_vec())
}

/// Decode one native credential query into owned data.
pub(crate) fn decode_native_query(query: CredentialQuery) -> RuntimeResult<CredentialQueryOwned> {
    let service = decode_native_string(query.service, "query.service")?;
    let account = decode_native_string(query.account, "query.account")?;
    let access_group = match query.access_group {
        Some(access_group) => Some(decode_native_string(access_group, "query.accessGroup")?),
        None => None,
    };

    Ok(CredentialQueryOwned {
        service,
        account,
        access_group: normalize_optional_string(access_group),
        require_authentication: query.require_authentication,
    })
}

/// Decode one VM credential query into owned data.
pub(crate) fn decode_vm_query(
    context: &mut vm::BindingContext<'_>,
    query: CredentialQueryVm,
) -> RuntimeResult<CredentialQueryOwned> {
    let service = decode_vm_string(context, query.service, "query.service")?;
    let account = decode_vm_string(context, query.account, "query.account")?;
    let access_group = match query.access_group {
        Some(access_group) => Some(decode_vm_string(
            context,
            access_group,
            "query.accessGroup",
        )?),
        None => None,
    };

    Ok(CredentialQueryOwned {
        service,
        account,
        access_group: normalize_optional_string(access_group),
        require_authentication: query.require_authentication,
    })
}

/// Decode one native credential write request into owned data.
pub(crate) fn decode_native_write_options(
    options: CredentialWriteOptions,
) -> RuntimeResult<CredentialWriteOptionsOwned> {
    let service = decode_native_string(options.service, "options.service")?;
    let account = decode_native_string(options.account, "options.account")?;
    let access_group = match options.access_group {
        Some(access_group) => Some(decode_native_string(access_group, "options.accessGroup")?),
        None => None,
    };
    let bytes = decode_native_bytes(options.bytes, "options.bytes")?;

    Ok(CredentialWriteOptionsOwned {
        service,
        account,
        access_group: normalize_optional_string(access_group),
        bytes,
        accessibility: options.accessibility,
        authentication: options.authentication,
        replace_existing: options.replace_existing,
    })
}

/// Decode one VM credential write request into owned data.
pub(crate) fn decode_vm_write_options(
    context: &mut vm::BindingContext<'_>,
    options: CredentialWriteOptionsVm,
) -> RuntimeResult<CredentialWriteOptionsOwned> {
    let service = decode_vm_string(context, options.service, "options.service")?;
    let account = decode_vm_string(context, options.account, "options.account")?;
    let access_group = match options.access_group {
        Some(access_group) => Some(decode_vm_string(
            context,
            access_group,
            "options.accessGroup",
        )?),
        None => None,
    };
    let bytes = decode_vm_bytes(context, options.bytes, "options.bytes")?;

    Ok(CredentialWriteOptionsOwned {
        service,
        account,
        access_group: normalize_optional_string(access_group),
        bytes,
        accessibility: options.accessibility,
        authentication: options.authentication,
        replace_existing: options.replace_existing,
    })
}

/// Decode one native authentication request into owned data.
pub(crate) fn decode_native_authentication_options(
    options: CredentialAuthenticationOptions,
) -> RuntimeResult<CredentialAuthenticationOptionsOwned> {
    let title = decode_native_string(options.title, "options.title")?;
    let subtitle = decode_native_string(options.subtitle, "options.subtitle")?;
    let message = decode_native_string(options.message, "options.message")?;

    Ok(CredentialAuthenticationOptionsOwned {
        title,
        subtitle,
        message,
        requirement: options.requirement,
    })
}

/// Decode one VM authentication request into owned data.
pub(crate) fn decode_vm_authentication_options(
    context: &mut vm::BindingContext<'_>,
    options: CredentialAuthenticationOptionsVm,
) -> RuntimeResult<CredentialAuthenticationOptionsOwned> {
    let title = decode_vm_string(context, options.title, "options.title")?;
    let subtitle = decode_vm_string(context, options.subtitle, "options.subtitle")?;
    let message = decode_vm_string(context, options.message, "options.message")?;

    Ok(CredentialAuthenticationOptionsOwned {
        title,
        subtitle,
        message,
        requirement: options.requirement,
    })
}

/// Store one native credential record in call-local backing storage.
pub(crate) fn store_native_record(
    binding: &BindingCallContext,
    record: &CredentialRecordOwned,
) -> CredentialRecord {
    CredentialRecord {
        service: binding.store_string(&record.service),
        account: binding.store_string(&record.account),
        bytes: binding.store_slice(record.bytes.clone()),
        created_unix_ns: record.created_unix_ns,
        modified_unix_ns: record.modified_unix_ns,
    }
}

/// Store one VM credential record in the external call context.
pub(crate) fn store_vm_record(
    context: &mut vm::BindingContext<'_>,
    record: &CredentialRecordOwned,
) -> RuntimeResult<CredentialRecordVm> {
    Ok(CredentialRecordVm {
        service: context
            .string_handle(&record.service)
            .map_err(Box::<RuntimeError>::from)?,
        account: context
            .string_handle(&record.account)
            .map_err(Box::<RuntimeError>::from)?,
        bytes: VmSlice::from_bytes(&mut context.write(), &record.bytes)?,
        created_unix_ns: record.created_unix_ns,
        modified_unix_ns: record.modified_unix_ns,
    })
}

/// Authenticate one credential request through the native ABI surface.
pub(crate) unsafe fn destack_os_credentials_authenticate_native(
    binding: &BindingCallContext,
    out: *mut CredentialAuthenticationResult,
    options: CredentialAuthenticationOptions,
) -> RuntimeResult<()> {
    // validate the output pointer before decoding arguments
    core_platform::ensure_out(out, "out")?;

    // decode and run one authentication request
    let options = decode_native_authentication_options(options)?;
    let result = authenticate_credentials(binding, &options)?;

    // write the normalized result
    unsafe {
        out.write(result);
    }

    Ok(())
}

/// Authenticate one credential request through the VM ABI surface.
pub(crate) fn destack_os_credentials_authenticate_vm(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    options: CredentialAuthenticationOptionsVm,
) -> RuntimeResult<CredentialAuthenticationResult> {
    // decode and run one authentication request
    let options = decode_vm_authentication_options(context, options)?;

    authenticate_credentials(binding, &options)
}

/// Query credential presence through the native ABI surface.
pub(crate) unsafe fn destack_os_credentials_contains_native(
    binding: &BindingCallContext,
    out: *mut bool,
    service: NativeStringRef,
    account: NativeStringRef,
    access_group: Option<NativeStringRef>,
) -> RuntimeResult<()> {
    // validate the output pointer before decoding arguments
    core_platform::ensure_out(out, "out")?;

    // decode the credential identity fields
    let service = decode_native_string(service, "service")?;
    let account = decode_native_string(account, "account")?;
    let access_group = match access_group {
        Some(access_group) => Some(decode_native_string(access_group, "accessgroup")?),
        None => None,
    };
    let access_group = normalize_optional_string(access_group);

    // query and write one presence result
    let is_present = contains_credentials(binding, &service, &account, access_group.as_deref())?;
    unsafe {
        out.write(is_present);
    }

    Ok(())
}

/// Query credential presence through the VM ABI surface.
pub(crate) fn destack_os_credentials_contains_vm(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    service: vm::StringHandle,
    account: vm::StringHandle,
    access_group: Option<vm::StringHandle>,
) -> RuntimeResult<bool> {
    // decode the credential identity fields
    let service = decode_vm_string(context, service, "service")?;
    let account = decode_vm_string(context, account, "account")?;
    let access_group = match access_group {
        Some(access_group) => Some(decode_vm_string(context, access_group, "accessgroup")?),
        None => None,
    };
    let access_group = normalize_optional_string(access_group);

    contains_credentials(binding, &service, &account, access_group.as_deref())
}

/// Delete one credential record through the native ABI surface.
pub(crate) unsafe fn destack_os_credentials_delete_native(
    binding: &BindingCallContext,
    service: NativeStringRef,
    account: NativeStringRef,
    access_group: Option<NativeStringRef>,
) -> RuntimeResult<()> {
    // decode the credential identity fields
    let service = decode_native_string(service, "service")?;
    let account = decode_native_string(account, "account")?;
    let access_group = match access_group {
        Some(access_group) => Some(decode_native_string(access_group, "accessgroup")?),
        None => None,
    };
    let access_group = normalize_optional_string(access_group);

    delete_credentials(binding, &service, &account, access_group.as_deref())
}

/// Delete one credential record through the VM ABI surface.
pub(crate) fn destack_os_credentials_delete_vm(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    service: vm::StringHandle,
    account: vm::StringHandle,
    access_group: Option<vm::StringHandle>,
) -> RuntimeResult<()> {
    // decode the credential identity fields
    let service = decode_vm_string(context, service, "service")?;
    let account = decode_vm_string(context, account, "account")?;
    let access_group = match access_group {
        Some(access_group) => Some(decode_vm_string(context, access_group, "accessgroup")?),
        None => None,
    };
    let access_group = normalize_optional_string(access_group);

    delete_credentials(binding, &service, &account, access_group.as_deref())
}

/// Read one credential record through the native ABI surface.
pub(crate) unsafe fn destack_os_credentials_read_native(
    binding: &BindingCallContext,
    out: *mut CredentialRecord,
    query: CredentialQuery,
) -> RuntimeResult<()> {
    // validate the output pointer before decoding arguments
    core_platform::ensure_out(out, "out")?;

    // decode, read, and encode the record
    let query = decode_native_query(query)?;
    let record = read_credentials(binding, &query)?;
    let record = store_native_record(binding, &record);
    unsafe {
        out.write(record);
    }

    Ok(())
}

/// Read one credential record through the VM ABI surface.
pub(crate) fn destack_os_credentials_read_vm(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    query: CredentialQueryVm,
) -> RuntimeResult<CredentialRecordVm> {
    // decode, read, and encode the record
    let query = decode_vm_query(context, query)?;
    let record = read_credentials(binding, &query)?;

    store_vm_record(context, &record)
}

/// Write one credential record through the native ABI surface.
pub(crate) unsafe fn destack_os_credentials_write_native(
    binding: &BindingCallContext,
    options: CredentialWriteOptions,
) -> RuntimeResult<()> {
    // decode and persist the credential payload
    let options = decode_native_write_options(options)?;

    write_credentials(binding, &options)
}

/// Write one credential record through the VM ABI surface.
pub(crate) fn destack_os_credentials_write_vm(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    options: CredentialWriteOptionsVm,
) -> RuntimeResult<()> {
    // decode and persist the credential payload
    let options = decode_vm_write_options(context, options)?;

    write_credentials(binding, &options)
}

/// Read one credential record from the active host backend.
pub(crate) fn read_credentials(
    binding: &BindingCallContext,
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
    super::target::read_credentials(binding, query)
}

/// Write one credential record to the active host backend.
pub(crate) fn write_credentials(
    binding: &BindingCallContext,
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
    super::target::write_credentials(binding, options)
}

/// Delete one credential record from the active host backend.
pub(crate) fn delete_credentials(
    binding: &BindingCallContext,
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
    super::target::delete_credentials(binding, service, account, access_group)
}

/// Return whether one credential record exists on the active host backend.
pub(crate) fn contains_credentials(
    binding: &BindingCallContext,
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
    super::target::contains_credentials(binding, service, account, access_group)
}

/// Run one host authentication challenge.
pub(crate) fn authenticate_credentials(
    binding: &BindingCallContext,
    options: &CredentialAuthenticationOptionsOwned,
) -> RuntimeResult<CredentialAuthenticationResult> {
    // validate prompt fields for host APIs that require text
    validate_authentication_prompt(options)?;

    // dispatch to the host backend
    super::target::authenticate_credentials(binding, options)
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

/// Runtime-local guard state for create-only credential writes.
#[cfg(any(target_os = "linux", target_os = "windows"))]
#[derive(Debug, Default)]
pub(crate) struct CredentialCreateGuard {
    /// In-process mutex for check-then-write serialization.
    lock: Mutex<()>,
}

/// Shared runtime-local create-only guard registry.
#[cfg(any(target_os = "linux", target_os = "windows"))]
static CREDENTIAL_CREATE_GUARDS: OnceLock<Mutex<HashMap<u64, Arc<CredentialCreateGuard>>>> =
    OnceLock::new();

/// Return the create-only guard for one host runtime.
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn credential_create_guard(binding: &BindingCallContext) -> Arc<CredentialCreateGuard> {
    // resolve the runtime-local guard registry
    let registry = CREDENTIAL_CREATE_GUARDS.get_or_init(|| Mutex::new(HashMap::new()));

    // resolve the current host runtime id
    let runtime_id = binding.host().host_session_id().0;
    let mut registry = registry.lock();

    // reuse the live guard when this runtime already owns one
    if let Some(guard) = registry.get(&runtime_id) {
        return Arc::clone(guard);
    }

    // otherwise create and cache a new runtime-local guard
    let guard = Arc::new(CredentialCreateGuard::default());
    registry.insert(runtime_id, Arc::clone(&guard));

    guard
}

/// Remove one runtime-local credential create guard.
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub(crate) fn unregister_credential_runtime(host_session_id: u64) {
    let registry = CREDENTIAL_CREATE_GUARDS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut registry = registry.lock();

    registry.remove(&host_session_id);
}

/// Execute one closure under one runtime-local create-only write guard.
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub(crate) fn with_credential_create_guard<R>(
    binding: &BindingCallContext,
    execute: impl FnOnce() -> RuntimeResult<R>,
) -> RuntimeResult<R> {
    // resolve one runtime-local lock holder for credential writes
    let guard = credential_create_guard(binding);

    // serialize one check-then-write sequence for this runtime
    let _guard = guard.lock.lock();

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
