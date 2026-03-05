use std::os::raw::c_void;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{ptr, slice};

use core_foundation_sys::base::{CFRelease, CFTypeRef};
use core_foundation_sys::data::{CFDataGetBytePtr, CFDataGetLength, CFDataRef};
use core_foundation_sys::number::kCFBooleanTrue;
use core_foundation_sys::string::CFStringRef;
use security_framework_sys::base::{errSecDuplicateItem, errSecItemNotFound, errSecSuccess};
use security_framework_sys::item::{
    kSecAttrAccessControl, kSecAttrAccessGroup, kSecAttrAccount, kSecAttrService, kSecClass,
    kSecClassGenericPassword, kSecReturnAttributes, kSecReturnData, kSecUseAuthenticationUI,
    kSecUseAuthenticationUISkip, kSecValueData,
};
use security_framework_sys::keychain_item::{SecItemAdd, SecItemCopyMatching, SecItemDelete};

use crate::diagnostic::RuntimeResult;
use crate::platform::os::{
    CredentialAuthenticationMechanism, CredentialAuthenticationRequirement,
    CredentialAuthenticationResult,
};
use crate::runtime::BindingCallContext;

use super::super::super::core::{
    CredentialAuthenticationOptionsOwned, CredentialQueryOwned, CredentialRecordOwned,
    CredentialWriteOptionsOwned, OS_CREDENTIALS_AUTHENTICATE_OPERATION,
    OS_CREDENTIALS_CONTAINS_OPERATION, OS_CREDENTIALS_DELETE_OPERATION,
    OS_CREDENTIALS_READ_OPERATION, OS_CREDENTIALS_WRITE_OPERATION, already_exists, invalid_data,
};
use super::core::{
    OwnedCfReference, create_authentication_access_control, create_cf_data, create_cf_string,
    create_dictionary, create_optional_access_control, create_optional_access_group,
    map_keychain_status,
};

/// Security status code for user-canceled operation.
const ERR_SEC_USER_CANCELED: i32 = -128;
/// Static account key for temporary authentication probe records.
const AUTHENTICATION_PROBE_ACCOUNT: &str = "authenticate";
/// Prefix for temporary authentication probe service keys.
const AUTHENTICATION_PROBE_SERVICE_PREFIX: &str = "destack.os.credentials.authenticate";
/// Temporary payload bytes for one authentication probe record.
const AUTHENTICATION_PROBE_BYTES: &[u8] = &[1u8];

// security key for operation prompt text
unsafe extern "C" {
    static kSecUseOperationPrompt: CFStringRef;
}

