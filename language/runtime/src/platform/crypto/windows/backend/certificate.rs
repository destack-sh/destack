use std::collections::HashSet;
use std::ffi::c_void;
use std::ptr;

use openssl::x509::X509;
use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, GetLastError};
use windows_sys::Win32::Security::Cryptography::{
    CERT_CONTEXT, CERT_STORE_ADD_REPLACE_EXISTING, CERT_STORE_OPEN_EXISTING_FLAG,
    CERT_STORE_PROV_SYSTEM_W, CERT_STORE_READONLY_FLAG, CertAddEncodedCertificateToStore,
    CertCloseStore, CertDeleteCertificateFromStore, CertEnumCertificatesInStore,
    CertFreeCertificateContext, CertOpenStore, PKCS_7_ASN_ENCODING, X509_ASN_ENCODING,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::crypto::CryptoStoreKind;
use crate::platform::crypto::core::{CRYPTO_STORE_OPEN_OPERATION, push_der_certificate_if_unique};
use crate::runtime::BindingCallContext;

use super::constants::{
    WINDOWS_CERT_STORE_CURRENT_USER, WINDOWS_CERT_STORE_DELETE_ORDER, WINDOWS_CERT_STORE_IMPORT,
    WINDOWS_CERT_STORE_LOCAL_MACHINE,
};
use super::core::{invalid_data, permission_denied, windows_store_name_utf16};

/// Return one writable store location for one host store lane.
fn writable_store_location(kind: CryptoStoreKind) -> Option<u32> {
    match kind {
        CryptoStoreKind::User => Some(WINDOWS_CERT_STORE_CURRENT_USER),
        CryptoStoreKind::Machine => Some(WINDOWS_CERT_STORE_LOCAL_MACHINE),
        _ => None,
    }
}

/// Open one writable system certificate store handle.
fn open_writable_store(
    location: u32,
    store_name: &str,
    operation: &'static str,
) -> RuntimeResult<*mut c_void> {
    // open one existing writable store by location and name
    let store_name_utf16 = windows_store_name_utf16(store_name);
    let flags = location | CERT_STORE_OPEN_EXISTING_FLAG;
    let store = unsafe {
        CertOpenStore(
            CERT_STORE_PROV_SYSTEM_W,
            0,
            0,
            flags,
            store_name_utf16.as_ptr() as *const c_void,
        )
    };
    if store.is_null() {
        let code = unsafe { GetLastError() };
        return Err(permission_denied(
            operation,
            format!(
                "failed to open writable windows certificate store {store_name} with error code {code}"
            ),
        ));
    }

    Ok(store)
}

/// Open one writable system certificate store handle when present.
fn open_writable_store_if_present(
    location: u32,
    store_name: &str,
    operation: &'static str,
) -> RuntimeResult<Option<*mut c_void>> {
    // open one existing writable store by location and name
    let store_name_utf16 = windows_store_name_utf16(store_name);
    let flags = location | CERT_STORE_OPEN_EXISTING_FLAG;
    let store = unsafe {
        CertOpenStore(
            CERT_STORE_PROV_SYSTEM_W,
            0,
            0,
            flags,
            store_name_utf16.as_ptr() as *const c_void,
        )
    };
    if store.is_null() {
        let code = unsafe { GetLastError() };
        if code == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }

        return Err(permission_denied(
            operation,
            format!(
                "failed to open writable windows certificate store {store_name} with error code {code}"
            ),
        ));
    }

    Ok(Some(store))
}

