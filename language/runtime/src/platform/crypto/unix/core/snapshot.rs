use std::ffi::{CString, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::OpenOptionsExt;
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
/// File mode used for local snapshot and key files.
const HOST_SNAPSHOT_FILE_MODE: u32 = 0o600;
/// Temporary-file prefix for atomic snapshot writes.
const HOST_SNAPSHOT_TEMPORARY_PREFIX: &str = ".destack-crypto-snapshot-";

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
    let _ = kind;
    write_file_atomically(&snapshot_path, &encrypted_payload, config, operation)?;

    Ok(())
}

/// Return whether one host snapshot path and key sidecar are writable.
fn probe_snapshot_backend_writeability(snapshot_path: &Path) -> bool {
    // resolve the snapshot and key parent directories
    let Some(snapshot_parent) = snapshot_path.parent() else {
        return false;
    };

    let key_path = snapshot_encryption_key_path(snapshot_path);
    let Some(key_parent) = key_path.parent() else {
        return false;
    };

    // require one writable existing ancestor for the snapshot path
    if !path_has_writable_existing_ancestor(snapshot_parent) {
        return false;
    }

    // require one writable existing ancestor for the key path
    if key_parent != snapshot_parent && !path_has_writable_existing_ancestor(key_parent) {
        return false;
    }

    true
}

/// Return whether one path has one writable existing ancestor directory.
fn path_has_writable_existing_ancestor(path: &Path) -> bool {
    // walk up to one existing ancestor without mutating the filesystem
    let mut current = Some(path);
    while let Some(candidate) = current {
        if candidate.is_dir() {
            return directory_is_writable(candidate);
        }

        current = candidate.parent();
    }

    false
}

/// Return whether one directory can be traversed and written.
fn directory_is_writable(directory: &Path) -> bool {
    let directory = match CString::new(directory.as_os_str().as_bytes()) {
        Ok(directory) => directory,
        Err(_) => return false,
    };

    unsafe { libc::access(directory.as_ptr(), libc::W_OK | libc::X_OK) == 0 }
}

/// Resolve one sidecar encryption key path for one host snapshot path.
fn snapshot_encryption_key_path(snapshot_path: &Path) -> PathBuf {
    let file_name = snapshot_path
        .file_name()
        .map(|value| {
            let mut file_name = value.to_os_string();
            file_name.push(".key");
            file_name
        })
        .unwrap_or_else(|| OsString::from("host-store.keys.key"));

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
    let mut key_file = match open_new_private_file(&key_path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let key_bytes = fs::read(&key_path).map_err(|error| {
                permission_denied(
                    operation,
                    format!(
                        "failed to read {} host keystore key {key_path:?}: {error}",
                        config.store_label,
                    ),
                )
            })?;

            return decode_snapshot_encryption_key(&key_bytes, config, operation);
        }
        Err(error) => {
            return Err(permission_denied(
                operation,
                format!(
                    "failed to create {} host keystore key {key_path:?}: {error}",
                    config.store_label,
                ),
            ));
        }
    };
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

    sync_parent_directory(&key_path, config, operation)?;

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

/// Open one new private file with user-only permissions.
fn open_new_private_file(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    options.mode(HOST_SNAPSHOT_FILE_MODE);

    options.open(path)
}

/// Open one temporary private file for one atomic write.
fn open_temporary_private_file(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    options.mode(HOST_SNAPSHOT_FILE_MODE);

    options.open(path)
}

/// Return one temporary path in the destination directory.
fn temporary_snapshot_path(path: &Path) -> PathBuf {
    let temporary_identifier = next_store_write_probe_identifier();
    let file_name = path
        .file_name()
        .map(|value| {
            let mut file_name = OsString::from(HOST_SNAPSHOT_TEMPORARY_PREFIX);
            file_name.push(&temporary_identifier);
            file_name.push(".");
            file_name.push(value);
            file_name
        })
        .unwrap_or_else(|| {
            let mut file_name = OsString::from(HOST_SNAPSHOT_TEMPORARY_PREFIX);
            file_name.push(&temporary_identifier);
            file_name.push(".snapshot");
            file_name
        });

    path.with_file_name(file_name)
}