/// Read one credential record from the Apple keychain.
pub(crate) fn read_credentials(
    _binding: &BindingCallContext,
    query: &CredentialQueryOwned,
) -> RuntimeResult<CredentialRecordOwned> {
    // build keychain query attributes
    let service = create_cf_string(&query.service, OS_CREDENTIALS_READ_OPERATION)?;
    let account = create_cf_string(&query.account, OS_CREDENTIALS_READ_OPERATION)?;
    let access_group =
        create_optional_access_group(query.access_group.as_deref(), OS_CREDENTIALS_READ_OPERATION)?;
    let mut query_keys = vec![
        unsafe { kSecClass as *const c_void },
        unsafe { kSecAttrService as *const c_void },
        unsafe { kSecAttrAccount as *const c_void },
        unsafe { kSecReturnData as *const c_void },
    ];
    let mut query_values = vec![
        unsafe { kSecClassGenericPassword as *const c_void },
        service.as_ptr(),
        account.as_ptr(),
        unsafe { kCFBooleanTrue as *const c_void },
    ];

    // add optional access-group matching
    if let Some(access_group) = &access_group {
        query_keys.push(unsafe { kSecAttrAccessGroup as *const c_void });
        query_values.push(access_group.as_ptr());
    }

    // skip interactive prompts unless the caller explicitly requires authentication
    if !query.require_authentication {
        query_keys.push(unsafe { kSecUseAuthenticationUI as *const c_void });
        query_values.push(unsafe { kSecUseAuthenticationUISkip as *const c_void });
    }

    let query_dictionary = create_dictionary(
        &query_keys,
        &query_values,
        OS_CREDENTIALS_READ_OPERATION,
        "failed to create one keychain read query dictionary",
    )?;

    // execute one keychain read query
    let mut result: CFTypeRef = ptr::null();
    let status = unsafe { SecItemCopyMatching(query_dictionary.as_ptr().cast(), &mut result) };
    if status != errSecSuccess {
        return Err(map_keychain_status(OS_CREDENTIALS_READ_OPERATION, status));
    }
    if result.is_null() {
        return Err(invalid_data(
            OS_CREDENTIALS_READ_OPERATION,
            "SecItemCopyMatching returned one null value for a successful read",
        ));
    }
    let result = OwnedCfReference::new(
        result,
        OS_CREDENTIALS_READ_OPERATION,
        "null keychain read result",
    )?;

    // decode read bytes from one cfdata payload
    let data = result.as_ptr() as CFDataRef;
    let bytes_pointer = unsafe { CFDataGetBytePtr(data) };
    let bytes_length = unsafe { CFDataGetLength(data) };
    let bytes = if bytes_pointer.is_null() || bytes_length == 0 {
        Vec::new()
    } else {
        unsafe { slice::from_raw_parts(bytes_pointer, bytes_length as usize) }.to_vec()
    };

    Ok(CredentialRecordOwned {
        service: query.service.clone(),
        account: query.account.clone(),
        bytes,
        created_unix_ns: 0,
        modified_unix_ns: 0,
    })
}

/// Write one credential record to the Apple keychain.
pub(crate) fn write_credentials(
    _binding: &BindingCallContext,
    options: &CredentialWriteOptionsOwned,
) -> RuntimeResult<()> {
    // build keychain attribute objects
    let service = create_cf_string(&options.service, OS_CREDENTIALS_WRITE_OPERATION)?;
    let account = create_cf_string(&options.account, OS_CREDENTIALS_WRITE_OPERATION)?;
    let access_group = create_optional_access_group(
        options.access_group.as_deref(),
        OS_CREDENTIALS_WRITE_OPERATION,
    )?;
    let data = create_cf_data(&options.bytes, OS_CREDENTIALS_WRITE_OPERATION)?;
    let access_control = create_optional_access_control(options, OS_CREDENTIALS_WRITE_OPERATION)?;

    // compose one add query for this credential record
    let mut add_keys = vec![
        unsafe { kSecClass as *const c_void },
        unsafe { kSecAttrService as *const c_void },
        unsafe { kSecAttrAccount as *const c_void },
        unsafe { kSecValueData as *const c_void },
    ];
    let mut add_values = vec![
        unsafe { kSecClassGenericPassword as *const c_void },
        service.as_ptr(),
        account.as_ptr(),
        data.as_ptr(),
    ];

    // attach optional access-group selector
    if let Some(access_group) = &access_group {
        add_keys.push(unsafe { kSecAttrAccessGroup as *const c_void });
        add_values.push(access_group.as_ptr());
    }

    // attach optional access-control policy
    if let Some(access_control) = &access_control {
        add_keys.push(unsafe { kSecAttrAccessControl as *const c_void });
        add_values.push(access_control.as_ptr());
    }

    let add_query = create_dictionary(
        &add_keys,
        &add_values,
        OS_CREDENTIALS_WRITE_OPERATION,
        "failed to create one keychain add query dictionary",
    )?;

    // write one keychain record and handle duplicate behavior
    let add_status = unsafe { SecItemAdd(add_query.as_ptr().cast(), ptr::null_mut()) };
    if add_status == errSecSuccess {
        return Ok(());
    }
    if add_status == errSecDuplicateItem && !options.replace_existing {
        return Err(already_exists(
            OS_CREDENTIALS_WRITE_OPERATION,
            "credential already exists and replaceExisting is false",
        ));
    }
    if add_status != errSecDuplicateItem {
        return Err(map_keychain_status(
            OS_CREDENTIALS_WRITE_OPERATION,
            add_status,
        ));
    }

    // build one match query for this duplicate record
    let mut duplicate_match_keys = vec![
        unsafe { kSecClass as *const c_void },
        unsafe { kSecAttrService as *const c_void },
        unsafe { kSecAttrAccount as *const c_void },
    ];
    let mut duplicate_match_values = vec![
        unsafe { kSecClassGenericPassword as *const c_void },
        service.as_ptr(),
        account.as_ptr(),
    ];
    if let Some(access_group) = &access_group {
        duplicate_match_keys.push(unsafe { kSecAttrAccessGroup as *const c_void });
        duplicate_match_values.push(access_group.as_ptr());
    }
    let duplicate_match_query = create_dictionary(
        &duplicate_match_keys,
        &duplicate_match_values,
        OS_CREDENTIALS_WRITE_OPERATION,
        "failed to create one keychain duplicate-match dictionary",
    )?;

    // delete one existing duplicate so replacement updates policy metadata as well as bytes
    let delete_status = unsafe { SecItemDelete(duplicate_match_query.as_ptr().cast()) };
    if delete_status != errSecSuccess && delete_status != errSecItemNotFound {
        return Err(map_keychain_status(
            OS_CREDENTIALS_WRITE_OPERATION,
            delete_status,
        ));
    }

    // re-add the credential with full requested metadata and payload
    let replace_status = unsafe { SecItemAdd(add_query.as_ptr().cast(), ptr::null_mut()) };
    if replace_status != errSecSuccess {
        return Err(map_keychain_status(
            OS_CREDENTIALS_WRITE_OPERATION,
            replace_status,
        ));
    }

    Ok(())
}

