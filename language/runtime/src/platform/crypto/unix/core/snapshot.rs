use std::fs::{self, OpenOptions};
use std::io::Write;
#[cfg(target_family = "unix")]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use openssl::rand::rand_bytes;
use openssl::symm::{Cipher, decrypt_aead, encrypt_aead};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::crypto::CryptoStoreKind;

use super::{invalid_data, next_store_write_probe_identifier, permission_denied};

/// Snapshot payload magic for encrypted host key-store blobs.
const HOST_SNAPSHOT_MAGIC: &[u8; 8] = b"DSCKEY01";
/// Snapshot payload version for encrypted host key-store blobs.
const HOST_SNAPSHOT_VERSION: u8 = 1;
/// AES-GCM nonce size for encrypted host snapshots.
const HOST_SNAPSHOT_NONCE_BYTES: usize = 12;
/// AES-GCM tag size for encrypted host snapshots.
const HOST_SNAPSHOT_TAG_BYTES: usize = 16;
/// AES-256 key size for encrypted host snapshots.
const HOST_SNAPSHOT_KEY_BYTES: usize = 32;

/// Snapshot codec and error-label configuration.
#[derive(Clone, Copy)]
pub(crate) struct SnapshotConfig {
    /// Human-readable host label used for diagnostics.
    pub(crate) store_label: &'static str,
    /// Authenticated additional data bound to ciphertext payloads.
    pub(crate) associated_data: &'static [u8],
}

/// Return whether one host lane has one writable persistent snapshot backend.
pub(crate) fn host_store_persistence_backend_is_available(snapshot_path: Option<PathBuf>) -> bool {
    let Some(snapshot_path) = snapshot_path else {
        return false;
    };

    probe_snapshot_backend_writeability(&snapshot_path)
}

/// Load one backend host-key snapshot payload for one store lane.
pub(crate) fn load_host_key_snapshot_bytes(
    snapshot_path: Option<PathBuf>,
    config: SnapshotConfig,
    operation: &'static str,
) -> RuntimeResult<Option<Vec<u8>>> {
    // return none for lanes that do not resolve to one snapshot path
    let Some(snapshot_path) = snapshot_path else {
        return Ok(None);
    };

    // read encrypted or legacy snapshot payload
    let snapshot_payload = match fs::read(&snapshot_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            if error.kind() == std::io::ErrorKind::NotFound {
                return Ok(None);
            }

            return Err(permission_denied(
                operation,
                format!(
                    "failed to read {} host keystore snapshot {snapshot_path:?}: {error}",
                    config.store_label,
                ),
            ));
        }
    };

    // decode payload as encrypted snapshot when the magic prefix is present
    let snapshot_bytes =
        decode_snapshot_payload(&snapshot_path, &snapshot_payload, config, operation)?;

    Ok(Some(snapshot_bytes))
}

/// Store one backend host-key snapshot payload for one store lane.
pub(crate) fn store_host_key_snapshot_bytes(
    snapshot_path: Option<PathBuf>,
    kind: CryptoStoreKind,
    snapshot_bytes: &[u8],
    config: SnapshotConfig,
    operation: &'static str,
) -> RuntimeResult<()> {
    // reject lanes that do not resolve to one snapshot path
    let Some(snapshot_path) = snapshot_path else {
        return Err(core_platform::not_supported(operation));
    };

    // ensure parent directories exist before writing
    if let Some(parent) = snapshot_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            permission_denied(
                operation,
                format!(
                    "failed to create {} host keystore path {parent:?}: {error}",
                    config.store_label,
                ),
            )
        })?;
    }

    // encrypt snapshot bytes with one local sidecar key before persisting
    let encryption_key = load_or_create_snapshot_encryption_key(&snapshot_path, config, operation)?;
    let encrypted_payload =
        encode_encrypted_snapshot_payload(snapshot_bytes, &encryption_key, config, operation)?;
    fs::write(&snapshot_path, encrypted_payload).map_err(|error| {
        permission_denied(
            operation,
            format!(
                "failed to write {} host keystore snapshot {snapshot_path:?}: {error}",
                config.store_label,
            ),
        )
    })?;

    // harden local snapshot file mode
    #[cfg(target_family = "unix")]
    if matches!(kind, CryptoStoreKind::User | CryptoStoreKind::Machine) {
        let permissions = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&snapshot_path, permissions).map_err(|error| {
            permission_denied(
                operation,
                format!(
                    "failed to set {} host keystore permissions for {snapshot_path:?}: {error}",
                    config.store_label,
                ),
            )
        })?;
    }

    Ok(())
}

