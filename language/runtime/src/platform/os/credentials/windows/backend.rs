use std::ffi::c_void;
use std::{mem, ptr, slice};

use crate::diagnostic::RuntimeError;
use windows_sys::Win32::Devices::BiometricFramework::{
    WINBIO_E_CANCELED, WINBIO_E_DEVICE_BUSY, WINBIO_E_DISABLED, WINBIO_E_NO_MATCH,
    WINBIO_E_NOT_ACTIVE_CONSOLE, WINBIO_E_SENSOR_UNAVAILABLE, WINBIO_E_SESSION_BUSY,
    WINBIO_E_UNSUPPORTED_FACTOR, WINBIO_E_UNSUPPORTED_POOL_TYPE, WINBIO_IDENTITY,
    WINBIO_POOL_SYSTEM, WinBioCloseSession, WinBioIdentify, WinBioOpenSession,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_ACCOUNT_DISABLED, ERROR_ACCOUNT_LOCKED_OUT,
    ERROR_ACCOUNT_RESTRICTION, ERROR_BAD_LENGTH, ERROR_CANCELLED, ERROR_INSUFFICIENT_BUFFER,
    ERROR_INVALID_LOGON_HOURS, ERROR_INVALID_PARAMETER, ERROR_INVALID_WORKSTATION,
    ERROR_LOGON_FAILURE, ERROR_LOGON_TYPE_NOT_GRANTED, ERROR_NO_SUCH_LOGON_SESSION,
    ERROR_NOT_FOUND, ERROR_NOT_SUPPORTED, ERROR_PASSWORD_EXPIRED, ERROR_TIMEOUT, FILETIME,
    GetLastError, HANDLE,
};
use windows_sys::Win32::Security::Credentials::{
    CRED_MAX_CREDENTIAL_BLOB_SIZE, CRED_PACK_PROTECTED_CREDENTIALS, CRED_PERSIST_LOCAL_MACHINE,
    CRED_PERSIST_SESSION, CRED_TYPE_GENERIC, CREDENTIALW, CREDUI_INFOW, CREDUIWIN_GENERIC,
    CREDUIWIN_SECURE_PROMPT, CredDeleteW, CredFree, CredReadW, CredUIPromptForWindowsCredentialsW,
    CredUnPackAuthenticationBufferW, CredWriteW,
};
use windows_sys::Win32::Security::{
    LOGON32_LOGON_INTERACTIVE, LOGON32_LOGON_NETWORK, LOGON32_PROVIDER_DEFAULT, LogonUserW,
};
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::core::HRESULT;
use zeroize::Zeroize;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::wide_from_str;
use crate::platform::os::{
    CredentialAccessibility, CredentialAuthenticationMechanism, CredentialAuthenticationPolicy,
    CredentialAuthenticationRequirement, CredentialAuthenticationResult,
};
use crate::runtime::BindingCallContext;

