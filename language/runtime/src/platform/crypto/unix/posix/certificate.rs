use std::collections::HashSet;
use std::fs;
use std::path::Path;

use openssl::x509::X509;

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::CryptoStoreKind;
use crate::runtime::BindingCallContext;

use super::core::{
    configured_system_certificate_directories, configured_system_certificate_files, not_supported,
};

/// Return whether one host store lane supports certificate write operations.
pub(crate) fn host_store_supports_certificate_write(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    let _ = (context, kind);

    false
}

/// Import one certificate into one host store lane.
pub(crate) fn host_store_import_certificate(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    let _ = (context, kind, certificate);

    Err(not_supported(operation))
}

/// Delete one certificate from one host store lane.
pub(crate) fn host_store_delete_certificate(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    let _ = (context, kind, certificate);

    Err(not_supported(operation))
}

/// Collect certificates from configured Unix system trust-bundle locations.
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

/// Return whether one Unix host certificate source path exists.
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

/// Collect certificates from one Unix certificate bundle file.
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

/// Collect certificates from one Unix certificate directory.
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
