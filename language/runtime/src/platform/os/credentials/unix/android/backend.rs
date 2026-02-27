use crate::diagnostic::RuntimeResult;
use crate::host::{
    HOST_STATUS_BUFFER_TOO_SMALL, destack_host_android_credentials_authenticate,
    destack_host_android_credentials_contains, destack_host_android_credentials_delete,
    destack_host_android_credentials_read, destack_host_android_credentials_write,
};
use crate::platform::NativeSlice;
use crate::platform::os::{CredentialAuthenticationMechanism, CredentialAuthenticationResult};
use crate::runtime::BindingCallContext;

use super::super::super::core::{
    CredentialAuthenticationOptionsOwned, CredentialQueryOwned, CredentialRecordOwned,
    CredentialWriteOptionsOwned, OS_CREDENTIALS_AUTHENTICATE_OPERATION,
    OS_CREDENTIALS_CONTAINS_OPERATION, OS_CREDENTIALS_DELETE_OPERATION,
    OS_CREDENTIALS_READ_OPERATION, OS_CREDENTIALS_WRITE_OPERATION, invalid_data,
};
use super::core::{callback_runtime_id, decode_authentication_mechanism, host_status_result};

/// Initial scratch buffer size for Android host credential reads.
const INITIAL_ANDROID_READ_BUFFER_BYTES: usize = 512;
/// Maximum output buffer size accepted from Android host credential reads.
const MAX_ANDROID_READ_BUFFER_BYTES: usize = 1024 * 1024;

/// Read one credential record through Android host callback APIs.
pub(crate) fn read_credentials(
    context: &BindingCallContext,
    query: &CredentialQueryOwned,
) -> RuntimeResult<CredentialRecordOwned> {
    // resolve callback runtime identifier and callback function
    let runtime_id = callback_runtime_id(context, OS_CREDENTIALS_READ_OPERATION)?;

    // encode string arguments for host callback ABI
    let service = context.store_string(&query.service);
    let account = context.store_string(&query.account);
    let access_group = context.store_string_option(query.access_group.as_ref());

    // call host read with dynamic output-buffer growth
    let mut created_unix_ns = 0u64;
    let mut modified_unix_ns = 0u64;
    let mut bytes = vec![0u8; INITIAL_ANDROID_READ_BUFFER_BYTES];

    loop {
        // convert the dynamic output buffer size for host callback ABI width
        let output_capacity =
            checked_u32_length(bytes.len(), OS_CREDENTIALS_READ_OPERATION, "output buffer")?;

        let mut output_written = 0u32;
        let status = unsafe {
            destack_host_android_credentials_read(
                runtime_id,
                service,
                account,
                access_group,
                query.require_authentication,
                NativeSlice {
                    data: bytes.as_mut_ptr(),
                    len: output_capacity,
                },
                &mut output_written,
                &mut created_unix_ns,
                &mut modified_unix_ns,
            )
        };

        // grow output buffer when host reports required capacity
        if status == HOST_STATUS_BUFFER_TOO_SMALL {
            if output_written == 0 {
                return Err(invalid_data(
                    OS_CREDENTIALS_READ_OPERATION,
                    "android host credentials read reported buffer-too-small without required size",
                ));
            }

            let required_output_bytes = output_written as usize;
            if required_output_bytes <= bytes.len() {
                return Err(invalid_data(
                    OS_CREDENTIALS_READ_OPERATION,
                    "android host credentials read reported non-growing buffer-too-small capacity",
                ));
            }
            if required_output_bytes > MAX_ANDROID_READ_BUFFER_BYTES {
                return Err(invalid_data(
                    OS_CREDENTIALS_READ_OPERATION,
                    format!(
                        "android host credentials read reported one required size above max {MAX_ANDROID_READ_BUFFER_BYTES} bytes"
                    ),
                ));
            }

            bytes.resize(required_output_bytes, 0);
            continue;
        }

        // map host callback status and finalize output bytes
        host_status_result(status, OS_CREDENTIALS_READ_OPERATION, "read")?;

        if output_written as usize > bytes.len() {
            return Err(invalid_data(
                OS_CREDENTIALS_READ_OPERATION,
                "android host credentials read reported one output length larger than buffer capacity",
            ));
        }

        bytes.truncate(output_written as usize);
        break;
    }

    Ok(CredentialRecordOwned {
        service: query.service.clone(),
        account: query.account.clone(),
        bytes,
        created_unix_ns,
        modified_unix_ns,
    })
}

