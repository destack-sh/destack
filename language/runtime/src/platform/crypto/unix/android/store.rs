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

/// Snapshot payload magic for encrypted Android host key-store blobs.
const ANDROID_HOST_SNAPSHOT_MAGIC: &[u8; 8] = b"DSCKEY01";
/// Snapshot payload version for encrypted Android host key-store blobs.
const ANDROID_HOST_SNAPSHOT_VERSION: u8 = 1;
/// AES-GCM nonce size for encrypted host snapshots.
const ANDROID_HOST_SNAPSHOT_NONCE_BYTES: usize = 12;
/// AES-GCM tag size for encrypted host snapshots.
const ANDROID_HOST_SNAPSHOT_TAG_BYTES: usize = 16;
/// AES-256 key size for encrypted host snapshots.
const ANDROID_HOST_SNAPSHOT_KEY_BYTES: usize = 32;
/// Authenticated context bound into snapshot encryption and decryption.
const ANDROID_HOST_SNAPSHOT_AAD: &[u8] = b"destack.crypto.android.snapshot.v1";

/// Global sequence used to generate unique write probe identifiers.
static STORE_WRITE_PROBE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Return whether one host lane has a writable persistent-key backend.
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
                format!("failed to read android host keystore snapshot {path:?}: {error}"),
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
                format!("failed to create android host keystore path {parent:?}: {error}"),
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
            format!("failed to write android host keystore snapshot {path:?}: {error}"),
        )
    })?;

    // harden local snapshot file mode
    #[cfg(target_family = "unix")]
    if matches!(kind, CryptoStoreKind::User | CryptoStoreKind::Machine) {
        let permissions = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&path, permissions).map_err(|error| {
            permission_denied(
                operation,
                format!("failed to set android host keystore permissions for {path:?}: {error}"),
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
) -> RuntimeResult<[u8; ANDROID_HOST_SNAPSHOT_KEY_BYTES]> {
    // read one existing sidecar key when present
    let key_path = snapshot_encryption_key_path(snapshot_path);
    match fs::read(&key_path) {
        Ok(key_bytes) => return decode_snapshot_encryption_key(&key_bytes, operation),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(permission_denied(
                operation,
                format!("failed to read android host keystore key {key_path:?}: {error}"),
            ));
        }
    }

    // generate one new random key and persist it with strict permissions
    let mut key = [0u8; ANDROID_HOST_SNAPSHOT_KEY_BYTES];
    rand_bytes(&mut key).map_err(|error| invalid_data(operation, format!("{error}")))?;

    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            permission_denied(
                operation,
                format!("failed to create android host keystore key path {parent:?}: {error}"),
            )
        })?;
    }

    fs::write(&key_path, key).map_err(|error| {
        permission_denied(
            operation,
            format!("failed to write android host keystore key {key_path:?}: {error}"),
        )
    })?;

    #[cfg(target_family = "unix")]
    {
        let permissions = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&key_path, permissions).map_err(|error| {
            permission_denied(
                operation,
                format!(
                    "failed to set android host keystore key permissions {key_path:?}: {error}"
                ),
            )
        })?;
    }

    Ok(key)
}

/// Decode one sidecar snapshot encryption key payload.
fn decode_snapshot_encryption_key(
    key_bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<[u8; ANDROID_HOST_SNAPSHOT_KEY_BYTES]> {
    if key_bytes.len() != ANDROID_HOST_SNAPSHOT_KEY_BYTES {
        return Err(invalid_data(
            operation,
            "android host keystore encryption key has one invalid length",
        ));
    }

    let mut key = [0u8; ANDROID_HOST_SNAPSHOT_KEY_BYTES];
    key.copy_from_slice(key_bytes);

    Ok(key)
}

/// Encode one clear snapshot payload into one authenticated encrypted blob.
fn encode_encrypted_snapshot_payload(
    clear_payload: &[u8],
    encryption_key: &[u8; ANDROID_HOST_SNAPSHOT_KEY_BYTES],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // generate one random nonce for this snapshot write
    let mut nonce = [0u8; ANDROID_HOST_SNAPSHOT_NONCE_BYTES];
    rand_bytes(&mut nonce).map_err(|error| invalid_data(operation, format!("{error}")))?;

    // encrypt clear payload with AES-256-GCM and authenticated context
    let cipher = Cipher::aes_256_gcm();
    let mut tag = [0u8; ANDROID_HOST_SNAPSHOT_TAG_BYTES];
    let ciphertext = encrypt_aead(
        cipher,
        encryption_key,
        Some(&nonce),
        ANDROID_HOST_SNAPSHOT_AAD,
        clear_payload,
        &mut tag,
    )
    .map_err(|error| invalid_data(operation, format!("{error}")))?;

    // assemble envelope as magic, version, nonce, tag, and ciphertext
    let mut payload = Vec::with_capacity(
        ANDROID_HOST_SNAPSHOT_MAGIC.len()
            + 1
            + ANDROID_HOST_SNAPSHOT_NONCE_BYTES
            + ANDROID_HOST_SNAPSHOT_TAG_BYTES
            + ciphertext.len(),
    );
    payload.extend_from_slice(ANDROID_HOST_SNAPSHOT_MAGIC);
    payload.push(ANDROID_HOST_SNAPSHOT_VERSION);
    payload.extend_from_slice(&nonce);
    payload.extend_from_slice(&tag);
    payload.extend_from_slice(&ciphertext);

    Ok(payload)
}

/// Decode one persisted snapshot payload into one clear snapshot blob.
fn decode_snapshot_payload(
    snapshot_path: &Path,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // treat legacy payloads without magic prefix as clear snapshots
    if payload.len() < ANDROID_HOST_SNAPSHOT_MAGIC.len()
        || &payload[..ANDROID_HOST_SNAPSHOT_MAGIC.len()] != ANDROID_HOST_SNAPSHOT_MAGIC
    {
        return Ok(payload.to_vec());
    }

    // validate envelope lengths and version
    let envelope_header_len = ANDROID_HOST_SNAPSHOT_MAGIC.len()
        + 1
        + ANDROID_HOST_SNAPSHOT_NONCE_BYTES
        + ANDROID_HOST_SNAPSHOT_TAG_BYTES;
    if payload.len() < envelope_header_len {
        return Err(invalid_data(
            operation,
            "android host keystore snapshot envelope is truncated",
        ));
    }
    let version_index = ANDROID_HOST_SNAPSHOT_MAGIC.len();
    let version = payload[version_index];
    if version != ANDROID_HOST_SNAPSHOT_VERSION {
        return Err(invalid_data(
            operation,
            format!("android host keystore snapshot uses one unsupported version {version}"),
        ));
    }

    // load encryption key and decrypt ciphertext payload
    let nonce_start = version_index + 1;
    let nonce_end = nonce_start + ANDROID_HOST_SNAPSHOT_NONCE_BYTES;
    let tag_end = nonce_end + ANDROID_HOST_SNAPSHOT_TAG_BYTES;
    let nonce = &payload[nonce_start..nonce_end];
    let tag = &payload[nonce_end..tag_end];
    let ciphertext = &payload[tag_end..];
    let encryption_key = load_or_create_snapshot_encryption_key(snapshot_path, operation)?;
    let cipher = Cipher::aes_256_gcm();
    decrypt_aead(
        cipher,
        &encryption_key,
        Some(nonce),
        ANDROID_HOST_SNAPSHOT_AAD,
        ciphertext,
        tag,
    )
    .map_err(|error| invalid_data(operation, format!("{error}")))
}