/// Delete one credential record from the Apple keychain.
pub(crate) fn delete_credentials(
    _binding: &BindingCallContext,
    service: &str,
    account: &str,
    access_group: Option<&str>,
) -> RuntimeResult<()> {
    // build keychain query attributes
    let service = create_cf_string(service, OS_CREDENTIALS_DELETE_OPERATION)?;
    let account = create_cf_string(account, OS_CREDENTIALS_DELETE_OPERATION)?;
    let access_group = create_optional_access_group(access_group, OS_CREDENTIALS_DELETE_OPERATION)?;
    let mut query_keys = vec![
        unsafe { kSecClass as *const c_void },
        unsafe { kSecAttrService as *const c_void },
        unsafe { kSecAttrAccount as *const c_void },
        unsafe { kSecUseAuthenticationUI as *const c_void },
    ];
    let mut query_values = vec![
        unsafe { kSecClassGenericPassword as *const c_void },
        service.as_ptr(),
        account.as_ptr(),
        unsafe { kSecUseAuthenticationUISkip as *const c_void },
    ];

    // route one optional access-group selector
    if let Some(access_group) = &access_group {
        query_keys.push(unsafe { kSecAttrAccessGroup as *const c_void });
        query_values.push(access_group.as_ptr());
    }

    let query = create_dictionary(
        &query_keys,
        &query_values,
        OS_CREDENTIALS_DELETE_OPERATION,
        "failed to create one keychain delete query dictionary",
    )?;

    // execute one keychain delete query
    let status = unsafe { SecItemDelete(query.as_ptr().cast()) };
    if status != errSecSuccess {
        return Err(map_keychain_status(OS_CREDENTIALS_DELETE_OPERATION, status));
    }

    Ok(())
}

