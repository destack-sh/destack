use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use openssl::x509::X509;

/// Collect certificates from configured system trust-bundle locations.
pub(crate) fn collect_system_certificates(
    system_certificate_files: &[PathBuf],
    system_certificate_directories: &[PathBuf],
) -> Vec<X509> {
    let mut certificates = Vec::new();
    let mut seen_der_certificates = HashSet::new();

    // collect files first so explicit bundle overrides win deterministic order
    for path in system_certificate_files {
        collect_certificates_from_file(path, &mut certificates, &mut seen_der_certificates);
    }

    // collect directory entries after file candidates
    for path in system_certificate_directories {
        collect_certificates_from_directory(path, &mut certificates, &mut seen_der_certificates);
    }

    certificates
}

/// Return whether one system certificate source path exists.
pub(crate) fn has_system_certificate_source(
    system_certificate_files: &[PathBuf],
    system_certificate_directories: &[PathBuf],
) -> bool {
    // detect one existing bundle file
    for path in system_certificate_files {
        if path.is_file() {
            return true;
        }
    }

    // detect one existing certificate directory
    for path in system_certificate_directories {
        if path.is_dir() {
            return true;
        }
    }

    false
}

/// Collect certificates from one certificate bundle file.
fn collect_certificates_from_file(
    path: &Path,
    certificates: &mut Vec<X509>,
    seen_der_certificates: &mut HashSet<Vec<u8>>,
) {
    let Ok(bytes) = fs::read(path) else {
        return;
    };

    // parse PEM chains when present
    if let Ok(parsed_certificates) = X509::stack_from_pem(&bytes) {
        for certificate in parsed_certificates {
            if let Ok(der_bytes) = certificate.to_der() {
                push_der_certificate(&der_bytes, certificates, seen_der_certificates);
            }
        }

        return;
    }

    // otherwise try one DER certificate payload
    push_der_certificate(&bytes, certificates, seen_der_certificates);
}

/// Collect certificates from one certificate directory.
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
