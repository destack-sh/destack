use std::fs::{self, OpenOptions};
use std::io::Write;
#[cfg(target_family = "unix")]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use openssl::rand::rand_bytes;
use openssl::symm::{Cipher, decrypt_aead, encrypt_aead};
use openssl::x509::X509;

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::CryptoStoreKind;
use crate::platform::crypto::core::CRYPTO_STORE_OPEN_OPERATION;
use crate::runtime::BindingCallContext;

use super::certificate::{collect_system_certificates, has_system_certificate_source};
use super::core::{invalid_data, keystore_path, not_supported, permission_denied};

/// Snapshot payload magic for encrypted POSIX host key-store blobs.
const POSIX_HOST_SNAPSHOT_MAGIC: &[u8; 8] = b"DSCKEY01";
/// Snapshot payload version for encrypted POSIX host key-store blobs.
const POSIX_HOST_SNAPSHOT_VERSION: u8 = 1;
/// AES-GCM nonce size for encrypted host snapshots.
const POSIX_HOST_SNAPSHOT_NONCE_BYTES: usize = 12;
/// AES-GCM tag size for encrypted host snapshots.
const POSIX_HOST_SNAPSHOT_TAG_BYTES: usize = 16;
/// AES-256 key size for encrypted host snapshots.
const POSIX_HOST_SNAPSHOT_KEY_BYTES: usize = 32;
/// Authenticated context bound into snapshot encryption and decryption.
const POSIX_HOST_SNAPSHOT_AAD: &[u8] = b"destack.crypto.posix.snapshot.v1";

/// Global sequence used to generate unique write probe identifiers.
static STORE_WRITE_PROBE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(crate) fn host_store_persistence_backend_is_available(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    // require a resolved lane path before probing writeability
    let Some(path) = keystore_path(context, kind) else {
        return false;
    };

    // probe snapshot and key sidecar writeability without mutating active payloads
    probe_snapshot_backend_writeability(&path)
}

/// Return whether one host store lane is currently available.
pub(crate) fn host_store_lane_is_available(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    match kind {
        CryptoStoreKind::System => has_system_certificate_source(context),
        CryptoStoreKind::User => keystore_path(context, CryptoStoreKind::User).is_some(),
        CryptoStoreKind::Machine => {
            has_system_certificate_source(context)
                || keystore_path(context, CryptoStoreKind::Machine).is_some()
        }
        CryptoStoreKind::Provider | CryptoStoreKind::Ephemeral => false,
    }
}

/// Open one host store lane and return certificate snapshots.
pub(crate) fn open_host_store_certificates(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> RuntimeResult<Vec<X509>> {
    match kind {
        CryptoStoreKind::System => Ok(collect_system_certificates(context)),
        CryptoStoreKind::User => Ok(Vec::new()),
        CryptoStoreKind::Machine => Ok(collect_system_certificates(context)),
        CryptoStoreKind::Provider => Err(not_supported(CRYPTO_STORE_OPEN_OPERATION)),
        CryptoStoreKind::Ephemeral => Ok(Vec::new()),
    }
}

/// Load one backend host-key snapshot payload for one store lane.
pub(crate) fn load_host_key_snapshot_bytes(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    operation: &'static str,
) -> RuntimeResult<Option<Vec<u8>>> {
    // resolve lane snapshot path and return none for unsupported lanes
    let Some(path) = keystore_path(context, kind) else {
        return Ok(None);
    };

    // read encrypted or legacy snapshot payload
    let snapshot_payload = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            if error.kind() == std::io::ErrorKind::NotFound {
                return Ok(None);
            }

            return Err(permission_denied(
                operation,
                format!("failed to read unix host keystore snapshot {path:?}: {error}"),
            ));
        }
    };

    // decode payload as encrypted snapshot when the magic prefix is present
    let snapshot_bytes = decode_snapshot_payload(&path, &snapshot_payload, operation)?;

    Ok(Some(snapshot_bytes))
}

/// Store one backend host-key snapshot payload for one store lane.
pub(crate) fn store_host_key_snapshot_bytes(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    snapshot_bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve lane snapshot path and reject unsupported lanes
    let Some(path) = keystore_path(context, kind) else {
        return Err(not_supported(operation));
    };

    // ensure parent directories exist before writing
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            permission_denied(
                operation,
                format!("failed to create unix host keystore path {parent:?}: {error}"),
            )
        })?;
    }

    // encrypt snapshot bytes with one local sidecar key before persisting
    let encryption_key = load_or_create_snapshot_encryption_key(&path, operation)?;
    let encrypted_payload =
        encode_encrypted_snapshot_payload(snapshot_bytes, &encryption_key, operation)?;
    fs::write(&path, encrypted_payload).map_err(|error| {
        permission_denied(
            operation,
            format!("failed to write unix host keystore snapshot {path:?}: {error}"),
        )
    })?;

    // harden local snapshot file mode
    #[cfg(target_family = "unix")]
    if matches!(kind, CryptoStoreKind::User | CryptoStoreKind::Machine) {
        let permissions = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&path, permissions).map_err(|error| {
            permission_denied(
                operation,
                format!("failed to set unix host keystore permissions for {path:?}: {error}"),
            )
        })?;
    }

    Ok(())
}