/// Return whether one writable system store lane can be opened.
fn can_open_writable_store(location: u32, store_name: &str) -> bool {
    let store_name_utf16 = windows_store_name_utf16(store_name);
    let flags = location | CERT_STORE_OPEN_EXISTING_FLAG;
    let store = unsafe {
        CertOpenStore(
            CERT_STORE_PROV_SYSTEM_W,
            0,
            0,
            flags,
            store_name_utf16.as_ptr() as *const c_void,
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

/// Delete all matching certificates from one named store.
fn delete_certificate_from_named_store(
    location: u32,
    store_name: &str,
    certificate_der: &[u8],
    operation: &'static str,
) -> RuntimeResult<usize> {
    // open one writable store for deletion scan
    let Some(store) = open_writable_store_if_present(location, store_name, operation)? else {
        return Ok(0);
    };
    let mut deleted_count = 0usize;

    // enumerate certificates and delete every exact DER match
    let mut previous_context: *const CERT_CONTEXT = ptr::null();
    loop {
        let certificate_context = unsafe { CertEnumCertificatesInStore(store, previous_context) };
        if certificate_context.is_null() {
            break;
        }

        let context = unsafe { &*certificate_context };
        if !context.pbCertEncoded.is_null()
            && context.cbCertEncoded as usize == certificate_der.len()
        {
            let der_bytes = unsafe {
                std::slice::from_raw_parts(context.pbCertEncoded, context.cbCertEncoded as usize)
            };
            if der_bytes == certificate_der {
                let delete_status = unsafe { CertDeleteCertificateFromStore(certificate_context) };
                if delete_status == 0 {
                    let code = unsafe { GetLastError() };
                    unsafe {
                        CertCloseStore(store, 0);
                    }
                    return Err(permission_denied(
                        operation,
                        format!(
                            "failed to delete windows certificate from store {store_name} with error code {code}"
                        ),
                    ));
                }

                deleted_count += 1;
                previous_context = ptr::null();
                continue;
            }
        }

        previous_context = certificate_context;
    }

    // close one opened store handle after enumeration
    unsafe {
        CertCloseStore(store, 0);
    }

    Ok(deleted_count)
}

pub(crate) fn host_store_supports_certificate_write(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    writable_store_location(kind)
        .map(|location| can_open_writable_store(location, WINDOWS_CERT_STORE_IMPORT))
        .unwrap_or(false)
}

/// Import one certificate into one host store lane.
pub(crate) fn host_store_import_certificate(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve one writable store lane for this request
    let Some(location) = writable_store_location(kind) else {
        return Err(core_platform::not_supported(operation));
    };

    // encode certificate as DER bytes for crypt32 import
    let certificate_der = certificate
        .to_der()
        .map_err(|error| invalid_data(operation, format!("{error}")))?;

    // open one writable store and add the certificate payload
    let store = open_writable_store(location, WINDOWS_CERT_STORE_IMPORT, operation)?;
    let mut imported_context: *mut CERT_CONTEXT = ptr::null_mut();
    let add_status = unsafe {
        CertAddEncodedCertificateToStore(
            store,
            X509_ASN_ENCODING | PKCS_7_ASN_ENCODING,
            certificate_der.as_ptr(),
            certificate_der.len() as u32,
            CERT_STORE_ADD_REPLACE_EXISTING,
            &mut imported_context,
        )
    };

    // release any returned context and close the opened store
    if !imported_context.is_null() {
        unsafe {
            CertFreeCertificateContext(imported_context);
        }
    }
    unsafe {
        CertCloseStore(store, 0);
    }
    if add_status == 0 {
        let code = unsafe { GetLastError() };
        return Err(permission_denied(
            operation,
            format!(
                "failed to import one certificate into windows store {WINDOWS_CERT_STORE_IMPORT} with error code {code}"
            ),
        ));
    }

    Ok(())
}

/// Delete one certificate from one host store lane.
pub(crate) fn host_store_delete_certificate(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve one writable store lane for this request
    let Some(location) = writable_store_location(kind) else {
        return Err(core_platform::not_supported(operation));
    };

    // encode certificate as DER bytes for exact-match deletion
    let certificate_der = certificate
        .to_der()
        .map_err(|error| invalid_data(operation, format!("{error}")))?;

    // scan all relevant collections in this lane and delete every match
    let mut deleted_count = 0usize;
    for store_name in WINDOWS_CERT_STORE_DELETE_ORDER {
        deleted_count +=
            delete_certificate_from_named_store(location, store_name, &certificate_der, operation)?;
    }

    // return success when no lane store reports one match
    if deleted_count == 0 {
        return Ok(());
    }

    Ok(())
}

pub(super) fn windows_collect_certificates_from_location(
    location: u32,
    store_name: &str,
    certificates: &mut Vec<X509>,
    seen_der_certificates: &mut HashSet<Vec<u8>>,
) -> RuntimeResult<()> {
    // open one read-only system certificate store
    let store_name_utf16 = windows_store_name_utf16(store_name);
    let flags = location | CERT_STORE_OPEN_EXISTING_FLAG | CERT_STORE_READONLY_FLAG;
    let store = unsafe {
        CertOpenStore(
            CERT_STORE_PROV_SYSTEM_W,
            0,
            0,
            flags,
            store_name_utf16.as_ptr() as *const c_void,
        )
    };
    if store.is_null() {
        let code = unsafe { GetLastError() };
        if code == ERROR_FILE_NOT_FOUND {
            return Ok(());
        }

        return Err(permission_denied(
            CRYPTO_STORE_OPEN_OPERATION,
            format!("failed to open windows certificate store {store_name} with error code {code}"),
        ));
    }

    // enumerate and parse all DER certificates from the store
    let mut previous_context: *const CERT_CONTEXT = ptr::null();
    loop {
        let certificate_context = unsafe { CertEnumCertificatesInStore(store, previous_context) };
        if certificate_context.is_null() {
            break;
        }

        let context = unsafe { &*certificate_context };
        if !context.pbCertEncoded.is_null() && context.cbCertEncoded > 0 {
            let der_bytes = unsafe {
                std::slice::from_raw_parts(context.pbCertEncoded, context.cbCertEncoded as usize)
            };
            push_der_certificate_if_unique(der_bytes, certificates, seen_der_certificates);
        }

        previous_context = certificate_context;
    }

    // close one opened windows store handle
    unsafe {
        CertCloseStore(store, 0);
    }

    Ok(())
}