use super::super::core::{
    CredentialAuthenticationOptionsOwned, CredentialQueryOwned, CredentialRecordOwned,
    CredentialWriteOptionsOwned, OS_CREDENTIALS_AUTHENTICATE_OPERATION,
    OS_CREDENTIALS_CONTAINS_OPERATION, OS_CREDENTIALS_DELETE_OPERATION,
    OS_CREDENTIALS_READ_OPERATION, OS_CREDENTIALS_WRITE_OPERATION, already_exists,
    invalid_argument, invalid_data, no_replace_write_guard, not_found, not_supported,
    permission_denied, would_block,
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
/// Field name for one authenticate title conversion.
const AUTHENTICATE_TITLE_FIELD: &str = "options.title";
/// Field name for one authenticate message conversion.
const AUTHENTICATE_MESSAGE_FIELD: &str = "options.message";
/// WinBio biometric type bit for facial biometrics.
const WINBIO_TYPE_FACIAL_FEATURES: u32 = 0x0000_0002;
/// WinBio biometric type bit for fingerprint biometrics.
const WINBIO_TYPE_FINGERPRINT: u32 = 0x0000_0008;
/// WinBio biometric type bit for iris biometrics.
const WINBIO_TYPE_IRIS: u32 = 0x0000_0010;
/// WinBio factor mask for common biometric modalities.
const WINBIO_TYPE_COMMON_BIOMETRIC_MASK: u32 =
    WINBIO_TYPE_FACIAL_FEATURES | WINBIO_TYPE_FINGERPRINT | WINBIO_TYPE_IRIS;
/// WinBio session flags for default behavior.
const WINBIO_SESSION_FLAGS_DEFAULT: u32 = 0;
/// First logon type attempt for device-credential verification.
const DEVICE_CREDENTIAL_LOGON_TYPE_PRIMARY: u32 = LOGON32_LOGON_INTERACTIVE;
/// Fallback logon type attempt for device-credential verification.
const DEVICE_CREDENTIAL_LOGON_TYPE_FALLBACK: u32 = LOGON32_LOGON_NETWORK;
/// Prompt title for read-time credential authentication challenges.
const READ_AUTHENTICATION_PROMPT_TITLE: &str = "Credential Access";
/// Prompt subtitle prefix for read-time credential authentication challenges.
const READ_AUTHENTICATION_PROMPT_SUBTITLE_PREFIX: &str = "Read credential for service";
/// Prompt message prefix for read-time credential authentication challenges.
const READ_AUTHENTICATION_PROMPT_MESSAGE_PREFIX: &str = "Account";

/// Read one credential record from the Windows credential manager.
pub(crate) fn read_credentials(
    _context: &BindingCallContext,
    query: &CredentialQueryOwned,
) -> RuntimeResult<CredentialRecordOwned> {
    // enforce explicit host authentication before reading one credential payload
    if query.require_authentication {
        authenticate_read_access(&query.service, &query.account)?;
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

/// Run one read-time authentication challenge before returning credential bytes.
fn authenticate_read_access(service: &str, account: &str) -> RuntimeResult<()> {
    // build one deterministic read-access challenge prompt payload
    let options = CredentialAuthenticationOptionsOwned {
        title: READ_AUTHENTICATION_PROMPT_TITLE.to_string(),
        subtitle: format!("{READ_AUTHENTICATION_PROMPT_SUBTITLE_PREFIX}: {service}"),
        message: format!("{READ_AUTHENTICATION_PROMPT_MESSAGE_PREFIX}: {account}"),
        requirement: CredentialAuthenticationRequirement::DeviceCredential,
    };

    // require a successful credential challenge before returning secret bytes
    let result = authenticate_with_device_credential(&options)?;
    if result.authenticated {
        return Ok(());
    }

    Err(permission_denied(
        OS_CREDENTIALS_READ_OPERATION,
        "windows credential read authentication did not complete successfully",
    ))
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
    options: &CredentialAuthenticationOptionsOwned,
) -> RuntimeResult<CredentialAuthenticationResult> {
    // route by explicit requirement policy
    if options.requirement == CredentialAuthenticationRequirement::BiometricOrDeviceCredential {
        return authenticate_with_credential_ui(options);
    }

    // route biometric-only requests through Windows Biometric Framework
    if options.requirement == CredentialAuthenticationRequirement::Biometric {
        return authenticate_with_windows_biometric();
    }

    // route device-credential-only requests through a verification challenge
    authenticate_with_device_credential(options)
}

/// Run one host authentication challenge through Windows credential UI.
fn authenticate_with_credential_ui(
    options: &CredentialAuthenticationOptionsOwned,
) -> RuntimeResult<CredentialAuthenticationResult> {
    // compose one prompt caption and body from trimmed fields
    let title = options.title.trim();
    let subtitle = options.subtitle.trim();
    let message = options.message.trim();

    let caption_text = if title.is_empty() {
        "Authentication Required"
    } else {
        title
    };

    let mut prompt_lines = Vec::with_capacity(2);
    if !subtitle.is_empty() {
        prompt_lines.push(subtitle);
    }
    if !message.is_empty() {
        prompt_lines.push(message);
    }

    let prompt_text = if prompt_lines.is_empty() {
        caption_text.to_string()
    } else {
        prompt_lines.join("\n")
    };

    // convert prompt fields into utf16 for host api calls
    let caption = wide_from_str(AUTHENTICATE_TITLE_FIELD, caption_text)?;
    let prompt = wide_from_str(AUTHENTICATE_MESSAGE_FIELD, &prompt_text)?;

    // configure one windows credential prompt dialog descriptor
    let prompt_info = CREDUI_INFOW {
        cbSize: mem::size_of::<CREDUI_INFOW>() as u32,
        hwndParent: 0,
        pszMessageText: prompt.as_ptr(),
        pszCaptionText: caption.as_ptr(),
        hbmBanner: 0,
    };

    // execute one host authentication prompt and capture returned credentials buffer
    let mut auth_package = 0u32;
    let mut out_auth_buffer = ptr::null_mut();
    let mut out_auth_buffer_size = 0u32;
    let mut save = 0i32;
    let flags = CREDUIWIN_GENERIC | CREDUIWIN_SECURE_PROMPT;
    let status = unsafe {
        CredUIPromptForWindowsCredentialsW(
            &prompt_info,
            0,
            &mut auth_package,
            ptr::null(),
            0,
            &mut out_auth_buffer,
            &mut out_auth_buffer_size,
            &mut save,
            flags,
        )
    };

    // release returned auth buffer memory regardless of prompt outcome
    let _auth_buffer_guard = WindowsPromptBufferGuard { out_auth_buffer };

    // map successful challenge completion
    if status == 0 {
        return Ok(CredentialAuthenticationResult {
            authenticated: true,
            mechanism: CredentialAuthenticationMechanism::Unknown,
        });
    }

    // map user cancellation into explicit unauthenticated outcome
    if status == ERROR_CANCELLED {
        return Ok(CredentialAuthenticationResult {
            authenticated: false,
            mechanism: CredentialAuthenticationMechanism::Unknown,
        });
    }

    // map timeout outcomes as temporary host ui unavailability
    if status == ERROR_TIMEOUT {
        return Err(would_block(
            OS_CREDENTIALS_AUTHENTICATE_OPERATION,
            "windows credential prompt timed out",
        ));
    }

    // map remaining errors with existing windows credentials mapping
    Err(map_windows_credentials_error(
        OS_CREDENTIALS_AUTHENTICATE_OPERATION,
        status,
    ))
}

/// Run one device-credential-only host authentication challenge on Windows.
fn authenticate_with_device_credential(
    options: &CredentialAuthenticationOptionsOwned,
) -> RuntimeResult<CredentialAuthenticationResult> {
    // compose one prompt caption and body from trimmed fields
    let title = options.title.trim();
    let subtitle = options.subtitle.trim();
    let message = options.message.trim();

    let caption_text = if title.is_empty() {
        "Authentication Required"
    } else {
        title
    };

    let mut prompt_lines = Vec::with_capacity(2);
    if !subtitle.is_empty() {
        prompt_lines.push(subtitle);
    }
    if !message.is_empty() {
        prompt_lines.push(message);
    }

    let prompt_text = if prompt_lines.is_empty() {
        caption_text.to_string()
    } else {
        prompt_lines.join("\n")
    };

    // convert prompt fields into utf16 for host api calls
    let caption = wide_from_str(AUTHENTICATE_TITLE_FIELD, caption_text)?;
    let prompt = wide_from_str(AUTHENTICATE_MESSAGE_FIELD, &prompt_text)?;

    // configure one windows credential prompt dialog descriptor
    let prompt_info = CREDUI_INFOW {
        cbSize: mem::size_of::<CREDUI_INFOW>() as u32,
        hwndParent: 0,
        pszMessageText: prompt.as_ptr(),
        pszCaptionText: caption.as_ptr(),
        hbmBanner: 0,
    };

    // execute one host authentication prompt and capture returned credentials buffer
    let mut auth_package = 0u32;
    let mut out_auth_buffer = ptr::null_mut();
    let mut out_auth_buffer_size = 0u32;
    let mut save = 0i32;
    let flags = CREDUIWIN_SECURE_PROMPT;
    let status = unsafe {
        CredUIPromptForWindowsCredentialsW(
            &prompt_info,
            0,
            &mut auth_package,
            ptr::null(),
            0,
            &mut out_auth_buffer,
            &mut out_auth_buffer_size,
            &mut save,
            flags,
        )
    };

    // release returned auth buffer memory regardless of prompt outcome
    let auth_buffer_guard = WindowsPromptBufferGuard { out_auth_buffer };

    // map successful prompt completion into device-credential verification
    if status == 0 {
        let credentials = unpack_windows_authentication_buffer(
            auth_buffer_guard.out_auth_buffer,
            out_auth_buffer_size,
        )?;
        return verify_windows_device_credential(&credentials);
    }

    // map user cancellation into explicit unauthenticated outcome
    if status == ERROR_CANCELLED {
        return Ok(CredentialAuthenticationResult {
            authenticated: false,
            mechanism: CredentialAuthenticationMechanism::DeviceCredential,
        });
    }

    // map timeout outcomes as temporary host ui unavailability
    if status == ERROR_TIMEOUT {
        return Err(would_block(
            OS_CREDENTIALS_AUTHENTICATE_OPERATION,
            "windows credential prompt timed out",
        ));
    }

    // map remaining errors with existing windows credentials mapping
    Err(map_windows_credentials_error(
        OS_CREDENTIALS_AUTHENTICATE_OPERATION,
        status,
    ))
}

/// Unpack one Windows authentication buffer into one username-domain-password tuple.
fn unpack_windows_authentication_buffer(
    auth_buffer: *mut c_void,
    auth_buffer_size: u32,
) -> RuntimeResult<WindowsUnpackedCredentials> {
    // reject invalid prompt output buffers before unpacking
    if auth_buffer.is_null() || auth_buffer_size == 0 {
        return Err(invalid_data(
            OS_CREDENTIALS_AUTHENTICATE_OPERATION,
            "windows credential prompt returned one empty authentication buffer",
        ));
    }

    // first try protected-credentials unpack semantics
    let protected_result = try_unpack_windows_authentication_buffer(
        CRED_PACK_PROTECTED_CREDENTIALS,
        auth_buffer.cast_const(),
        auth_buffer_size,
    );
    if let Ok(credentials) = protected_result {
        return Ok(credentials);
    }

    // otherwise fall back to plain unpack semantics for compatibility lanes
    try_unpack_windows_authentication_buffer(0, auth_buffer.cast_const(), auth_buffer_size)
}

/// Try one unpack operation for one Windows authentication buffer flag mode.
fn try_unpack_windows_authentication_buffer(
    unpack_flags: u32,
    auth_buffer: *const c_void,
    auth_buffer_size: u32,
) -> RuntimeResult<WindowsUnpackedCredentials> {
    // query required output buffer lengths
    let mut username_length = 0u32;
    let mut domain_length = 0u32;
    let mut password_length = 0u32;
    let probe_status = unsafe {
        CredUnPackAuthenticationBufferW(
            unpack_flags,
            auth_buffer,
            auth_buffer_size,
            ptr::null_mut(),
            &mut username_length,
            ptr::null_mut(),
            &mut domain_length,
            ptr::null_mut(),
            &mut password_length,
        )
    };
    if probe_status != 0 {
        return Err(invalid_data(
            OS_CREDENTIALS_AUTHENTICATE_OPERATION,
            "windows credential unpack length probe unexpectedly succeeded",
        ));
    }
    let probe_error = unsafe { GetLastError() };
    if probe_error != ERROR_INSUFFICIENT_BUFFER {
        return Err(map_windows_credentials_error(
            OS_CREDENTIALS_AUTHENTICATE_OPERATION,
            probe_error,
        ));
    }

    // allocate one output buffer per unpacked credential field
    let mut username = vec![0u16; username_length.max(1) as usize];
    let mut domain = vec![0u16; domain_length.max(1) as usize];
    let mut password = vec![0u16; password_length.max(1) as usize];

    // unpack one username-domain-password tuple
    let unpack_status = unsafe {
        CredUnPackAuthenticationBufferW(
            unpack_flags,
            auth_buffer,
            auth_buffer_size,
            username.as_mut_ptr(),
            &mut username_length,
            domain.as_mut_ptr(),
            &mut domain_length,
            password.as_mut_ptr(),
            &mut password_length,
        )
    };
    if unpack_status == 0 {
        return Err(map_windows_credentials_error(
            OS_CREDENTIALS_AUTHENTICATE_OPERATION,
            unsafe { GetLastError() },
        ));
    }

    Ok(WindowsUnpackedCredentials {
        username,
        domain,
        password,
    })
}

/// Verify one unpacked username-domain-password tuple through Windows logon lanes.
fn verify_windows_device_credential(
    credentials: &WindowsUnpackedCredentials,
) -> RuntimeResult<CredentialAuthenticationResult> {
    // reject missing usernames because Windows logon requires one principal identifier
    if credentials.username.is_empty() || credentials.username[0] == 0 {
        return Err(invalid_data(
            OS_CREDENTIALS_AUTHENTICATE_OPERATION,
            "windows credential prompt returned one empty username",
        ));
    }

    // prepare nullable domain and password pointers for host logon calls
    let username_pointer = credentials.username.as_ptr();
    let domain_pointer = if credentials.domain.is_empty() || credentials.domain[0] == 0 {
        ptr::null()
    } else {
        credentials.domain.as_ptr()
    };
    let password_pointer = if credentials.password.is_empty() || credentials.password[0] == 0 {
        ptr::null()
    } else {
        credentials.password.as_ptr()
    };

    // first try interactive logon, then network logon when interactive type is unavailable
    let logon_types = [
        DEVICE_CREDENTIAL_LOGON_TYPE_PRIMARY,
        DEVICE_CREDENTIAL_LOGON_TYPE_FALLBACK,
    ];
    let mut last_error_code = ERROR_LOGON_TYPE_NOT_GRANTED;
    for logon_type in logon_types {
        let mut token_handle: HANDLE = 0;
        let logon_status = unsafe {
            LogonUserW(
                username_pointer,
                domain_pointer,
                password_pointer,
                logon_type,
                LOGON32_PROVIDER_DEFAULT,
                &mut token_handle,
            )
        };

        // close one successful logon token handle and return authenticated outcome
        if logon_status != 0 {
            let _token_guard = WindowsLogonTokenGuard { token_handle };
            return Ok(CredentialAuthenticationResult {
                authenticated: true,
                mechanism: CredentialAuthenticationMechanism::DeviceCredential,
            });
        }

        // track one failed logon error to decide fallback and result mapping
        let error_code = unsafe { GetLastError() };
        last_error_code = error_code;
        if error_code == ERROR_LOGON_TYPE_NOT_GRANTED {
            continue;
        }

        break;
    }

    // map expected authentication rejections into unauthenticated outcomes
    if is_windows_device_credential_rejection(last_error_code) {
        return Ok(CredentialAuthenticationResult {
            authenticated: false,
            mechanism: CredentialAuthenticationMechanism::DeviceCredential,
        });
    }

    // map permission-limited logon lanes
    if last_error_code == ERROR_ACCESS_DENIED || last_error_code == ERROR_NO_SUCH_LOGON_SESSION {
        return Err(permission_denied(
            OS_CREDENTIALS_AUTHENTICATE_OPERATION,
            format!("windows logon verification denied one operation with code {last_error_code}"),
        ));
    }

    // map temporary host failure lanes
    if last_error_code == ERROR_TIMEOUT {
        return Err(would_block(
            OS_CREDENTIALS_AUTHENTICATE_OPERATION,
            format!("windows logon verification timed out with code {last_error_code}"),
        ));
    }

    // map unsupported host lanes explicitly
    if last_error_code == ERROR_NOT_SUPPORTED || last_error_code == ERROR_LOGON_TYPE_NOT_GRANTED {
        return Err(not_supported(OS_CREDENTIALS_AUTHENTICATE_OPERATION));
    }

    // map malformed-argument lanes
    if last_error_code == ERROR_INVALID_PARAMETER || last_error_code == ERROR_BAD_LENGTH {
        return Err(invalid_argument(
            "options",
            format!(
                "windows logon verification rejected one authentication argument with code {last_error_code}"
            ),
        ));
    }

    // map unknown host failures into explicit ioInvalidData
    Err(invalid_data(
        OS_CREDENTIALS_AUTHENTICATE_OPERATION,
        format!("windows logon verification failed with code {last_error_code}"),
    ))
}

/// Return whether one Windows logon error means credentials were not authenticated.
fn is_windows_device_credential_rejection(error_code: u32) -> bool {
    error_code == ERROR_LOGON_FAILURE
        || error_code == ERROR_ACCOUNT_RESTRICTION
        || error_code == ERROR_INVALID_LOGON_HOURS
        || error_code == ERROR_INVALID_WORKSTATION
        || error_code == ERROR_PASSWORD_EXPIRED
        || error_code == ERROR_ACCOUNT_DISABLED
        || error_code == ERROR_ACCOUNT_LOCKED_OUT
}

/// Run one biometric-only host authentication challenge through Windows Biometric Framework.
fn authenticate_with_windows_biometric() -> RuntimeResult<CredentialAuthenticationResult> {
    // open one biometric session across common biometric factors
    let mut session_handle = 0u32;
    let open_status = unsafe {
        WinBioOpenSession(
            WINBIO_TYPE_COMMON_BIOMETRIC_MASK,
            WINBIO_POOL_SYSTEM,
            WINBIO_SESSION_FLAGS_DEFAULT,
            ptr::null(),
            0,
            ptr::null(),
            &mut session_handle,
        )
    };
    if open_status < 0 {
        return Err(map_windows_biometric_error(open_status));
    }

    // close the WinBio session handle on every return path
    let session_guard = WindowsBiometricSessionGuard { session_handle };

    // execute one biometric identify challenge
    let mut unit_id = 0u32;
    let mut identity = unsafe { mem::zeroed::<WINBIO_IDENTITY>() };
    let mut subfactor = 0u8;
    let mut reject_detail = 0u32;
    let identify_status = unsafe {
        WinBioIdentify(
            session_guard.session_handle,
            &mut unit_id,
            &mut identity,
            &mut subfactor,
            &mut reject_detail,
        )
    };

    // map successful biometric verification
    if identify_status >= 0 {
        return Ok(CredentialAuthenticationResult {
            authenticated: true,
            mechanism: CredentialAuthenticationMechanism::Biometric,
        });
    }

    // map expected non-authenticated outcomes
    if identify_status == WINBIO_E_NO_MATCH || identify_status == WINBIO_E_CANCELED {
        return Ok(CredentialAuthenticationResult {
            authenticated: false,
            mechanism: CredentialAuthenticationMechanism::Biometric,
        });
    }

    // map host biometric errors into runtime lanes
    Err(map_windows_biometric_error(identify_status))
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

    // map temporary host unavailability and prompt timeouts
    if error_code == ERROR_TIMEOUT {
        return would_block(
            operation,
            format!("windows credential manager timed out with code {error_code}"),
        );
    }

    // map backend unavailability to not-supported
    if error_code == ERROR_NOT_SUPPORTED {
        return not_supported(operation);
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

/// Map one WinBio error code into one runtime error.
fn map_windows_biometric_error(error_code: HRESULT) -> Box<RuntimeError> {
    // map unsupported biometric host lanes
    if error_code == WINBIO_E_DISABLED
        || error_code == WINBIO_E_SENSOR_UNAVAILABLE
        || error_code == WINBIO_E_UNSUPPORTED_FACTOR
        || error_code == WINBIO_E_UNSUPPORTED_POOL_TYPE
        || error_code == WINBIO_E_NOT_ACTIVE_CONSOLE
    {
        return not_supported(OS_CREDENTIALS_AUTHENTICATE_OPERATION);
    }

    // map temporary busy states
    if error_code == WINBIO_E_DEVICE_BUSY || error_code == WINBIO_E_SESSION_BUSY {
        return would_block(
            OS_CREDENTIALS_AUTHENTICATE_OPERATION,
            format!("windows biometric framework is busy with code {error_code}"),
        );
    }

    // map explicit access-denied outcomes
    if error_code == ERROR_ACCESS_DENIED as HRESULT {
        return permission_denied(
            OS_CREDENTIALS_AUTHENTICATE_OPERATION,
            "windows biometric framework denied one authentication operation",
        );
    }

    // map unknown biometric host failures
    invalid_data(
        OS_CREDENTIALS_AUTHENTICATE_OPERATION,
        format!("windows biometric framework failed with code {error_code}"),
    )
}

/// Guard one WinBio session handle and close it on drop.
struct WindowsBiometricSessionGuard {
    /// Open WinBio session handle.
    session_handle: u32,
}

/// Guard one credential prompt output buffer and free it on drop.
struct WindowsPromptBufferGuard {
    /// Prompt output buffer allocated by Windows.
    out_auth_buffer: *mut c_void,
}

impl Drop for WindowsPromptBufferGuard {
    fn drop(&mut self) {
        // release one credentials ui output buffer
        if self.out_auth_buffer.is_null() {
            return;
        }

        unsafe {
            CoTaskMemFree(self.out_auth_buffer.cast());
        }
    }
}

/// Guard one Windows logon token and close it on drop.
struct WindowsLogonTokenGuard {
    /// Logon token handle from successful credential verification.
    token_handle: HANDLE,
}

impl Drop for WindowsLogonTokenGuard {
    fn drop(&mut self) {
        // close one successful logon token handle
        if self.token_handle == 0 {
            return;
        }

        unsafe {
            let _close_status = CloseHandle(self.token_handle);
        }
    }
}

/// One unpacked Windows authentication tuple.
struct WindowsUnpackedCredentials {
    /// Username field in null-terminated utf16 form.
    username: Vec<u16>,
    /// Domain field in null-terminated utf16 form.
    domain: Vec<u16>,
    /// Password field in null-terminated utf16 form.
    password: Vec<u16>,
}

impl Drop for WindowsUnpackedCredentials {
    fn drop(&mut self) {
        // clear credential bytes before releasing allocation
        self.username.zeroize();
        self.domain.zeroize();
        self.password.zeroize();
    }
}

impl Drop for WindowsBiometricSessionGuard {
    fn drop(&mut self) {
        // close one open WinBio session
        unsafe {
            let _close_status = WinBioCloseSession(self.session_handle);
        }
    }
}