/// Return whether one host snapshot path and key sidecar are writable.
fn probe_snapshot_backend_writeability(snapshot_path: &Path) -> bool {
    // resolve the snapshot and key parent directories
    let Some(snapshot_parent) = snapshot_path.parent() else {
        return false;
    };
    if fs::create_dir_all(snapshot_parent).is_err() {
        return false;
    }

    let key_path = snapshot_encryption_key_path(snapshot_path);
    let Some(key_parent) = key_path.parent() else {
        return false;
    };
    if fs::create_dir_all(key_parent).is_err() {
        return false;
    }

    // probe snapshot-parent writeability
    if !probe_directory_writeability(snapshot_parent) {
        return false;
    }

    // probe key-parent writeability when it differs from the snapshot parent
    if key_parent != snapshot_parent && !probe_directory_writeability(key_parent) {
        return false;
    }

    true
}

/// Return whether one directory is writable by creating one disposable probe file.
fn probe_directory_writeability(directory: &Path) -> bool {
    // create one short-lived probe file and write one payload
    let probe_identifier = next_store_write_probe_identifier();
    let probe_path = directory.join(format!(".destack-crypto-write-probe-{probe_identifier}"));
    let mut probe_file = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe_path)
    {
        Ok(file) => file,
        Err(_) => return false,
    };
    if probe_file.write_all(b"probe").is_err() {
        let _ = fs::remove_file(&probe_path);
        return false;
    }
    if probe_file.sync_all().is_err() {
        let _ = fs::remove_file(&probe_path);
        return false;
    }

    // remove the probe file and report final result
    fs::remove_file(&probe_path).is_ok()
}

/// Resolve one sidecar encryption key path for one host snapshot path.
fn snapshot_encryption_key_path(snapshot_path: &Path) -> PathBuf {
    let file_name = snapshot_path
        .file_name()
        .and_then(|value| value.to_str())
        .map(|value| format!("{value}.key"))
        .unwrap_or_else(|| "host-store.keys.key".to_string());

    snapshot_path.with_file_name(file_name)
}

/// Load one snapshot encryption key or create one new key when missing.
fn load_or_create_snapshot_encryption_key(
    snapshot_path: &Path,
    config: SnapshotConfig,
    operation: &'static str,
) -> RuntimeResult<[u8; HOST_SNAPSHOT_KEY_BYTES]> {
    // read one existing sidecar key when present
    let key_path = snapshot_encryption_key_path(snapshot_path);
    match fs::read(&key_path) {
        Ok(key_bytes) => return decode_snapshot_encryption_key(&key_bytes, config, operation),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(permission_denied(
                operation,
                format!(
                    "failed to read {} host keystore key {key_path:?}: {error}",
                    config.store_label,
                ),
            ));
        }
    }

    // create parent directories before writing one new sidecar key
    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            permission_denied(
                operation,
                format!(
                    "failed to create {} host keystore key path {parent:?}: {error}",
                    config.store_label,
                ),
            )
        })?;
    }

    // generate one random key payload
    let mut key = [0u8; HOST_SNAPSHOT_KEY_BYTES];
    rand_bytes(&mut key).map_err(|error| invalid_data(operation, format!("{error}")))?;

    // write the key atomically when the file does not yet exist
    let mut key_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&key_path)
        .map_err(|error| {
            permission_denied(
                operation,
                format!(
                    "failed to create {} host keystore key {key_path:?}: {error}",
                    config.store_label,
                ),
            )
        })?;
    key_file.write_all(&key).map_err(|error| {
        permission_denied(
            operation,
            format!(
                "failed to write {} host keystore key {key_path:?}: {error}",
                config.store_label,
            ),
        )
    })?;
    key_file.sync_all().map_err(|error| {
        permission_denied(
            operation,
            format!(
                "failed to flush {} host keystore key {key_path:?}: {error}",
                config.store_label,
            ),
        )
    })?;

    // harden key file mode after create
    #[cfg(target_family = "unix")]
    {
        let permissions = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&key_path, permissions).map_err(|error| {
            permission_denied(
                operation,
                format!(
                    "failed to set {} host keystore key permissions for {key_path:?}: {error}",
                    config.store_label,
                ),
            )
        })?;
    }

    Ok(key)
}

