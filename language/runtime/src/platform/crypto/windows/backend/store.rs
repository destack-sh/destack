use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use getrandom::fill as fill_secure_random;
use openssl::x509::X509;
use windows_sys::Win32::Storage::FileSystem::{
    MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::crypto::CryptoStoreKind;
use crate::platform::crypto::core::CRYPTO_STORE_OPEN_OPERATION;
use crate::runtime::BindingCallContext;

use super::certificate::windows_collect_certificates_from_location;
use super::constants::{
    WINDOWS_CERT_STORE_CURRENT_USER, WINDOWS_CERT_STORE_LOCAL_MACHINE, WINDOWS_CERT_STORE_PROBE,
    WINDOWS_CERT_STORE_SCAN_ORDER,
};
use super::core::{
    permission_denied, windows_dpapi_protect, windows_dpapi_unprotect, windows_keystore_path,
    windows_try_open_store,
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
/// Temporary-file prefix for atomic snapshot writes.
const WINDOWS_SNAPSHOT_TEMPORARY_PREFIX: &str = ".destack-crypto-snapshot-";

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
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    windows_keystore_path(binding, kind).is_some()
}

/// Return whether one host store lane is currently available.
pub(crate) fn host_store_lane_is_available(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    // resolve lane availability through native store probes and snapshot lanes
    match kind {
        CryptoStoreKind::System => has_openable_system_store(CryptoStoreKind::System),
        CryptoStoreKind::User => {
            has_openable_system_store(CryptoStoreKind::User)
                || windows_keystore_path(binding, CryptoStoreKind::User).is_some()
        }
        CryptoStoreKind::Machine => {
            has_openable_system_store(CryptoStoreKind::Machine)
                || windows_keystore_path(binding, CryptoStoreKind::Machine).is_some()
        }
        CryptoStoreKind::Provider | CryptoStoreKind::Ephemeral => false,
    }
}

/// Open one host store lane and return certificate snapshots.
pub(crate) fn open_host_store_certificates(
    _binding: &BindingCallContext,
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
        CryptoStoreKind::Provider => {
            return Err(core_platform::not_supported(CRYPTO_STORE_OPEN_OPERATION));
        }
        CryptoStoreKind::Ephemeral => return Ok(Vec::new()),
    }

    Ok(certificates)
}

/// Load one host-key snapshot blob from Windows crypt32 storage.
pub(crate) fn load_host_key_snapshot_bytes(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    operation: &'static str,
) -> RuntimeResult<Option<Vec<u8>>> {
    // resolve one store path for the selected lane
    let Some(path) = windows_keystore_path(binding, kind) else {
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
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    snapshot_bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve one store path for the selected lane
    let Some(path) = windows_keystore_path(binding, kind) else {
        return Err(core_platform::not_supported(operation));
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
    write_snapshot_bytes_atomically(&path, &protected_bytes, operation)?;

    Ok(())
}

/// Return one temporary path in the destination directory.
fn temporary_snapshot_path(path: &Path, operation: &'static str) -> RuntimeResult<PathBuf> {
    let temporary_identifier = temporary_snapshot_identifier(operation)?;
    let file_name = path
        .file_name()
        .map(|value| {
            let mut file_name = OsString::from(WINDOWS_SNAPSHOT_TEMPORARY_PREFIX);
            file_name.push(&temporary_identifier);
            file_name.push(".");
            file_name.push(value);
            file_name
        })
        .unwrap_or_else(|| {
            let mut file_name = OsString::from(WINDOWS_SNAPSHOT_TEMPORARY_PREFIX);
            file_name.push(&temporary_identifier);
            file_name.push(".keys");
            file_name
        });

    Ok(path.with_file_name(file_name))
}

/// Return one random temporary-file identifier.
fn temporary_snapshot_identifier(operation: &'static str) -> RuntimeResult<String> {
    // generate one random 128-bit identifier payload
    let mut random_bytes = [0u8; 16];
    fill_secure_random(&mut random_bytes).map_err(|error| {
        permission_denied(
            operation,
            format!("failed to generate windows host keystore temporary name: {error}"),
        )
    })?;

    // format one lowercase hex identifier
    let mut identifier = String::with_capacity(random_bytes.len() * 2);
    for byte in random_bytes {
        let high_nibble = byte >> 4;
        let low_nibble = byte & 0x0f;
        identifier.push(nibble_to_hex(high_nibble));
        identifier.push(nibble_to_hex(low_nibble));
    }

    Ok(identifier)
}

/// Convert one nibble value into one lowercase hex character.
fn nibble_to_hex(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        10..=15 => (b'a' + (nibble - 10)) as char,
        _ => '0',
    }
}

/// Replace one file with one fully-written temporary sibling.
fn replace_file(path: &Path, temporary_path: &Path) -> std::io::Result<()> {
    let mut path_wide = path.as_os_str().encode_wide().collect::<Vec<_>>();
    path_wide.push(0);
    let mut temporary_wide = temporary_path.as_os_str().encode_wide().collect::<Vec<_>>();
    temporary_wide.push(0);

    let status = unsafe {
        MoveFileExW(
            temporary_wide.as_ptr(),
            path_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if status != 0 {
        return Ok(());
    }

    Err(std::io::Error::last_os_error())
}

/// Write one snapshot file atomically through one same-directory temporary path.
fn write_snapshot_bytes_atomically(
    path: &Path,
    bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<()> {
    // create one temporary file in the destination directory
    let temporary_path = temporary_snapshot_path(path, operation)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary_path)
        .map_err(|error| {
            permission_denied(
                operation,
                format!(
                    "failed to create windows host keystore temporary file {temporary_path:?}: {error}"
                ),
            )
        })?;

    // write and flush the protected snapshot payload before rename
    let write_result = (|| -> RuntimeResult<()> {
        file.write_all(bytes).map_err(|error| {
            permission_denied(
                operation,
                format!(
                    "failed to write windows host keystore temporary file {temporary_path:?}: {error}"
                ),
            )
        })?;
        file.sync_all().map_err(|error| {
            permission_denied(
                operation,
                format!(
                    "failed to flush windows host keystore temporary file {temporary_path:?}: {error}"
                ),
            )
        })?;

        Ok(())
    })();
    drop(file);
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }

    // replace the destination with one fully-written temporary file
    replace_file(path, &temporary_path).map_err(|error| {
        let _ = fs::remove_file(&temporary_path);
        permission_denied(
            operation,
            format!("failed to install windows host keystore snapshot {path:?}: {error}"),
        )
    })?;

    // flush the final file handle after rename
    let file = File::open(path).map_err(|error| {
        permission_denied(
            operation,
            format!("failed to reopen windows host keystore snapshot {path:?}: {error}"),
        )
    })?;
    file.sync_all().map_err(|error| {
        permission_denied(
            operation,
            format!("failed to flush windows host keystore snapshot {path:?}: {error}"),
        )
    })?;

    Ok(())
}