/// Write one credential record through Android host callback APIs.
pub(crate) fn write_credentials(
    context: &BindingCallContext,
    options: &CredentialWriteOptionsOwned,
) -> RuntimeResult<()> {
    // resolve callback runtime identifier and callback function
    let runtime_id = callback_runtime_id(context, OS_CREDENTIALS_WRITE_OPERATION)?;

    // encode string and payload arguments for host callback ABI
    let service = context.store_string(&options.service);
    let account = context.store_string(&options.account);
    let access_group = context.store_string_option(options.access_group.as_ref());

    // convert payload length for host callback ABI width
    let payload_length = checked_u32_length(
        options.bytes.len(),
        OS_CREDENTIALS_WRITE_OPERATION,
        "options.bytes",
    )?;

    let status = unsafe {
        destack_host_android_credentials_write(
            runtime_id,
            service,
            account,
            access_group,
            NativeSlice {
                data: options.bytes.as_ptr().cast_mut(),
                len: payload_length,
            },
            options.accessibility as u32,
            options.authentication as u32,
            options.replace_existing,
        )
    };

    // map host callback status into runtime result
    host_status_result(status, OS_CREDENTIALS_WRITE_OPERATION, "write")
}

/// Delete one credential record through Android host callback APIs.
pub(crate) fn delete_credentials(
    context: &BindingCallContext,
    service: &str,
    account: &str,
    access_group: Option<&str>,
) -> RuntimeResult<()> {
    // resolve callback runtime identifier and callback function
    let runtime_id = callback_runtime_id(context, OS_CREDENTIALS_DELETE_OPERATION)?;

    // encode string arguments for host callback ABI
    let service = context.store_string(service);
    let account = context.store_string(account);
    let access_group = access_group.map(str::to_owned);
    let access_group = context.store_string_option(access_group.as_ref());

    let status = unsafe {
        destack_host_android_credentials_delete(runtime_id, service, account, access_group)
    };

    // map host callback status into runtime result
    host_status_result(status, OS_CREDENTIALS_DELETE_OPERATION, "delete")
}

/// Return whether one credential record exists through Android host callback APIs.
pub(crate) fn contains_credentials(
    context: &BindingCallContext,
    service: &str,
    account: &str,
    access_group: Option<&str>,
) -> RuntimeResult<bool> {
    // resolve callback runtime identifier and callback function
    let runtime_id = callback_runtime_id(context, OS_CREDENTIALS_CONTAINS_OPERATION)?;

    // encode string arguments for host callback ABI
    let service = context.store_string(service);
    let account = context.store_string(account);
    let access_group = access_group.map(str::to_owned);
    let access_group = context.store_string_option(access_group.as_ref());

    let mut is_present = false;
    let status = unsafe {
        destack_host_android_credentials_contains(
            runtime_id,
            service,
            account,
            access_group,
            &mut is_present,
        )
    };

    // map host callback status and return output flag
    host_status_result(status, OS_CREDENTIALS_CONTAINS_OPERATION, "contains")?;

    Ok(is_present)
}

/// Run one host authentication challenge through Android host callback APIs.
pub(crate) fn authenticate_credentials(
    context: &BindingCallContext,
    options: &CredentialAuthenticationOptionsOwned,
) -> RuntimeResult<CredentialAuthenticationResult> {
    // resolve callback runtime identifier and callback function
    let runtime_id = callback_runtime_id(context, OS_CREDENTIALS_AUTHENTICATE_OPERATION)?;

    // encode prompt fields for host callback ABI
    let title = context.store_string(&options.title);
    let subtitle = context.store_string(&options.subtitle);
    let message = context.store_string(&options.message);

    let mut authenticated = false;
    let mut mechanism_code = CredentialAuthenticationMechanism::Unknown as u32;
    let status = unsafe {
        destack_host_android_credentials_authenticate(
            runtime_id,
            title,
            subtitle,
            message,
            options.requirement as u32,
            &mut authenticated,
            &mut mechanism_code,
        )
    };

    // map host callback status and decode mechanism value
    host_status_result(
        status,
        OS_CREDENTIALS_AUTHENTICATE_OPERATION,
        "authenticate",
    )?;

    let mechanism =
        decode_authentication_mechanism(mechanism_code, OS_CREDENTIALS_AUTHENTICATE_OPERATION)?;

    Ok(CredentialAuthenticationResult {
        authenticated,
        mechanism,
    })
}

/// Convert one native length to one u32 host callback ABI length.
fn checked_u32_length(
    length: usize,
    operation: &'static str,
    field: &'static str,
) -> RuntimeResult<u32> {
    u32::try_from(length).map_err(|_| {
        invalid_data(
            operation,
            format!("android host credentials {field} length exceeds u32::MAX"),
        )
    })
}