/// Decode one snapshot encryption key payload.
fn decode_snapshot_encryption_key(
    key_bytes: &[u8],
    config: SnapshotConfig,
    operation: &'static str,
) -> RuntimeResult<[u8; HOST_SNAPSHOT_KEY_BYTES]> {
    // validate key size and copy into one fixed-size key array
    if key_bytes.len() != HOST_SNAPSHOT_KEY_BYTES {
        return Err(invalid_data(
            operation,
            format!(
                "{} host keystore key has one invalid size",
                config.store_label
            ),
        ));
    }
    let mut key = [0u8; HOST_SNAPSHOT_KEY_BYTES];
    key.copy_from_slice(key_bytes);

    Ok(key)
}

/// Decode one stored snapshot payload as encrypted or legacy plaintext bytes.
fn decode_snapshot_payload(
    snapshot_path: &Path,
    payload: &[u8],
    config: SnapshotConfig,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // preserve plaintext legacy payloads that predate encrypted storage
    if !payload.starts_with(HOST_SNAPSHOT_MAGIC) {
        return Ok(payload.to_vec());
    }

    // validate encrypted payload envelope size and version
    let header_size =
        HOST_SNAPSHOT_MAGIC.len() + 1 + HOST_SNAPSHOT_NONCE_BYTES + HOST_SNAPSHOT_TAG_BYTES;
    if payload.len() < header_size {
        return Err(invalid_data(
            operation,
            format!(
                "{} host keystore snapshot payload is truncated",
                config.store_label
            ),
        ));
    }
    let version_offset = HOST_SNAPSHOT_MAGIC.len();
    let version = payload[version_offset];
    if version != HOST_SNAPSHOT_VERSION {
        return Err(invalid_data(
            operation,
            format!(
                "{} host keystore snapshot payload has one unsupported version",
                config.store_label,
            ),
        ));
    }

    // parse encrypted payload lanes
    let nonce_start = version_offset + 1;
    let nonce_end = nonce_start + HOST_SNAPSHOT_NONCE_BYTES;
    let tag_end = nonce_end + HOST_SNAPSHOT_TAG_BYTES;
    let nonce = &payload[nonce_start..nonce_end];
    let tag = &payload[nonce_end..tag_end];
    let ciphertext = &payload[tag_end..];

    // decrypt payload with one sidecar encryption key
    let key = load_or_create_snapshot_encryption_key(snapshot_path, config, operation)?;
    decrypt_aead(
        Cipher::aes_256_gcm(),
        &key,
        Some(nonce),
        config.associated_data,
        ciphertext,
        tag,
    )
    .map_err(|error| {
        invalid_data(
            operation,
            format!(
                "failed to decrypt {} host keystore snapshot payload: {error}",
                config.store_label,
            ),
        )
    })
}

/// Encode one plaintext snapshot payload into one encrypted envelope.
fn encode_encrypted_snapshot_payload(
    snapshot_bytes: &[u8],
    key: &[u8; HOST_SNAPSHOT_KEY_BYTES],
    config: SnapshotConfig,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // generate one random nonce and encrypt payload bytes
    let mut nonce = [0u8; HOST_SNAPSHOT_NONCE_BYTES];
    rand_bytes(&mut nonce).map_err(|error| invalid_data(operation, format!("{error}")))?;
    let mut tag = [0u8; HOST_SNAPSHOT_TAG_BYTES];
    let ciphertext = encrypt_aead(
        Cipher::aes_256_gcm(),
        key,
        Some(&nonce),
        config.associated_data,
        snapshot_bytes,
        &mut tag,
    )
    .map_err(|error| {
        invalid_data(
            operation,
            format!(
                "failed to encrypt {} host keystore snapshot payload: {error}",
                config.store_label,
            ),
        )
    })?;

    // assemble encrypted payload envelope
    let mut payload = Vec::with_capacity(
        HOST_SNAPSHOT_MAGIC.len() + 1 + nonce.len() + tag.len() + ciphertext.len(),
    );
    payload.extend_from_slice(HOST_SNAPSHOT_MAGIC);
    payload.push(HOST_SNAPSHOT_VERSION);
    payload.extend_from_slice(&nonce);
    payload.extend_from_slice(&tag);
    payload.extend_from_slice(&ciphertext);

    Ok(payload)
}
