use std::collections::HashSet;
use std::os::raw::c_void;
use std::path::Path;
use std::{fs, ptr};

use core_foundation_sys::base::{CFRelease, CFTypeRef, kCFAllocatorDefault};
use core_foundation_sys::data::CFDataCreate;
use core_foundation_sys::dictionary::{
    CFDictionaryCreate, kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks,
};
use openssl::x509::X509;
use security_framework_sys::base::{errSecDuplicateItem, errSecItemNotFound, errSecSuccess};
use security_framework_sys::item::{
    kSecClass, kSecClassCertificate, kSecUseAuthenticationUI, kSecUseAuthenticationUISkip,
    kSecValueData,
};
use security_framework_sys::keychain_item::{SecItemAdd, SecItemDelete};

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::CryptoStoreKind;
use crate::runtime::BindingCallContext;

use super::core::{
    configured_system_certificate_directories, configured_system_certificate_files, invalid_data,
    not_supported, permission_denied,
};

/// Return whether one host store lane supports certificate write operations.
pub(crate) fn host_store_supports_certificate_write(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    let _ = context;

    kind == CryptoStoreKind::User
}

/// Import one certificate into one host store lane.
pub(crate) fn host_store_import_certificate(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    let _ = context;

    // certificate writes are user-lane only
    if kind != CryptoStoreKind::User {
        return Err(not_supported(operation));
    }

    // serialize one der payload for keychain insert
    let certificate_bytes = certificate
        .to_der()
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let certificate_data = unsafe {
        CFDataCreate(
            kCFAllocatorDefault,
            certificate_bytes.as_ptr(),
            certificate_bytes.len() as isize,
        )
    };
    if certificate_data.is_null() {
        return Err(invalid_data(
            operation,
            "failed to create one certificate data payload",
        ));
    }

    // build one keychain insert query
    let query_keys = unsafe {
        [
            kSecClass as *const c_void,
            kSecValueData as *const c_void,
            kSecUseAuthenticationUI as *const c_void,
        ]
    };
    let query_values = unsafe {
        [
            kSecClassCertificate as *const c_void,
            certificate_data as *const c_void,
            kSecUseAuthenticationUISkip as *const c_void,
        ]
    };
    let query = unsafe {
        CFDictionaryCreate(
            kCFAllocatorDefault,
            query_keys.as_ptr(),
            query_values.as_ptr(),
            query_keys.len() as isize,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };
    if query.is_null() {
        unsafe {
            CFRelease(certificate_data as CFTypeRef);
        }

        return Err(invalid_data(
            operation,
            "failed to create one certificate import query",
        ));
    }

    // import one keychain certificate item
    let status = unsafe { SecItemAdd(query, ptr::null_mut()) };
    unsafe {
        CFRelease(query as CFTypeRef);
        CFRelease(certificate_data as CFTypeRef);
    }
    if status == errSecSuccess || status == errSecDuplicateItem {
        return Ok(());
    }

    Err(permission_denied(
        operation,
        format!("SecItemAdd failed with status code {status}"),
    ))
}

/// Delete one certificate from one host store lane.
pub(crate) fn host_store_delete_certificate(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    let _ = context;

    // certificate writes are user-lane only
    if kind != CryptoStoreKind::User {
        return Err(not_supported(operation));
    }

    // serialize one der payload for keychain lookup and delete
    let certificate_bytes = certificate
        .to_der()
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let certificate_data = unsafe {
        CFDataCreate(
            kCFAllocatorDefault,
            certificate_bytes.as_ptr(),
            certificate_bytes.len() as isize,
        )
    };
    if certificate_data.is_null() {
        return Err(invalid_data(
            operation,
            "failed to create one certificate data payload",
        ));
    }

    // build one keychain delete query
    let query_keys = unsafe {
        [
            kSecClass as *const c_void,
            kSecValueData as *const c_void,
            kSecUseAuthenticationUI as *const c_void,
        ]
    };
    let query_values = unsafe {
        [
            kSecClassCertificate as *const c_void,
            certificate_data as *const c_void,
            kSecUseAuthenticationUISkip as *const c_void,
        ]
    };
    let query = unsafe {
        CFDictionaryCreate(
            kCFAllocatorDefault,
            query_keys.as_ptr(),
            query_values.as_ptr(),
            query_keys.len() as isize,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };
    if query.is_null() {
        unsafe {
            CFRelease(certificate_data as CFTypeRef);
        }

        return Err(invalid_data(
            operation,
            "failed to create one certificate delete query",
        ));
    }

    // delete one keychain certificate item
    let status = unsafe { SecItemDelete(query) };
    unsafe {
        CFRelease(query as CFTypeRef);
        CFRelease(certificate_data as CFTypeRef);
    }
    if status == errSecSuccess || status == errSecItemNotFound {
        return Ok(());
    }

    Err(permission_denied(
        operation,
        format!("SecItemDelete failed with status code {status}"),
    ))
}

/// Collect certificates from configured iOS system trust-bundle locations.
pub(super) fn collect_system_certificates(context: &BindingCallContext) -> Vec<X509> {
    let mut certificates = Vec::new();
    let mut seen_der_certificates = HashSet::new();
    let system_certificate_files = configured_system_certificate_files(context);
    let system_certificate_directories = configured_system_certificate_directories(context);

    for path in system_certificate_files {
        collect_certificates_from_file(&path, &mut certificates, &mut seen_der_certificates);
    }

    for path in system_certificate_directories {
        collect_certificates_from_directory(&path, &mut certificates, &mut seen_der_certificates);
    }

    certificates
}

/// Return whether one iOS host certificate source path exists.
pub(super) fn has_system_certificate_source(context: &BindingCallContext) -> bool {
    for path in configured_system_certificate_files(context) {
        if path.is_file() {
            return true;
        }
    }

    for path in configured_system_certificate_directories(context) {
        if path.is_dir() {
            return true;
        }
    }

    false
}

/// Collect certificates from one iOS certificate bundle file.
fn collect_certificates_from_file(
    path: &Path,
    certificates: &mut Vec<X509>,
    seen_der_certificates: &mut HashSet<Vec<u8>>,
) {
    let Ok(bytes) = fs::read(path) else {
        return;
    };

    if let Ok(parsed_certificates) = X509::stack_from_pem(&bytes) {
        for certificate in parsed_certificates {
            if let Ok(der_bytes) = certificate.to_der() {
                push_der_certificate(&der_bytes, certificates, seen_der_certificates);
            }
        }
        return;
    }

    push_der_certificate(&bytes, certificates, seen_der_certificates);
}

/// Collect certificates from one iOS certificate directory.
fn collect_certificates_from_directory(
    path: &Path,
    certificates: &mut Vec<X509>,
    seen_der_certificates: &mut HashSet<Vec<u8>>,
) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let file_path = entry.path();
        if !file_path.is_file() {
            continue;
        }

        collect_certificates_from_file(&file_path, certificates, seen_der_certificates);
    }
}

/// Push one unique parsed certificate payload.
fn push_der_certificate(
    der_bytes: &[u8],
    certificates: &mut Vec<X509>,
    seen_der_certificates: &mut HashSet<Vec<u8>>,
) {
    if !seen_der_certificates.insert(der_bytes.to_vec()) {
        return;
    }

    if let Ok(certificate) = X509::from_der(der_bytes) {
        certificates.push(certificate);
    }
}
