use std::{ptr, slice};

use crate::diagnostic::RuntimeError;
use windows_sys::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_BAD_LENGTH, ERROR_INVALID_PARAMETER, ERROR_NO_SUCH_LOGON_SESSION,
    ERROR_NOT_FOUND, FILETIME, GetLastError,
};
use windows_sys::Win32::Security::Credentials::{
    CRED_MAX_CREDENTIAL_BLOB_SIZE, CRED_PERSIST_LOCAL_MACHINE, CRED_PERSIST_SESSION,
    CRED_TYPE_GENERIC, CREDENTIALW, CredDeleteW, CredFree, CredReadW, CredWriteW,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::core::wide_from_str;
use crate::platform::os::{
    CredentialAccessibility, CredentialAuthenticationPolicy, CredentialAuthenticationResult,
};
use crate::runtime::BindingCallContext;

use super::super::core::{
    CredentialAuthenticationOptionsOwned, CredentialQueryOwned, CredentialRecordOwned,
    CredentialWriteOptionsOwned, OS_CREDENTIALS_AUTHENTICATE_OPERATION,
    OS_CREDENTIALS_CONTAINS_OPERATION, OS_CREDENTIALS_DELETE_OPERATION,
    OS_CREDENTIALS_READ_OPERATION, OS_CREDENTIALS_WRITE_OPERATION, already_exists,
    invalid_argument, invalid_data, no_replace_write_guard, not_found, not_supported,
    permission_denied,
};

/// Windows FILETIME to unix epoch offset in 100ns ticks.
const WINDOWS_EPOCH_OFFSET_100NS: u64 = 116_444_736_000_000_000;
/// Field name for one query target conversion.
const QUERY_TARGET_FIELD: &str = "query.target";
/// Field name for one credential target conversion.
const CREDENTIAL_TARGET_FIELD: &str = "credential.target";
/// Field name for one write target conversion.
const OPTIONS_TARGET_FIELD: &str = "options.target";
/// Field name for one write account conversion.
const OPTIONS_ACCOUNT_FIELD: &str = "options.account";

/// Read one credential record from the Windows credential manager.
pub(crate) fn read_credentials(
    _context: &BindingCallContext,
    query: &CredentialQueryOwned,
) -> RuntimeResult<CredentialRecordOwned> {
    // reject auth-required reads because credman has no per-read challenge lane
    if query.require_authentication {
        return Err(not_supported(OS_CREDENTIALS_READ_OPERATION));
    }

    // reject access-group routes because windows credential manager has no access-group model
    if query.access_group.is_some() {
        return Err(not_supported(OS_CREDENTIALS_READ_OPERATION));
    }

    // build target lookup name
    let target_name = target_name_for_service_account(&query.service, &query.account)?;
    let target_name = wide_from_str(QUERY_TARGET_FIELD, &target_name)?;

    // execute one credential lookup
    let mut credential: *mut CREDENTIALW = ptr::null_mut();
    let status = unsafe { CredReadW(target_name.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) };
    if status == 0 {
        return Err(map_windows_credentials_error(
            OS_CREDENTIALS_READ_OPERATION,
            unsafe { GetLastError() },
        ));
    }
    if credential.is_null() {
        return Err(invalid_data(
            OS_CREDENTIALS_READ_OPERATION,
            "CredReadW returned one null credential pointer",
        ));
    }

    // decode blob payload and last-write timestamp
    let record = unsafe {
        let credential_ref = &*credential;
        let bytes =
            if credential_ref.CredentialBlob.is_null() || credential_ref.CredentialBlobSize == 0 {
                Vec::new()
            } else {
                let bytes = slice::from_raw_parts(
                    credential_ref.CredentialBlob,
                    credential_ref.CredentialBlobSize as usize,
                );
                bytes.to_vec()
            };
        let modified_unix_ns = filetime_to_unix_ns(credential_ref.LastWritten);

        CredentialRecordOwned {
            service: query.service.clone(),
            account: query.account.clone(),
            bytes,
            created_unix_ns: 0,
            modified_unix_ns,
        }
    };

    // release the credential manager allocation
    unsafe {
        CredFree(credential.cast());
    }

    Ok(record)
}

/// Write one credential record through the Windows credential manager.
pub(crate) fn write_credentials(
    context: &BindingCallContext,
    options: &CredentialWriteOptionsOwned,
) -> RuntimeResult<()> {
    // reject access-group routes because windows credential manager has no access-group model
    if options.access_group.is_some() {
        return Err(not_supported(OS_CREDENTIALS_WRITE_OPERATION));
    }

    // reject authentication policies that the generic credential lane cannot represent
    if options.authentication != CredentialAuthenticationPolicy::None {
        return Err(not_supported(OS_CREDENTIALS_WRITE_OPERATION));
    }

    // enforce credential manager blob limits
    if options.bytes.len() > CRED_MAX_CREDENTIAL_BLOB_SIZE as usize {
        return Err(invalid_argument(
            "options.bytes",
            format!(
                "credential payload exceeds windows max blob size {CRED_MAX_CREDENTIAL_BLOB_SIZE}"
            ),
        ));
    }

    // reject duplicate writes when replacement is disabled
    if !options.replace_existing {
        // serialize check-then-write in-process: this is strict per process, not cross-process atomic
        let _guard = no_replace_write_guard();
        let already_present =
            contains_credentials(context, &options.service, &options.account, None)?;
        if already_present {
            return Err(already_exists(
                OS_CREDENTIALS_WRITE_OPERATION,
                "credential already exists and replaceExisting is false",
            ));
        }

        // write one new credential while the no-replace guard is held
        return write_credential_record(options);
    }

    // write one replacement credential payload
    write_credential_record(options)
}

/// Delete one credential record from the Windows credential manager.
pub(crate) fn delete_credentials(
    _context: &BindingCallContext,
    service: &str,
    account: &str,
    access_group: Option<&str>,
) -> RuntimeResult<()> {
    // reject access-group routes because windows credential manager has no access-group model
    if access_group.is_some() {
        return Err(not_supported(OS_CREDENTIALS_DELETE_OPERATION));
    }

    // build target lookup name
    let target_name = target_name_for_service_account(service, account)?;
    let target_name = wide_from_str(CREDENTIAL_TARGET_FIELD, &target_name)?;

    // delete one credential entry
    let status = unsafe { CredDeleteW(target_name.as_ptr(), CRED_TYPE_GENERIC, 0) };
    if status == 0 {
        return Err(map_windows_credentials_error(
            OS_CREDENTIALS_DELETE_OPERATION,
            unsafe { GetLastError() },
        ));
    }

    Ok(())
}

/// Return whether one credential record exists in the Windows credential manager.
pub(crate) fn contains_credentials(
    _context: &BindingCallContext,
    service: &str,
    account: &str,
    access_group: Option<&str>,
) -> RuntimeResult<bool> {
    // reject access-group routes because windows credential manager has no access-group model
    if access_group.is_some() {
        return Err(not_supported(OS_CREDENTIALS_CONTAINS_OPERATION));
    }

    // build target lookup name
    let target_name = target_name_for_service_account(service, account)?;
    let target_name = wide_from_str(CREDENTIAL_TARGET_FIELD, &target_name)?;

    // execute one credential lookup
    let mut credential: *mut CREDENTIALW = ptr::null_mut();
    let status = unsafe { CredReadW(target_name.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) };
    if status == 0 {
        let error_code = unsafe { GetLastError() };

        // missing credentials map to false for contains semantics
        if error_code == ERROR_NOT_FOUND {
            return Ok(false);
        }

        return Err(map_windows_credentials_error(
            OS_CREDENTIALS_CONTAINS_OPERATION,
            error_code,
        ));
    }

    // release one successful credential lookup allocation
    if !credential.is_null() {
        unsafe {
            CredFree(credential.cast());
        }
    }

    Ok(true)
}

/// Run one host authentication challenge on Windows.
pub(crate) fn authenticate_credentials(
    _context: &BindingCallContext,
    _options: &CredentialAuthenticationOptionsOwned,
) -> RuntimeResult<CredentialAuthenticationResult> {
    Err(not_supported(OS_CREDENTIALS_AUTHENTICATE_OPERATION))
}

/// Build one deterministic credential manager target name.
fn target_name_for_service_account(service: &str, account: &str) -> RuntimeResult<String> {
    // encode the service length to keep the composite key unambiguous
    if service.len() > u32::MAX as usize {
        return Err(invalid_argument(
            "service",
            "service name exceeds supported credential key length",
        ));
    }

    Ok(format!(
        "destack.os.credentials:{}:{}:{}",
        service.len(),
        service,
        account
    ))
}

/// Write one Windows credential manager record.
fn write_credential_record(options: &CredentialWriteOptionsOwned) -> RuntimeResult<()> {
    // build credential manager target and user names
    let target_name = target_name_for_service_account(&options.service, &options.account)?;
    let target_name = wide_from_str(OPTIONS_TARGET_FIELD, &target_name)?;
    let user_name = wide_from_str(OPTIONS_ACCOUNT_FIELD, &options.account)?;

    // map persistence to one windows lane
    let persist = match options.accessibility {
        CredentialAccessibility::WhenUnlocked => CRED_PERSIST_SESSION,
        CredentialAccessibility::AfterFirstUnlock | CredentialAccessibility::HostDefault => {
            CRED_PERSIST_LOCAL_MACHINE
        }
    };

    // construct one native credential payload and write it
    let mut credential = CREDENTIALW {
        Flags: 0,
        Type: CRED_TYPE_GENERIC,
        TargetName: target_name.as_ptr() as *mut u16,
        Comment: ptr::null_mut(),
        LastWritten: FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        },
        CredentialBlobSize: options.bytes.len() as u32,
        CredentialBlob: options.bytes.as_ptr() as *mut u8,
        Persist: persist,
        AttributeCount: 0,
        Attributes: ptr::null_mut(),
        TargetAlias: ptr::null_mut(),
        UserName: user_name.as_ptr() as *mut u16,
    };
    let status = unsafe { CredWriteW(&mut credential as *mut CREDENTIALW, 0) };
    if status == 0 {
        return Err(map_windows_credentials_error(
            OS_CREDENTIALS_WRITE_OPERATION,
            unsafe { GetLastError() },
        ));
    }

    Ok(())
}

