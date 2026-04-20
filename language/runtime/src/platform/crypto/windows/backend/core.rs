use std::ffi::c_void;
use std::path::PathBuf;
use std::{env, ptr};

use windows_sys::Win32::Foundation::{GetLastError, LocalFree};
use windows_sys::Win32::Security::Cryptography::{
    CERT_STORE_OPEN_EXISTING_FLAG, CERT_STORE_PROV_SYSTEM_W, CERT_STORE_READONLY_FLAG,
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_LOCAL_MACHINE, CertCloseStore, CertOpenStore,
    CryptProtectData, CryptUnprotectData,
};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::crypto::CryptoStoreKind;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::runtime::BindingCallContext;

use super::constants::{
    WINDOWS_MACHINE_KEYSTORE_RELATIVE_PATH, WINDOWS_USER_KEYSTORE_RELATIVE_PATH,
};

pub(super) fn permission_denied(
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

/// Return one ioInvalidData runtime error.
pub(super) fn invalid_data(
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

pub(super) fn windows_keystore_path(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> Option<PathBuf> {
    // use runtime-configured path overrides first
    if let Some(path) = windows_configured_keystore_path(binding, kind) {
        return Some(path);
    }

    // resolve root path by lane scope
    let root = match kind {
        CryptoStoreKind::User => env::var_os("LOCALAPPDATA")
            .or_else(|| env::var_os("APPDATA"))
            .map(PathBuf::from),
        CryptoStoreKind::Machine => env::var_os("PROGRAMDATA").map(PathBuf::from),
        _ => None,
    }?;

    // append lane-specific relative path
    let relative = match kind {
        CryptoStoreKind::User => WINDOWS_USER_KEYSTORE_RELATIVE_PATH,
        CryptoStoreKind::Machine => WINDOWS_MACHINE_KEYSTORE_RELATIVE_PATH,
        _ => return None,
    };

    Some(root.join(relative))
}

/// Return one runtime-configured windows host-keystore path override for one lane.
pub(super) fn windows_configured_keystore_path(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> Option<PathBuf> {
    match kind {
        CryptoStoreKind::User => binding
            .worker()
            .options
            .crypto
            .host_store_paths
            .user
            .clone(),
        CryptoStoreKind::Machine => binding
            .worker()
            .options
            .crypto
            .host_store_paths
            .machine
            .clone(),
        _ => None,
    }
}

/// Protect one payload with crypt32.
pub(super) fn windows_dpapi_protect(
    bytes: &[u8],
    protect_for_machine: bool,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // prepare input and output blobs for crypt32
    let input_blob = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut output_blob = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: ptr::null_mut(),
    };
    let mut flags = 0u32;
    if protect_for_machine {
        flags |= CRYPTPROTECT_LOCAL_MACHINE;
    }

    // run crypt32 protection
    let status = unsafe {
        CryptProtectData(
            &input_blob,
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            flags,
            &mut output_blob,
        )
    };
    if status == 0 {
        let code = unsafe { GetLastError() };
        return Err(permission_denied(
            operation,
            format!("CryptProtectData failed with error code {code}"),
        ));
    }

    // copy protected output and free crypt32-owned allocation
    let protected_bytes = unsafe {
        std::slice::from_raw_parts(output_blob.pbData, output_blob.cbData as usize).to_vec()
    };
    unsafe {
        let _ = LocalFree(output_blob.pbData as _);
    }

    Ok(protected_bytes)
}

/// Unprotect one payload with crypt32.
pub(super) fn windows_dpapi_unprotect(
    protected_bytes: &[u8],
    protected_for_machine: bool,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // prepare input and output blobs for crypt32
    let input_blob = CRYPT_INTEGER_BLOB {
        cbData: protected_bytes.len() as u32,
        pbData: protected_bytes.as_ptr() as *mut u8,
    };
    let mut output_blob = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: ptr::null_mut(),
    };
    let mut description = ptr::null_mut();
    let mut flags = 0u32;
    if protected_for_machine {
        flags |= CRYPTPROTECT_LOCAL_MACHINE;
    }

    // run crypt32 unprotect
    let status = unsafe {
        CryptUnprotectData(
            &input_blob,
            &mut description,
            ptr::null(),
            ptr::null(),
            ptr::null(),
            flags,
            &mut output_blob,
        )
    };
    if status == 0 {
        let code = unsafe { GetLastError() };
        if !description.is_null() {
            unsafe {
                let _ = LocalFree(description as _);
            }
        }
        return Err(permission_denied(
            operation,
            format!("CryptUnprotectData failed with error code {code}"),
        ));
    }

    // copy plaintext output and free crypt32-owned allocations
    let bytes =
        unsafe { std::slice::from_raw_parts(output_blob.pbData, output_blob.cbData as usize) }
            .to_vec();
    unsafe {
        let _ = LocalFree(output_blob.pbData as _);
        if !description.is_null() {
            let _ = LocalFree(description as _);
        }
    }

    Ok(bytes)
}

/// Return whether one Windows store can be opened.
pub(super) fn windows_try_open_store(location: u32, store_name: &str) -> bool {
    // open and close one read-only system store
    let store_name = windows_store_name_utf16(store_name);
    let flags = location | CERT_STORE_OPEN_EXISTING_FLAG | CERT_STORE_READONLY_FLAG;
    let store = unsafe {
        CertOpenStore(
            CERT_STORE_PROV_SYSTEM_W,
            0,
            0,
            flags,
            store_name.as_ptr() as *const c_void,
        )
    };
    if store.is_null() {
        return false;
    }
    unsafe {
        CertCloseStore(store, 0);
    }

    true
}

/// Convert one store name into one UTF-16 null-terminated string.
pub(super) fn windows_store_name_utf16(store_name: &str) -> Vec<u16> {
    let mut store_name_utf16 = store_name.encode_utf16().collect::<Vec<u16>>();
    store_name_utf16.push(0);
    store_name_utf16
}