/// Write one file atomically through one same-directory temporary path.
fn write_file_atomically(
    path: &Path,
    bytes: &[u8],
    config: SnapshotConfig,
    operation: &'static str,
) -> RuntimeResult<()> {
    // create one private temporary file in the destination directory
    let temporary_path = temporary_snapshot_path(path);
    let mut file = open_temporary_private_file(&temporary_path).map_err(|error| {
        permission_denied(
            operation,
            format!(
                "failed to create {} host keystore temporary file {temporary_path:?}: {error}",
                config.store_label,
            ),
        )
    })?;

    // write and flush the temporary payload before rename
    let write_result = (|| -> RuntimeResult<()> {
        file.write_all(bytes).map_err(|error| {
            permission_denied(
                operation,
                format!(
                    "failed to write {} host keystore temporary file {temporary_path:?}: {error}",
                    config.store_label,
                ),
            )
        })?;
        file.sync_all().map_err(|error| {
            permission_denied(
                operation,
                format!(
                    "failed to flush {} host keystore temporary file {temporary_path:?}: {error}",
                    config.store_label,
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
    fs::rename(&temporary_path, path).map_err(|error| {
        let _ = fs::remove_file(&temporary_path);
        permission_denied(
            operation,
            format!(
                "failed to install {} host keystore snapshot {path:?}: {error}",
                config.store_label,
            ),
        )
    })?;
    sync_parent_directory(path, config, operation)?;

    Ok(())
}

/// Flush one parent directory after creating or replacing one file.
fn sync_parent_directory(
    path: &Path,
    config: SnapshotConfig,
    operation: &'static str,
) -> RuntimeResult<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };

    let directory = File::open(parent).map_err(|error| {
        permission_denied(
            operation,
            format!(
                "failed to open {} host keystore parent directory {parent:?}: {error}",
                config.store_label,
            ),
        )
    })?;
    directory.sync_all().map_err(|error| {
        permission_denied(
            operation,
            format!(
                "failed to flush {} host keystore parent directory {parent:?}: {error}",
                config.store_label,
            ),
        )
    })?;

    Ok(())
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

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::fs;
    use std::os::unix::ffi::OsStringExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::platform::crypto::CryptoStoreKind;

    use super::{
        HOST_SNAPSHOT_MAGIC, HOST_SNAPSHOT_TEMPORARY_PREFIX, SnapshotConfig,
        host_store_persistence_backend_is_available, load_host_key_snapshot_bytes,
        snapshot_encryption_key_path, store_host_key_snapshot_bytes, temporary_snapshot_path,
    };

    /// Snapshot codec configuration for unit tests.
    const TEST_SNAPSHOT_CONFIG: SnapshotConfig = SnapshotConfig {
        store_label: "test",
        associated_data: b"destack.crypto.snapshot.test.v1",
    };

    /// Return one unique temporary snapshot path.
    fn unique_snapshot_path(label: &str) -> std::path::PathBuf {
        let unique_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after the unix epoch")
            .as_nanos();
        std::env::temp_dir()
            .join(format!(
                "destack-crypto-snapshot-test-{label}-{unique_nanos}"
            ))
            .join("user-store.keys")
    }

    /// Keep persistence probes side-effect free.
    #[test]
    fn test_host_store_persistence_probe_does_not_create_directories() {
        // probe one nested snapshot path under a unique temporary root
        let unique_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after the unix epoch")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("destack-crypto-snapshot-probe-{unique_nanos}"));
        let snapshot_path = root.join("nested/user-store.keys");
        let key_path = snapshot_path.with_file_name("user-store.keys.key");
        assert!(!root.exists());

        // report no backend availability without creating any probe artifacts
        let is_available = host_store_persistence_backend_is_available(Some(snapshot_path));
        assert!(!is_available);
        assert!(!root.exists());
        assert!(!key_path.exists());
    }

    /// Persist encrypted snapshots and allow atomic replacement writes.
    #[test]
    fn test_store_host_key_snapshot_bytes_encrypts_and_overwrites() {
        // write one initial snapshot payload
        let snapshot_path = unique_snapshot_path("overwrite");
        let first_plaintext = b"first host snapshot payload";
        store_host_key_snapshot_bytes(
            Some(snapshot_path.clone()),
            CryptoStoreKind::User,
            first_plaintext,
            TEST_SNAPSHOT_CONFIG,
            "destack.crypto.test.storeSnapshot",
        )
        .expect("initial snapshot write should succeed");

        // ensure the raw on-disk payload is encrypted and versioned
        let raw_bytes = fs::read(&snapshot_path).expect("stored snapshot should exist");
        assert_ne!(raw_bytes, first_plaintext);
        assert!(raw_bytes.starts_with(HOST_SNAPSHOT_MAGIC));

        // replace the snapshot payload and verify the decoded bytes change
        let second_plaintext = b"second host snapshot payload";
        store_host_key_snapshot_bytes(
            Some(snapshot_path.clone()),
            CryptoStoreKind::User,
            second_plaintext,
            TEST_SNAPSHOT_CONFIG,
            "destack.crypto.test.storeSnapshot",
        )
        .expect("replacement snapshot write should succeed");
        let loaded_bytes = load_host_key_snapshot_bytes(
            Some(snapshot_path.clone()),
            TEST_SNAPSHOT_CONFIG,
            "destack.crypto.test.loadSnapshot",
        )
        .expect("snapshot load should succeed")
        .expect("snapshot bytes should exist");
        assert_eq!(loaded_bytes, second_plaintext);

        // cleanup
        let _ = fs::remove_file(snapshot_path.with_file_name("user-store.keys.key"));
        let _ = fs::remove_file(&snapshot_path);
        let _ = snapshot_path.parent().map(fs::remove_dir_all);
    }

    /// Preserve non-UTF8 snapshot file names when deriving sibling paths.
    #[test]
    fn test_snapshot_paths_preserve_non_utf8_file_names() {
        // build one snapshot path with one non-utf8 file name
        let snapshot_path = std::env::temp_dir().join(OsString::from_vec(vec![
            b'u', b's', b'e', b'r', b'-', b's', b't', b'o', b'r', b'e', b'.', b'k', b'e', b'y',
            0xff,
        ]));

        // derive the sidecar key and temporary paths without collapsing the file name
        let key_path = snapshot_encryption_key_path(&snapshot_path);
        let temporary_path = temporary_snapshot_path(&snapshot_path);

        // keep the original byte payload in both derived sibling paths
        let snapshot_name = snapshot_path
            .file_name()
            .expect("snapshot file name should exist");
        let snapshot_name = snapshot_name.as_encoded_bytes();
        let key_name = key_path.file_name().expect("key file name should exist");
        let key_name = key_name.as_encoded_bytes();
        let temporary_name = temporary_path
            .file_name()
            .expect("temporary file name should exist");
        let temporary_name = temporary_name.as_encoded_bytes();
        assert!(key_name.starts_with(snapshot_name));
        assert!(key_name.ends_with(b".key"));
        assert!(temporary_name.starts_with(HOST_SNAPSHOT_TEMPORARY_PREFIX.as_bytes()));
        assert!(temporary_name.ends_with(snapshot_name));
    }
}