/// Return whether one credential record exists in the Apple keychain.
pub(crate) fn contains_credentials(
    _binding: &BindingCallContext,
    service: &str,
    account: &str,
    access_group: Option<&str>,
) -> RuntimeResult<bool> {
    // build keychain query attributes
    let service = create_cf_string(service, OS_CREDENTIALS_CONTAINS_OPERATION)?;
    let account = create_cf_string(account, OS_CREDENTIALS_CONTAINS_OPERATION)?;
    let access_group =
        create_optional_access_group(access_group, OS_CREDENTIALS_CONTAINS_OPERATION)?;
    let mut query_keys = vec![
        unsafe { kSecClass as *const c_void },
        unsafe { kSecAttrService as *const c_void },
        unsafe { kSecAttrAccount as *const c_void },
        unsafe { kSecReturnAttributes as *const c_void },
        unsafe { kSecUseAuthenticationUI as *const c_void },
    ];
    let mut query_values = vec![
        unsafe { kSecClassGenericPassword as *const c_void },
        service.as_ptr(),
        account.as_ptr(),
        unsafe { kCFBooleanTrue as *const c_void },
        unsafe { kSecUseAuthenticationUISkip as *const c_void },
    ];

    // route one optional access-group selector
    if let Some(access_group) = &access_group {
        query_keys.push(unsafe { kSecAttrAccessGroup as *const c_void });
        query_values.push(access_group.as_ptr());
    }

    let query = create_dictionary(
        &query_keys,
        &query_values,
        OS_CREDENTIALS_CONTAINS_OPERATION,
        "failed to create one keychain contains query dictionary",
    )?;

    // execute one keychain contains query
    let mut result: CFTypeRef = ptr::null();
    let status = unsafe { SecItemCopyMatching(query.as_ptr().cast(), &mut result) };
    if !result.is_null() {
        unsafe {
            CFRelease(result);
        }
    }
    if status == errSecSuccess {
        return Ok(true);
    }
    if status == errSecItemNotFound {
        return Ok(false);
    }

    Err(map_keychain_status(
        OS_CREDENTIALS_CONTAINS_OPERATION,
        status,
    ))
}

/// Run one host authentication challenge through Apple keychain APIs.
pub(crate) fn authenticate_credentials(
    _binding: &BindingCallContext,
    options: &CredentialAuthenticationOptionsOwned,
) -> RuntimeResult<CredentialAuthenticationResult> {
    // build one unique keychain probe identity
    let service_name = authentication_probe_service_name();
    let service = create_cf_string(&service_name, OS_CREDENTIALS_AUTHENTICATE_OPERATION)?;
    let account = create_cf_string(
        AUTHENTICATION_PROBE_ACCOUNT,
        OS_CREDENTIALS_AUTHENTICATE_OPERATION,
    )?;
    let prompt = create_cf_string(
        &authentication_prompt_text(options),
        OS_CREDENTIALS_AUTHENTICATE_OPERATION,
    )?;
    let access_control = create_authentication_access_control(
        options.requirement,
        OS_CREDENTIALS_AUTHENTICATE_OPERATION,
    )?;
    let payload = create_cf_data(
        AUTHENTICATION_PROBE_BYTES,
        OS_CREDENTIALS_AUTHENTICATE_OPERATION,
    )?;

    // add one temporary keychain item guarded by user-presence access control
    let add_keys = [
        unsafe { kSecClass as *const c_void },
        unsafe { kSecAttrService as *const c_void },
        unsafe { kSecAttrAccount as *const c_void },
        unsafe { kSecValueData as *const c_void },
        unsafe { kSecAttrAccessControl as *const c_void },
    ];
    let add_values = [
        unsafe { kSecClassGenericPassword as *const c_void },
        service.as_ptr(),
        account.as_ptr(),
        payload.as_ptr(),
        access_control.as_ptr(),
    ];
    let add_query = create_dictionary(
        &add_keys,
        &add_values,
        OS_CREDENTIALS_AUTHENTICATE_OPERATION,
        "failed to create one keychain authentication add query dictionary",
    )?;

    let add_status = unsafe { SecItemAdd(add_query.as_ptr().cast(), ptr::null_mut()) };
    if add_status != errSecSuccess {
        return Err(map_keychain_status(
            OS_CREDENTIALS_AUTHENTICATE_OPERATION,
            add_status,
        ));
    }

    // execute one authenticated read to trigger host prompt UI
    let read_keys = [
        unsafe { kSecClass as *const c_void },
        unsafe { kSecAttrService as *const c_void },
        unsafe { kSecAttrAccount as *const c_void },
        unsafe { kSecReturnData as *const c_void },
        unsafe { kSecUseOperationPrompt as *const c_void },
    ];
    let read_values = [
        unsafe { kSecClassGenericPassword as *const c_void },
        service.as_ptr(),
        account.as_ptr(),
        unsafe { kCFBooleanTrue as *const c_void },
        prompt.as_ptr(),
    ];
    let read_query = create_dictionary(
        &read_keys,
        &read_values,
        OS_CREDENTIALS_AUTHENTICATE_OPERATION,
        "failed to create one keychain authentication read query dictionary",
    )?;

    let mut result: CFTypeRef = ptr::null();
    let read_status = unsafe { SecItemCopyMatching(read_query.as_ptr().cast(), &mut result) };
    if !result.is_null() {
        unsafe {
            CFRelease(result);
        }
    }

    // delete the temporary probe item, ignore cleanup failures to preserve auth result semantics
    let _cleanup = delete_authentication_probe_item(&service, &account);

    // map successful host authentication
    let mechanism = authentication_mechanism_for_requirement(options.requirement);
    if read_status == errSecSuccess {
        return Ok(CredentialAuthenticationResult {
            authenticated: true,
            mechanism,
        });
    }

    // map expected auth-denied and user-cancel paths into a false authentication result
    if read_status == ERR_SEC_USER_CANCELED
        || read_status == security_framework_sys::base::errSecAuthFailed
    {
        return Ok(CredentialAuthenticationResult {
            authenticated: false,
            mechanism,
        });
    }

    Err(map_keychain_status(
        OS_CREDENTIALS_AUTHENTICATE_OPERATION,
        read_status,
    ))
}

