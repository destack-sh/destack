use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::os::{
    CredentialAuthenticationOptions, CredentialAuthenticationResult, CredentialQuery,
    CredentialRecord, CredentialWriteOptions,
};
use crate::platform::{NativeStringRef, PlatformError};
use crate::runtime::BindingCallContext;

use super::credentials::{
    CredentialAuthenticationOptionsOwned, CredentialQueryOwned, CredentialWriteOptionsOwned,
    authenticate_credentials, contains_credentials, decode_native_bytes, decode_native_string,
    delete_credentials, normalize_optional_string, read_credentials, write_credentials,
};
pub(crate) use super::host::*;
pub(crate) use super::info::*;
pub(crate) use super::power::*;
pub(crate) use super::unsupported::*;

/// Request one host credential authentication challenge.
///
/// Request one host authentication challenge and return challenge outcome.
///
/// # Platform
/// Unix and Windows.
/// Uses keychain authentication prompts on Apple platforms, host callback bridge lanes on Android, and Windows CredUI prompt lanes for `BiometricOrDeviceCredential`.
/// Windows `Biometric` requests use Windows Biometric Framework lanes where available.
/// Windows `DeviceCredential` requests use CredUI prompt plus host logon verification lanes.
/// Linux and other unsupported Unix hosts return `notSupported`.
///
/// # Errors
/// Returns invalidArgumentValue, ioPermissionDenied, ioWouldBlock, ioInterrupted, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.credentials.auth`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_credentials_authenticate(
    context: &BindingCallContext,
    out: *mut CredentialAuthenticationResult,
    options: CredentialAuthenticationOptions,
) -> RuntimeResult<()> {
    // reject null output pointers
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode native authentication options into owned values
    let options = CredentialAuthenticationOptionsOwned {
        title: decode_native_string(options.title, "options.title")?,
        subtitle: decode_native_string(options.subtitle, "options.subtitle")?,
        message: decode_native_string(options.message, "options.message")?,
        requirement: options.requirement,
    };

    // execute one authentication challenge
    let result = authenticate_credentials(context, &options)?;

    // write the output payload
    unsafe {
        out.write(result);
    }

    Ok(())
}

/// Query credential presence.
///
/// Return whether one credential record exists for one service and account pair.
///
/// # Platform
/// Unix and Windows.
/// Uses host credential-query APIs.
/// Optional access-group routing is honored on Apple keychain backends and Android host callback backends.
/// Returns `notSupported` on backends without access-group lanes.
///
/// # Errors
/// Returns invalidArgumentValue, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.credentials.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_credentials_contains(
    context: &BindingCallContext,
    out: *mut bool,
    service: NativeStringRef,
    account: NativeStringRef,
    access_group: NativeStringRef,
) -> RuntimeResult<()> {
    // reject null output pointers
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode native service, account, and optional access-group values
    let service = decode_native_string(service, "service")?;
    let account = decode_native_string(account, "account")?;
    let access_group =
        normalize_optional_string(decode_native_string(access_group, "accessGroup")?);

    // execute one contains query
    let result = contains_credentials(context, &service, &account, access_group.as_deref())?;

    // write the output payload
    unsafe {
        out.write(result);
    }

    Ok(())
}

/// Delete one credential record.
///
/// Delete one secure credential payload from host credential store.
///
/// # Platform
/// Unix and Windows.
/// Uses host credential-delete APIs.
/// Optional access-group routing is honored on Apple keychain backends and Android host callback backends.
/// Returns `notSupported` on backends without access-group lanes.
///
/// # Errors
/// Returns invalidArgumentValue, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.credentials.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_credentials_delete(
    context: &BindingCallContext,
    service: NativeStringRef,
    account: NativeStringRef,
    access_group: NativeStringRef,
) -> RuntimeResult<()> {
    // decode native service, account, and optional access-group values
    let service = decode_native_string(service, "service")?;
    let account = decode_native_string(account, "account")?;
    let access_group =
        normalize_optional_string(decode_native_string(access_group, "accessGroup")?);

    // execute one delete operation
    delete_credentials(context, &service, &account, access_group.as_deref())
}

/// Read one credential record.
///
/// Read one secure credential payload from host credential store.
///
/// # Platform
/// Unix and Windows.
/// Uses Keychain on Apple platforms, host callback bridge lanes on Android, Windows Credential Manager, and Linux keyutils plus Secret Service credential stores where available.
///
/// # Errors
/// Returns invalidArgumentValue, ioNotFound, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.credentials.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_credentials_read(
    context: &BindingCallContext,
    out: *mut CredentialRecord,
    query: CredentialQuery,
) -> RuntimeResult<()> {
    // reject null output pointers
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode native query payload into owned values
    let query = CredentialQueryOwned {
        service: decode_native_string(query.service, "query.service")?,
        account: decode_native_string(query.account, "query.account")?,
        access_group: normalize_optional_string(decode_native_string(
            query.access_group,
            "query.accessGroup",
        )?),
        require_authentication: query.require_authentication,
    };

    // execute one read operation
    let record = read_credentials(context, &query)?;

    // encode output payload in call-local storage
    let output = CredentialRecord {
        service: context.store_string(&record.service),
        account: context.store_string(&record.account),
        bytes: context.store_slice(record.bytes),
        created_unix_ns: record.created_unix_ns,
        modified_unix_ns: record.modified_unix_ns,
    };

    // write the output payload
    unsafe {
        out.write(output);
    }

    Ok(())
}

/// Write one credential record.
///
/// Create or replace one secure credential payload in host credential store.
///
/// # Platform
/// Unix and Windows.
/// Uses host credential-write APIs.
/// `replaceExisting=false` is strict within one runtime process and best effort across concurrent external writers.
///
/// # Errors
/// Returns invalidArgumentValue, ioAlreadyExists, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.credentials.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_credentials_write(
    context: &BindingCallContext,
    options: CredentialWriteOptions,
) -> RuntimeResult<()> {
    // decode native write options into owned values
    let options = CredentialWriteOptionsOwned {
        service: decode_native_string(options.service, "options.service")?,
        account: decode_native_string(options.account, "options.account")?,
        access_group: normalize_optional_string(decode_native_string(
            options.access_group,
            "options.accessGroup",
        )?),
        bytes: decode_native_bytes(options.bytes, "options.bytes")?,
        accessibility: options.accessibility,
        authentication: options.authentication,
        replace_existing: options.replace_existing,
    };

    // execute one write operation
    write_credentials(context, &options)
}