/// Build one unique probe identifier for temporary backend probes.
fn next_store_write_probe_identifier() -> String {
    // include one process-local sequence to avoid collisions in one runtime
    let sequence = STORE_WRITE_PROBE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let process_id = std::process::id();

    // include one wall-clock component to avoid collisions across process restarts
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);

    format!("{process_id}.{timestamp}.{sequence}")
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
    operation: &'static str,
) -> RuntimeResult<[u8; POSIX_HOST_SNAPSHOT_KEY_BYTES]> {
    // read one existing sidecar key when present
    let key_path = snapshot_encryption_key_path(snapshot_path);
    match fs::read(&key_path) {
        Ok(key_bytes) => return decode_snapshot_encryption_key(&key_bytes, operation),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(permission_denied(
                operation,
                format!("failed to read unix host keystore key {key_path:?}: {error}"),
            ));
        }
    }

    // create parent directories before writing one new sidecar key
    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            permission_denied(
                operation,
                format!("failed to create unix host keystore key path {parent:?}: {error}"),
            )
        })?;
    }

    // generate one random key payload
    let mut key = [0u8; POSIX_HOST_SNAPSHOT_KEY_BYTES];
    rand_bytes(&mut key).map_err(|error| {
        invalid_data(
            operation,
            format!("failed to generate snapshot key: {error}"),
        )
    })?;

    // write the key atomically when the file does not yet exist
    let mut key_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&key_path)
        .map_err(|error| {
            permission_denied(
                operation,
                format!("failed to create unix host keystore key {key_path:?}: {error}"),
            )
        })?;
    key_file.write_all(&key).map_err(|error| {
        permission_denied(
            operation,
            format!("failed to write unix host keystore key {key_path:?}: {error}"),
        )
    })?;
    key_file.sync_all().map_err(|error| {
        permission_denied(
            operation,
            format!("failed to flush unix host keystore key {key_path:?}: {error}"),
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
                    "failed to set unix host keystore key permissions for {key_path:?}: {error}"
                ),
            )
        })?;
    }

    Ok(key)
}

/// Decode one snapshot encryption key payload.
fn decode_snapshot_encryption_key(
    key_bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<[u8; POSIX_HOST_SNAPSHOT_KEY_BYTES]> {
    // validate key size and copy into one fixed-size key array
    if key_bytes.len() != POSIX_HOST_SNAPSHOT_KEY_BYTES {
        return Err(invalid_data(
            operation,
            "unix host keystore key has one invalid size",
        ));
    }
    let mut key = [0u8; POSIX_HOST_SNAPSHOT_KEY_BYTES];
    key.copy_from_slice(key_bytes);

    Ok(key)
}

/// Decode one stored snapshot payload as encrypted or legacy plaintext bytes.
fn decode_snapshot_payload(
    snapshot_path: &Path,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // preserve plaintext legacy payloads that predate encrypted storage
    if !payload.starts_with(POSIX_HOST_SNAPSHOT_MAGIC) {
        return Ok(payload.to_vec());
    }

    // validate encrypted payload envelope size and version
    let header_size = POSIX_HOST_SNAPSHOT_MAGIC.len()
        + 1
        + POSIX_HOST_SNAPSHOT_NONCE_BYTES
        + POSIX_HOST_SNAPSHOT_TAG_BYTES;
    if payload.len() < header_size {
        return Err(invalid_data(
            operation,
            "unix host keystore snapshot payload is truncated",
        ));
    }
    let version_offset = POSIX_HOST_SNAPSHOT_MAGIC.len();
    let version = payload[version_offset];
    if version != POSIX_HOST_SNAPSHOT_VERSION {
        return Err(invalid_data(
            operation,
            "unix host keystore snapshot payload has one unsupported version",
        ));
    }

    // parse encrypted payload lanes
    let nonce_start = version_offset + 1;
    let nonce_end = nonce_start + POSIX_HOST_SNAPSHOT_NONCE_BYTES;
    let tag_end = nonce_end + POSIX_HOST_SNAPSHOT_TAG_BYTES;
    let nonce = &payload[nonce_start..nonce_end];
    let tag = &payload[nonce_end..tag_end];
    let ciphertext = &payload[tag_end..];

    // decrypt payload with one sidecar encryption key
    let key = load_or_create_snapshot_encryption_key(snapshot_path, operation)?;
    decrypt_aead(
        Cipher::aes_256_gcm(),
        &key,
        Some(nonce),
        POSIX_HOST_SNAPSHOT_AAD,
        ciphertext,
        tag,
    )
    .map_err(|error| {
        invalid_data(
            operation,
            format!("failed to decrypt unix host keystore snapshot payload: {error}"),
        )
    })
}

/// Encode one plaintext snapshot payload into one encrypted envelope.
fn encode_encrypted_snapshot_payload(
    snapshot_bytes: &[u8],
    key: &[u8; POSIX_HOST_SNAPSHOT_KEY_BYTES],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // generate one random nonce and encrypt payload bytes
    let mut nonce = [0u8; POSIX_HOST_SNAPSHOT_NONCE_BYTES];
    rand_bytes(&mut nonce).map_err(|error| {
        invalid_data(
            operation,
            format!("failed to generate unix host snapshot nonce: {error}"),
        )
    })?;
    let mut tag = [0u8; POSIX_HOST_SNAPSHOT_TAG_BYTES];
    let ciphertext = encrypt_aead(
        Cipher::aes_256_gcm(),
        key,
        Some(&nonce),
        POSIX_HOST_SNAPSHOT_AAD,
        snapshot_bytes,
        &mut tag,
    )
    .map_err(|error| {
        invalid_data(
            operation,
            format!("failed to encrypt unix host keystore snapshot payload: {error}"),
        )
    })?;

    // assemble encrypted payload envelope
    let mut payload = Vec::with_capacity(
        POSIX_HOST_SNAPSHOT_MAGIC.len() + 1 + nonce.len() + tag.len() + ciphertext.len(),
    );
    payload.extend_from_slice(POSIX_HOST_SNAPSHOT_MAGIC);
    payload.push(POSIX_HOST_SNAPSHOT_VERSION);
    payload.extend_from_slice(&nonce);
    payload.extend_from_slice(&tag);
    payload.extend_from_slice(&ciphertext);

    Ok(payload)
}
