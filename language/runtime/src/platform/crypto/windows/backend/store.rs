use std::collections::HashSet;
use std::fs;

use openssl::x509::X509;

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::CryptoStoreKind;
use crate::platform::crypto::core::CRYPTO_STORE_OPEN_OPERATION;
use crate::runtime::BindingCallContext;

use super::certificate::windows_collect_certificates_from_location;
use super::constants::{
    WINDOWS_CERT_STORE_CURRENT_USER, WINDOWS_CERT_STORE_LOCAL_MACHINE, WINDOWS_CERT_STORE_PROBE,
    WINDOWS_CERT_STORE_SCAN_ORDER,
};
use super::core::{
    not_supported, permission_denied, windows_dpapi_protect, windows_dpapi_unprotect,
    windows_keystore_path, windows_try_open_store,
};

/// Windows host store locations for the system lane.
const SYSTEM_STORE_LOCATIONS: [u32; 2] = [
    WINDOWS_CERT_STORE_CURRENT_USER,
    WINDOWS_CERT_STORE_LOCAL_MACHINE,
];

/// Windows host store locations for the user lane.
const USER_STORE_LOCATIONS: [u32; 1] = [WINDOWS_CERT_STORE_CURRENT_USER];

/// Windows host store locations for the machine lane.
const MACHINE_STORE_LOCATIONS: [u32; 1] = [WINDOWS_CERT_STORE_LOCAL_MACHINE];

/// Return certificate store locations for one host store lane.
fn store_locations_for_kind(kind: CryptoStoreKind) -> Option<&'static [u32]> {
    match kind {
        CryptoStoreKind::System => Some(&SYSTEM_STORE_LOCATIONS),
        CryptoStoreKind::User => Some(&USER_STORE_LOCATIONS),
        CryptoStoreKind::Machine => Some(&MACHINE_STORE_LOCATIONS),
        CryptoStoreKind::Provider | CryptoStoreKind::Ephemeral => None,
    }
}

/// Return whether any native system store in one lane can be opened.
fn has_openable_system_store(kind: CryptoStoreKind) -> bool {
    // probe one representative collection for each lane location
    let Some(locations) = store_locations_for_kind(kind) else {
        return false;
    };
    for location in locations {
        if windows_try_open_store(*location, WINDOWS_CERT_STORE_PROBE) {
            return true;
        }
    }

    false
}

/// Collect certificates from all configured windows collections in one lane.
fn collect_lane_certificates(
    locations: &[u32],
    certificates: &mut Vec<X509>,
    seen_der_certificates: &mut HashSet<Vec<u8>>,
) -> RuntimeResult<()> {
    // scan all lane locations in deterministic collection order
    for location in locations {
        for store_name in WINDOWS_CERT_STORE_SCAN_ORDER {
            windows_collect_certificates_from_location(
                *location,
                store_name,
                certificates,
                seen_der_certificates,
            )?;
        }
    }

    Ok(())
}

/// Return whether one host-lane store supports persistent key writes.
pub(crate) fn host_store_supports_key_persistence(kind: CryptoStoreKind) -> bool {
    matches!(kind, CryptoStoreKind::User | CryptoStoreKind::Machine)
}

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
    // resolve lane availability through native store probes and snapshot lanes
    match kind {
        CryptoStoreKind::System => has_openable_system_store(CryptoStoreKind::System),
        CryptoStoreKind::User => {
            has_openable_system_store(CryptoStoreKind::User)
                || windows_keystore_path(context, CryptoStoreKind::User).is_some()
        }
        CryptoStoreKind::Machine => {
            has_openable_system_store(CryptoStoreKind::Machine)
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

    // collect lane certificates from all configured windows store locations
    match kind {
        CryptoStoreKind::System => collect_lane_certificates(
            &SYSTEM_STORE_LOCATIONS,
            &mut certificates,
            &mut seen_der_certificates,
        )?,
        CryptoStoreKind::User => collect_lane_certificates(
            &USER_STORE_LOCATIONS,
            &mut certificates,
            &mut seen_der_certificates,
        )?,
        CryptoStoreKind::Machine => collect_lane_certificates(
            &MACHINE_STORE_LOCATIONS,
            &mut certificates,
            &mut seen_der_certificates,
        )?,
        CryptoStoreKind::Provider => return Err(not_supported(CRYPTO_STORE_OPEN_OPERATION)),
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
