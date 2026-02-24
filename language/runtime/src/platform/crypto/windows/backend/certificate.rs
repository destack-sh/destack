use std::collections::HashSet;

use openssl::x509::X509;
use windows_sys::Win32::Security::Cryptography::{
    CERT_CONTEXT, CERT_STORE_OPEN_EXISTING_FLAG, CERT_STORE_PROV_SYSTEM_W,
    CERT_STORE_READONLY_FLAG, CertCloseStore, CertEnumCertificatesInStore, CertOpenStore,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::CryptoStoreKind;
use crate::runtime::BindingCallContext;

use super::core::{not_supported, permission_denied, windows_store_name_utf16};

pub(crate) fn host_store_supports_certificate_write(
    _context: &BindingCallContext,
    _kind: CryptoStoreKind,
) -> bool {
    false
}

/// Import one certificate into one host store lane.
pub(crate) fn host_store_import_certificate(
    _context: &BindingCallContext,
    _kind: CryptoStoreKind,
    _certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    Err(not_supported(operation))
}

/// Delete one certificate from one host store lane.
pub(crate) fn host_store_delete_certificate(
    _context: &BindingCallContext,
    _kind: CryptoStoreKind,
    _certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    Err(not_supported(operation))
}

pub(super) fn windows_collect_certificates_from_location(
    location: u32,
    store_name: &str,
    certificates: &mut Vec<X509>,
    seen_der_certificates: &mut HashSet<Vec<u8>>,
) -> RuntimeResult<()> {
    use std::ffi::c_void;

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
        return Err(permission_denied(
            STORE_OPEN_OPERATION,
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
            push_der_certificate(der_bytes, certificates, seen_der_certificates);
        }

        previous_context = certificate_context;
    }

    // close one opened windows store handle
    unsafe {
        CertCloseStore(store, 0);
    }

    Ok(())
}

/// Push one certificate when DER bytes are unique and parseable.
fn push_der_certificate(
    der_bytes: &[u8],
    certificates: &mut Vec<X509>,
    seen_der_certificates: &mut HashSet<Vec<u8>>,
) {
    // skip duplicates across multi-store merges
    if !seen_der_certificates.insert(der_bytes.to_vec()) {
        return;
    }

    // parse and append one x509 certificate
    if let Ok(certificate) = X509::from_der(der_bytes) {
        certificates.push(certificate);
    }
}
