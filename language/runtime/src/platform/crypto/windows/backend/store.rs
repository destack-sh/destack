use std::collections::HashSet;
use std::fs;

use openssl::x509::X509;
use windows_sys::Win32::Security::Cryptography::{
    CERT_SYSTEM_STORE_CURRENT_USER, CERT_SYSTEM_STORE_LOCAL_MACHINE,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::CryptoStoreKind;
use crate::runtime::BindingCallContext;

use super::certificate::windows_collect_certificates_from_location;
use super::constants::STORE_OPEN_OPERATION;
use super::core::{
    not_supported, permission_denied, windows_dpapi_protect, windows_dpapi_unprotect,
    windows_keystore_path, windows_try_open_store,
};

pub(crate) fn host_store_persistence_backend_is_available(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    windows_keystore_path(context, kind).is_some()
}

/// Return whether one host store lane is currently available.
pub(crate) fn host_store_lane_is_available(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    // probe one representative store for each lane
    match kind {
        CryptoStoreKind::System => {
            windows_try_open_store(CERT_SYSTEM_STORE_CURRENT_USER, "ROOT")
                || windows_try_open_store(CERT_SYSTEM_STORE_LOCAL_MACHINE, "ROOT")
        }
        CryptoStoreKind::User => {
            windows_try_open_store(CERT_SYSTEM_STORE_CURRENT_USER, "ROOT")
                || windows_keystore_path(context, CryptoStoreKind::User).is_some()
        }
        CryptoStoreKind::Machine => {
            windows_try_open_store(CERT_SYSTEM_STORE_LOCAL_MACHINE, "ROOT")
                || windows_keystore_path(context, CryptoStoreKind::Machine).is_some()
        }
        CryptoStoreKind::Provider | CryptoStoreKind::Ephemeral => false,
    }
}

/// Open one host store lane and return certificate snapshots.
pub(crate) fn open_host_store_certificates(
    _context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> RuntimeResult<Vec<X509>> {
    // merge certificates from all store collections in this lane
    let mut certificates = Vec::new();
    let mut seen_der_certificates = HashSet::new();
    match kind {
        CryptoStoreKind::System => {
            windows_collect_certificates_from_location(
                CERT_SYSTEM_STORE_CURRENT_USER,
                "ROOT",
                &mut certificates,
                &mut seen_der_certificates,
            )?;
            windows_collect_certificates_from_location(
                CERT_SYSTEM_STORE_CURRENT_USER,
                "CA",
                &mut certificates,
                &mut seen_der_certificates,
            )?;
            windows_collect_certificates_from_location(
                CERT_SYSTEM_STORE_CURRENT_USER,
                "TrustedPeople",
                &mut certificates,
                &mut seen_der_certificates,
            )?;
            windows_collect_certificates_from_location(
                CERT_SYSTEM_STORE_LOCAL_MACHINE,
                "ROOT",
                &mut certificates,
                &mut seen_der_certificates,
            )?;
            windows_collect_certificates_from_location(
                CERT_SYSTEM_STORE_LOCAL_MACHINE,
                "CA",
                &mut certificates,
                &mut seen_der_certificates,
            )?;
            windows_collect_certificates_from_location(
                CERT_SYSTEM_STORE_LOCAL_MACHINE,
                "TrustedPeople",
                &mut certificates,
                &mut seen_der_certificates,
            )?;
        }
        CryptoStoreKind::User => {
            windows_collect_certificates_from_location(
                CERT_SYSTEM_STORE_CURRENT_USER,
                "ROOT",
                &mut certificates,
                &mut seen_der_certificates,
            )?;
            windows_collect_certificates_from_location(
                CERT_SYSTEM_STORE_CURRENT_USER,
                "CA",
                &mut certificates,
                &mut seen_der_certificates,
            )?;
            windows_collect_certificates_from_location(
                CERT_SYSTEM_STORE_CURRENT_USER,
                "TrustedPeople",
                &mut certificates,
                &mut seen_der_certificates,
            )?;
        }
        CryptoStoreKind::Machine => {
            windows_collect_certificates_from_location(
                CERT_SYSTEM_STORE_LOCAL_MACHINE,
                "ROOT",
                &mut certificates,
                &mut seen_der_certificates,
            )?;
            windows_collect_certificates_from_location(
                CERT_SYSTEM_STORE_LOCAL_MACHINE,
                "CA",
                &mut certificates,
                &mut seen_der_certificates,
            )?;
            windows_collect_certificates_from_location(
                CERT_SYSTEM_STORE_LOCAL_MACHINE,
                "TrustedPeople",
                &mut certificates,
                &mut seen_der_certificates,
            )?;
        }
        CryptoStoreKind::Provider => return Err(not_supported(STORE_OPEN_OPERATION)),
        CryptoStoreKind::Ephemeral => return Ok(Vec::new()),
    }

    Ok(certificates)
}

/// Load one host-key snapshot blob from Windows crypt32 storage.
pub(crate) fn load_host_key_snapshot_bytes(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    operation: &'static str,
) -> RuntimeResult<Option<Vec<u8>>> {
    // resolve one store path for the selected lane
    let Some(path) = windows_keystore_path(context, kind) else {
        return Ok(None);
    };

    // read protected snapshot bytes when the file exists
    let protected_bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            if error.kind() == std::io::ErrorKind::NotFound {
                return Ok(None);
            }
            return Err(permission_denied(
                operation,
                format!("failed to read windows host keystore snapshot {path:?}: {error}"),
            ));
        }
    };

    // unprotect bytes through crypt32 lane policy
    let protected_for_machine = kind == CryptoStoreKind::Machine;
    let bytes = windows_dpapi_unprotect(&protected_bytes, protected_for_machine, operation)?;

    Ok(Some(bytes))
}

/// Store one host-key snapshot blob into Windows crypt32 storage.
pub(crate) fn store_host_key_snapshot_bytes(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    snapshot_bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve one store path for the selected lane
    let Some(path) = windows_keystore_path(context, kind) else {
        return Err(not_supported(operation));
    };

    // create parent directories before writing the snapshot
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            permission_denied(
                operation,
                format!("failed to create windows host keystore path {parent:?}: {error}"),
            )
        })?;
    }

    // protect bytes through crypt32 lane policy and write to disk
    let protect_for_machine = kind == CryptoStoreKind::Machine;
    let protected_bytes = windows_dpapi_protect(snapshot_bytes, protect_for_machine, operation)?;
    fs::write(&path, protected_bytes).map_err(|error| {
        permission_denied(
            operation,
            format!("failed to write windows host keystore snapshot {path:?}: {error}"),
        )
    })?;

    Ok(())
}