/// Convert one FILETIME value into unix nanoseconds.
fn filetime_to_unix_ns(filetime: FILETIME) -> u64 {
    // compose the 64-bit windows timestamp value from high and low words
    let timestamp_100ns = ((filetime.dwHighDateTime as u64) << 32) | filetime.dwLowDateTime as u64;

    // clamp values before the unix epoch to zero
    if timestamp_100ns <= WINDOWS_EPOCH_OFFSET_100NS {
        return 0;
    }

    // convert windows 100ns ticks to unix nanoseconds
    let unix_100ns = timestamp_100ns - WINDOWS_EPOCH_OFFSET_100NS;
    unix_100ns.saturating_mul(100)
}

/// Map one windows credentials error code into one runtime error.
fn map_windows_credentials_error(operation: &'static str, error_code: u32) -> Box<RuntimeError> {
    // map record-not-found errors
    if error_code == ERROR_NOT_FOUND {
        return not_found(operation, "credential record not found");
    }

    // map access-denied and unavailable-session errors
    if error_code == ERROR_ACCESS_DENIED || error_code == ERROR_NO_SUCH_LOGON_SESSION {
        return permission_denied(
            operation,
            format!("windows credential manager denied operation with code {error_code}"),
        );
    }

    // map malformed argument and blob-length errors
    if error_code == ERROR_INVALID_PARAMETER || error_code == ERROR_BAD_LENGTH {
        return invalid_argument(
            "credential",
            format!(
                "windows credential manager rejected one credential argument with code {error_code}"
            ),
        );
    }

    // map unknown host errors into one generic ioInvalidData lane
    invalid_data(
        operation,
        format!("windows credential manager failed with code {error_code}"),
    )
}