/// Return the effective authentication mechanism for one requirement policy.
fn authentication_mechanism_for_requirement(
    requirement: CredentialAuthenticationRequirement,
) -> CredentialAuthenticationMechanism {
    match requirement {
        CredentialAuthenticationRequirement::Biometric => {
            CredentialAuthenticationMechanism::Biometric
        }
        CredentialAuthenticationRequirement::DeviceCredential => {
            CredentialAuthenticationMechanism::DeviceCredential
        }
        CredentialAuthenticationRequirement::BiometricOrDeviceCredential => {
            CredentialAuthenticationMechanism::Unknown
        }
    }
}

/// Build one best-effort authentication prompt string.
fn authentication_prompt_text(options: &CredentialAuthenticationOptionsOwned) -> String {
    // compose non-empty prompt fields into one host-visible message block
    let mut lines = Vec::with_capacity(3);
    if !options.title.is_empty() {
        lines.push(options.title.as_str());
    }
    if !options.subtitle.is_empty() {
        lines.push(options.subtitle.as_str());
    }
    if !options.message.is_empty() {
        lines.push(options.message.as_str());
    }

    lines.join("\n")
}

/// Build one unique service key for authentication probe records.
fn authentication_probe_service_name() -> String {
    let unix_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);

    format!("{AUTHENTICATION_PROBE_SERVICE_PREFIX}.{unix_nanos}")
}

/// Delete one temporary authentication probe record.
fn delete_authentication_probe_item(
    service: &OwnedCfReference,
    account: &OwnedCfReference,
) -> RuntimeResult<()> {
    let delete_keys = [
        unsafe { kSecClass as *const c_void },
        unsafe { kSecAttrService as *const c_void },
        unsafe { kSecAttrAccount as *const c_void },
    ];
    let delete_values = [
        unsafe { kSecClassGenericPassword as *const c_void },
        service.as_ptr(),
        account.as_ptr(),
    ];
    let delete_query = create_dictionary(
        &delete_keys,
        &delete_values,
        OS_CREDENTIALS_AUTHENTICATE_OPERATION,
        "failed to create one keychain authentication cleanup query dictionary",
    )?;

    let delete_status = unsafe { SecItemDelete(delete_query.as_ptr().cast()) };
    if delete_status == errSecSuccess || delete_status == errSecItemNotFound {
        return Ok(());
    }

    Err(map_keychain_status(
        OS_CREDENTIALS_AUTHENTICATE_OPERATION,
        delete_status,
    ))
}
