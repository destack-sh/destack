use std::sync::Arc;

use openssl::symm::{Cipher, Crypter, Mode};
use parking_lot::Mutex;
use zeroize::{Zeroize, Zeroizing};

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::{
    CryptoCipherAlgorithm, CryptoCipherDirection, CryptoCipherParameters, CryptoKeyAlgorithm,
    CryptoStoreKind, host as crypto_host,
};
use crate::platform::resource::ResourceEntry;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::constants::DEFAULT_AEAD_TAG_LENGTH_BYTES;
use super::core::{
    CRYPTO_CIPHER_LABEL, CRYPTO_CIPHER_RESOURCE_KIND, CryptoCipherResource, CryptoCipherState,
    HostKeyMaterial, KEY_USAGE_DECRYPT, KEY_USAGE_ENCRYPT, decode_native_bytes, openssl_error,
    resolve_cipher_resource,
};
use super::key::{require_key_usage, resolve_host_secret_key_material, resolve_secret_key_bytes};

/// Return the effective AEAD tag length lane.
fn tag_length_bytes(parameters: CryptoCipherParameters) -> u32 {
    parameters.tag_length_bytes.unwrap_or(0)
}

/// Encrypt one payload in one shot.
pub(crate) fn cipher_encrypt(
    binding: &BindingCallContext,
    key: resource::CryptoKeyHandle,
    parameters: CryptoCipherParameters,
    payload: &[u8],
) -> RuntimeResult<(Vec<u8>, Vec<u8>)> {
    // enforce key usage policy
    require_key_usage(
        binding,
        key,
        KEY_USAGE_ENCRYPT,
        "destack.crypto.cipher.encrypt",
    )?;

    // route host-managed secret-key lanes through host cipher primitives
    if let Some((host_key, store_kind, key_algorithm)) =
        resolve_host_secret_key_material(binding, key, "destack.crypto.cipher.encrypt")?
    {
        return crypto_host::host_key_cipher_encrypt(
            binding,
            &host_key,
            store_kind,
            key_algorithm,
            parameters,
            payload,
            "destack.crypto.cipher.encrypt",
        );
    }

    // resolve secret key bytes and keep them zeroized on all paths
    let key_bytes = resolve_secret_key_bytes(binding, key, "destack.crypto.cipher.encrypt")?;
    let key_bytes = Zeroizing::new(key_bytes);

    // process cipher operation
    cipher_process(
        &key_bytes,
        CryptoCipherDirection::Encrypt,
        parameters,
        payload,
        "destack.crypto.cipher.encrypt",
    )
}

/// Decrypt one payload in one shot.
pub(crate) fn cipher_decrypt(
    binding: &BindingCallContext,
    key: resource::CryptoKeyHandle,
    parameters: CryptoCipherParameters,
    payload: &[u8],
) -> RuntimeResult<Vec<u8>> {
    // enforce key usage policy
    require_key_usage(
        binding,
        key,
        KEY_USAGE_DECRYPT,
        "destack.crypto.cipher.decrypt",
    )?;

    // route host-managed secret-key lanes through host cipher primitives
    if let Some((host_key, store_kind, key_algorithm)) =
        resolve_host_secret_key_material(binding, key, "destack.crypto.cipher.decrypt")?
    {
        return crypto_host::host_key_cipher_decrypt(
            binding,
            &host_key,
            store_kind,
            key_algorithm,
            parameters,
            payload,
            "destack.crypto.cipher.decrypt",
        );
    }

    // resolve secret key bytes and keep them zeroized on all paths
    let key_bytes = resolve_secret_key_bytes(binding, key, "destack.crypto.cipher.decrypt")?;
    let key_bytes = Zeroizing::new(key_bytes);

    // process cipher operation and return decrypted output
    let (output, _) = cipher_process(
        &key_bytes,
        CryptoCipherDirection::Decrypt,
        parameters,
        payload,
        "destack.crypto.cipher.decrypt",
    )?;

    Ok(output)
}

/// Open one streaming cipher context.
pub(crate) fn cipher_open(
    binding: &BindingCallContext,
    key: resource::CryptoKeyHandle,
    direction: CryptoCipherDirection,
    parameters: CryptoCipherParameters,
) -> RuntimeResult<resource::CryptoCipherHandle> {
    // map stream direction to key usage policy
    let required_usage = match direction {
        CryptoCipherDirection::Encrypt => KEY_USAGE_ENCRYPT,
        CryptoCipherDirection::Decrypt => KEY_USAGE_DECRYPT,
    };

    // enforce key usage policy
    require_key_usage(binding, key, required_usage, "destack.crypto.cipher.open")?;

    // build host-secret stream state when this key is host managed
    if let Some((material, store_kind, key_algorithm)) =
        resolve_host_secret_key_material(binding, key, "destack.crypto.cipher.open")?
    {
        let (nonce, additional_data, decrypt_tag) =
            prepare_host_cipher_stream_parameters(direction, parameters)?;
        let resource_value = CryptoCipherResource {
            algorithm: parameters.algorithm,
            direction,
            tag_length_bytes: tag_length_bytes(parameters),
            state: CryptoCipherState::HostSecret {
                material,
                store_kind,
                key_algorithm,
                nonce,
                decrypt_tag,
                additional_data,
                payload: Vec::new(),
            },
        };
        let entry = ResourceEntry::new(CRYPTO_CIPHER_RESOURCE_KIND)
            .with_label(CRYPTO_CIPHER_LABEL)
            .with_payload(Arc::new(Mutex::new(resource_value)));
        let resource_id =
            binding
                .worker()
                .resources
                .insert(binding.world(), entry, Some(binding.engine()));

        return Ok(resource::CryptoCipherHandle(resource_id));
    }

    // resolve key bytes and initialize streaming cipher state
    let key = resolve_secret_key_bytes(binding, key, "destack.crypto.cipher.open")?;
    let crypter = build_cipher_state(&key, direction, parameters, "destack.crypto.cipher.open")?;

    // publish cipher resource
    let resource_value = CryptoCipherResource {
        algorithm: parameters.algorithm,
        direction,
        tag_length_bytes: tag_length_bytes(parameters),
        state: CryptoCipherState::Software { key, crypter },
    };
    let entry = ResourceEntry::new(CRYPTO_CIPHER_RESOURCE_KIND)
        .with_label(CRYPTO_CIPHER_LABEL)
        .with_payload(Arc::new(Mutex::new(resource_value)));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));

    Ok(resource::CryptoCipherHandle(resource_id))
}

/// Update one streaming cipher with additional data.
pub(crate) fn cipher_update_additional_data(
    binding: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    additional_data: &[u8],
) -> RuntimeResult<()> {
    // resolve and lock cipher resource
    let resource = resolve_cipher_resource(
        binding,
        handle,
        "destack.crypto.cipher.updateAdditionalData",
    )?;
    let mut resource = resource.lock();

    // feed additional authenticated data for the active stream state
    match &mut resource.state {
        CryptoCipherState::Software { crypter, .. } => crypter
            .aad_update(additional_data)
            .map_err(|error| openssl_error("destack.crypto.cipher.updateAdditionalData", error)),
        CryptoCipherState::HostSecret {
            additional_data: buffered_additional_data,
            ..
        } => {
            buffered_additional_data.extend_from_slice(additional_data);
            Ok(())
        }
    }
}

/// Update one streaming cipher with payload bytes.
pub(crate) fn cipher_update(
    binding: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    payload: &[u8],
) -> RuntimeResult<Vec<u8>> {
    // resolve and lock cipher resource
    let resource = resolve_cipher_resource(binding, handle, "destack.crypto.cipher.update")?;
    let mut resource = resource.lock();

    // cache algorithm lane before mutable state match
    let algorithm = resource.algorithm;

    // run update for the active stream state
    match &mut resource.state {
        CryptoCipherState::Software { key, crypter } => {
            let cipher = openssl_cipher(algorithm, key.len())?;
            let mut output = vec![0u8; payload.len() + cipher.block_size()];

            let written = crypter
                .update(payload, &mut output)
                .map_err(|error| openssl_error("destack.crypto.cipher.update", error))?;
            output.truncate(written);
            Ok(output)
        }
        CryptoCipherState::HostSecret {
            payload: buffered_payload,
            ..
        } => {
            buffered_payload.extend_from_slice(payload);
            Ok(Vec::new())
        }
    }
}

/// Finalize one software cipher stream.
fn cipher_finish_software(
    algorithm: CryptoCipherAlgorithm,
    direction: CryptoCipherDirection,
    tag_length_bytes: u32,
    key: &[u8],
    crypter: &mut Crypter,
    final_payload: &[u8],
) -> RuntimeResult<(Vec<u8>, Vec<u8>)> {
    // process final payload and finalize cryptographic state
    let cipher = openssl_cipher(algorithm, key.len())?;
    let mut output = vec![0u8; final_payload.len() + cipher.block_size()];
    let mut written = crypter
        .update(final_payload, &mut output)
        .map_err(|error| openssl_error("destack.crypto.cipher.finish", error))?;
    written += crypter
        .finalize(&mut output[written..])
        .map_err(|error| openssl_error("destack.crypto.cipher.finish", error))?;
    output.truncate(written);

    // emit authentication tag for AEAD encryption streams
    let tag = if direction == CryptoCipherDirection::Encrypt && is_aead_cipher(algorithm) {
        let tag_length =
            resolve_aead_tag_length(algorithm, tag_length_bytes, "parameters.tagLengthBytes")?;
        let mut tag = vec![0u8; tag_length];
        crypter
            .get_tag(&mut tag)
            .map_err(|error| openssl_error("destack.crypto.cipher.finish", error))?;
        tag
    } else {
        Vec::new()
    };

    Ok((output, tag))
}

/// Finalize one host-secret cipher stream.
fn cipher_finish_host_secret(
    binding: &BindingCallContext,
    algorithm: CryptoCipherAlgorithm,
    direction: CryptoCipherDirection,
    tag_length_bytes: u32,
    material: &HostKeyMaterial,
    store_kind: CryptoStoreKind,
    key_algorithm: CryptoKeyAlgorithm,
    nonce: &[u8],
    decrypt_tag: &[u8],
    additional_data: &[u8],
    buffered_payload: &mut Vec<u8>,
    final_payload: &[u8],
) -> RuntimeResult<(Vec<u8>, Vec<u8>)> {
    // append final payload bytes into the buffered stream payload
    buffered_payload.extend_from_slice(final_payload);

    // materialize runtime parameters from buffered host-stream state
    let parameters = CryptoCipherParameters {
        algorithm,
        nonce: binding.store_slice_copy(nonce),
        additional_data: binding.store_slice_copy(additional_data),
        tag: binding.store_slice_copy(decrypt_tag),
        tag_length_bytes: Some(tag_length_bytes),
    };

    // dispatch one-shot host cipher for buffered stream payload
    if direction == CryptoCipherDirection::Encrypt {
        return crypto_host::host_key_cipher_encrypt(
            binding,
            material,
            store_kind,
            key_algorithm,
            parameters,
            buffered_payload,
            "destack.crypto.cipher.finish",
        );
    }

    let output = crypto_host::host_key_cipher_decrypt(
        binding,
        material,
        store_kind,
        key_algorithm,
        parameters,
        buffered_payload,
        "destack.crypto.cipher.finish",
    )?;

    Ok((output, Vec::new()))
}

/// Finalize one streaming cipher context.
pub(crate) fn cipher_finish(
    binding: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    final_payload: &[u8],
) -> RuntimeResult<(Vec<u8>, Vec<u8>)> {
    // resolve and lock cipher resource
    let resource = resolve_cipher_resource(binding, handle, "destack.crypto.cipher.finish")?;
    let mut resource = resource.lock();

    // finalize the active stream state
    let algorithm = resource.algorithm;
    let direction = resource.direction;
    let tag_length_bytes = resource.tag_length_bytes;
    match &mut resource.state {
        CryptoCipherState::Software { key, crypter } => cipher_finish_software(
            algorithm,
            direction,
            tag_length_bytes,
            key,
            crypter,
            final_payload,
        ),
        CryptoCipherState::HostSecret {
            material,
            store_kind,
            key_algorithm,
            nonce,
            decrypt_tag,
            additional_data,
            payload,
        } => cipher_finish_host_secret(
            binding,
            algorithm,
            direction,
            tag_length_bytes,
            material,
            *store_kind,
            *key_algorithm,
            nonce,
            decrypt_tag,
            additional_data,
            payload,
            final_payload,
        ),
    }
}

/// Reset one streaming cipher context with new parameters.
pub(crate) fn cipher_reset(
    binding: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    parameters: CryptoCipherParameters,
) -> RuntimeResult<()> {
    // resolve and lock cipher resource
    let resource = resolve_cipher_resource(binding, handle, "destack.crypto.cipher.reset")?;
    let mut resource = resource.lock();

    // replace active algorithm parameters
    resource.algorithm = parameters.algorithm;
    resource.tag_length_bytes = tag_length_bytes(parameters);
    let direction = resource.direction;

    // rebuild stream state with reset parameters
    match &mut resource.state {
        CryptoCipherState::Software { key, crypter } => {
            *crypter =
                build_cipher_state(key, direction, parameters, "destack.crypto.cipher.reset")?;
        }
        CryptoCipherState::HostSecret {
            nonce,
            decrypt_tag,
            additional_data,
            payload,
            ..
        } => {
            let (next_nonce, next_additional_data, next_decrypt_tag) =
                prepare_host_cipher_stream_parameters(direction, parameters)?;
            *nonce = next_nonce;
            *decrypt_tag = next_decrypt_tag;
            *additional_data = next_additional_data;
            payload.clear();
        }
    }

    Ok(())
}

/// Close one streaming cipher context.
pub(crate) fn cipher_close(
    binding: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
) -> RuntimeResult<()> {
    // remove cipher resource entry
    let Some(entry) =
        binding
            .worker()
            .resources
            .remove(binding.world(), handle.0, Some(binding.engine()))
    else {
        return Err(core_platform::io_not_found(
            "destack.crypto.cipher.close",
            format!("unknown crypto cipher handle {}", handle.0.local_id),
        ));
    };

    // validate handle kind
    if entry.kind != CRYPTO_CIPHER_RESOURCE_KIND {
        return Err(core_platform::io_not_found(
            "destack.crypto.cipher.close",
            format!("unknown crypto cipher handle {}", handle.0.local_id),
        ));
    }

    // wipe sensitive stream state before releasing the final resource entry
    let payload = entry.payload_cloned::<Arc<Mutex<CryptoCipherResource>>>();
    if let Some(resource) = payload {
        let mut resource = resource.lock();
        match &mut resource.state {
            CryptoCipherState::Software { key, .. } => {
                key.zeroize();
            }
            CryptoCipherState::HostSecret {
                nonce,
                decrypt_tag,
                additional_data,
                payload,
                ..
            } => {
                nonce.zeroize();
                decrypt_tag.zeroize();
                additional_data.zeroize();
                payload.zeroize();
            }
        }
    }

    Ok(())
}

/// Return the expected nonce length for one cipher algorithm.
fn expected_nonce_length(algorithm: CryptoCipherAlgorithm) -> RuntimeResult<usize> {
    match algorithm {
        CryptoCipherAlgorithm::AesGcm => Ok(12),
        CryptoCipherAlgorithm::AesCtr => Ok(16),
        CryptoCipherAlgorithm::AesCbc => Ok(16),
        CryptoCipherAlgorithm::ChaCha20Poly1305 => Ok(12),
        CryptoCipherAlgorithm::Unknown => Err(core_platform::invalid_argument(
            "parameters.algorithm",
            "algorithm must not be Unknown",
        )),
    }
}

/// Decode and validate one host cipher stream parameter set.
fn prepare_host_cipher_stream_parameters(
    direction: CryptoCipherDirection,
    parameters: CryptoCipherParameters,
) -> RuntimeResult<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    // decode byte arguments from native handles
    let nonce = decode_native_bytes(parameters.nonce, "parameters.nonce")?;
    let additional_data =
        decode_native_bytes(parameters.additional_data, "parameters.additionalData")?;
    let decrypt_tag = decode_native_bytes(parameters.tag, "parameters.tag")?;

    // enforce nonce length shape for this algorithm lane
    let expected_nonce_length = expected_nonce_length(parameters.algorithm)?;
    if nonce.len() != expected_nonce_length {
        return Err(core_platform::invalid_argument(
            "parameters.nonce",
            format!("nonce length must be {expected_nonce_length} bytes"),
        ));
    }

    // enforce tag shape for aead and non-aead lanes
    if is_aead_cipher(parameters.algorithm) {
        let expected_tag_length = resolve_aead_tag_length(
            parameters.algorithm,
            tag_length_bytes(parameters),
            "parameters.tagLengthBytes",
        )?;
        if direction == CryptoCipherDirection::Decrypt && decrypt_tag.len() != expected_tag_length {
            return Err(core_platform::invalid_argument(
                "parameters.tag",
                format!("tag length must be {expected_tag_length} bytes"),
            ));
        }
        if direction == CryptoCipherDirection::Encrypt && !decrypt_tag.is_empty() {
            return Err(core_platform::invalid_argument(
                "parameters.tag",
                "tag must be empty when encrypting",
            ));
        }
    } else if !decrypt_tag.is_empty() {
        return Err(core_platform::invalid_argument(
            "parameters.tag",
            "tag is only valid for AEAD algorithms",
        ));
    }

    Ok((nonce, additional_data, decrypt_tag))
}

/// Return one openssl cipher object for one cipher algorithm and key length.
pub(super) fn openssl_cipher(
    algorithm: CryptoCipherAlgorithm,
    key_len: usize,
) -> RuntimeResult<Cipher> {
    // resolve openssl cipher by algorithm and key length
    let cipher = match algorithm {
        CryptoCipherAlgorithm::AesGcm => match key_len {
            16 => Some(Cipher::aes_128_gcm()),
            24 => Some(Cipher::aes_192_gcm()),
            32 => Some(Cipher::aes_256_gcm()),
            _ => None,
        },
        CryptoCipherAlgorithm::AesCtr => match key_len {
            16 => Some(Cipher::aes_128_ctr()),
            24 => Some(Cipher::aes_192_ctr()),
            32 => Some(Cipher::aes_256_ctr()),
            _ => None,
        },
        CryptoCipherAlgorithm::AesCbc => match key_len {
            16 => Some(Cipher::aes_128_cbc()),
            24 => Some(Cipher::aes_192_cbc()),
            32 => Some(Cipher::aes_256_cbc()),
            _ => None,
        },
        CryptoCipherAlgorithm::ChaCha20Poly1305 => {
            if key_len == 32 {
                Some(Cipher::chacha20_poly1305())
            } else {
                None
            }
        }
        CryptoCipherAlgorithm::Unknown => None,
    };

    cipher.ok_or_else(|| {
        core_platform::invalid_argument("algorithm", "unsupported algorithm for key length")
    })
}

/// Return whether one cipher algorithm is AEAD.
pub(super) fn is_aead_cipher(algorithm: CryptoCipherAlgorithm) -> bool {
    matches!(
        algorithm,
        CryptoCipherAlgorithm::AesGcm | CryptoCipherAlgorithm::ChaCha20Poly1305
    )
}

/// Resolve the effective AEAD tag length for one operation.
pub(super) fn resolve_aead_tag_length(
    algorithm: CryptoCipherAlgorithm,
    tag_length_bytes: u32,
    field: &str,
) -> RuntimeResult<usize> {
    // apply runtime default tag length when caller omits explicit value
    let tag_length = if tag_length_bytes == 0 {
        DEFAULT_AEAD_TAG_LENGTH_BYTES
    } else {
        tag_length_bytes as usize
    };

    // enforce chacha20 poly1305 fixed tag size
    if algorithm == CryptoCipherAlgorithm::ChaCha20Poly1305 && tag_length != 16 {
        return Err(core_platform::invalid_argument(
            field,
            "tag length must be 16 bytes for chacha20poly1305",
        ));
    }

    // enforce aes gcm allowed tag range
    if algorithm == CryptoCipherAlgorithm::AesGcm && !(12..=16).contains(&tag_length) {
        return Err(core_platform::invalid_argument(
            field,
            "tag length must be between 12 and 16 bytes for aes-gcm",
        ));
    }

    Ok(tag_length)
}

/// Build one streaming cipher state.
pub(super) fn build_cipher_state(
    key: &[u8],
    direction: CryptoCipherDirection,
    parameters: CryptoCipherParameters,
    operation: &'static str,
) -> RuntimeResult<Crypter> {
    // resolve cipher implementation and decode nonce
    let cipher = openssl_cipher(parameters.algorithm, key.len())?;
    let nonce = decode_native_bytes(parameters.nonce, "parameters.nonce")?;

    // enforce nonce length expected by selected cipher
    let expected_nonce_length = cipher.iv_len().ok_or_else(|| {
        core_platform::invalid_argument(
            "parameters.nonce",
            "cipher does not expose one nonce/iv length",
        )
    })?;
    if nonce.len() != expected_nonce_length {
        return Err(core_platform::invalid_argument(
            "parameters.nonce",
            format!("nonce length must be {expected_nonce_length} bytes"),
        ));
    }

    // build openssl crypter state
    let mode = match direction {
        CryptoCipherDirection::Encrypt => Mode::Encrypt,
        CryptoCipherDirection::Decrypt => Mode::Decrypt,
    };
    let mut crypter = Crypter::new(cipher, mode, key, Some(&nonce))
        .map_err(|error| openssl_error(operation, error))?;

    // configure aead tag behavior
    if is_aead_cipher(parameters.algorithm) {
        if direction == CryptoCipherDirection::Encrypt {
            resolve_aead_tag_length(
                parameters.algorithm,
                tag_length_bytes(parameters),
                "parameters.tagLengthBytes",
            )?;
        } else {
            let tag = decode_native_bytes(parameters.tag, "parameters.tag")?;
            if tag.is_empty() {
                return Err(core_platform::invalid_argument(
                    "parameters.tag",
                    "authentication tag is required for decrypt",
                ));
            }
            let tag_length_bytes = tag_length_bytes(parameters);
            if tag_length_bytes != 0 && tag_length_bytes as usize != tag.len() {
                return Err(core_platform::invalid_argument(
                    "parameters.tagLengthBytes",
                    "tagLengthBytes must match decrypt tag length",
                ));
            }
            crypter
                .set_tag(&tag)
                .map_err(|error| openssl_error(operation, error))?;
        }
    }

    // apply additional authenticated data when present
    let additional_data =
        decode_native_bytes(parameters.additional_data, "parameters.additionalData")?;
    if !additional_data.is_empty() {
        crypter
            .aad_update(&additional_data)
            .map_err(|error| openssl_error(operation, error))?;
    }

    Ok(crypter)
}

/// Encrypt or decrypt one payload in one shot.
pub(super) fn cipher_process(
    key: &[u8],
    direction: CryptoCipherDirection,
    parameters: CryptoCipherParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<(Vec<u8>, Vec<u8>)> {
    // initialize one-shot crypter state
    let mut crypter = build_cipher_state(key, direction, parameters, operation)?;
    let cipher = openssl_cipher(parameters.algorithm, key.len())?;

    // process payload and finalize output bytes
    let mut output = vec![0u8; payload.len() + cipher.block_size()];
    let mut written = crypter
        .update(payload, &mut output)
        .map_err(|error| openssl_error(operation, error))?;
    written += crypter
        .finalize(&mut output[written..])
        .map_err(|error| openssl_error(operation, error))?;
    output.truncate(written);

    // emit authentication tag for aead encrypt operations
    let tag = if direction == CryptoCipherDirection::Encrypt && is_aead_cipher(parameters.algorithm)
    {
        let tag_length = resolve_aead_tag_length(
            parameters.algorithm,
            tag_length_bytes(parameters),
            "parameters.tagLengthBytes",
        )?;
        let mut tag = vec![0u8; tag_length];
        crypter
            .get_tag(&mut tag)
            .map_err(|error| openssl_error(operation, error))?;
        tag
    } else {
        Vec::new()
    };

    Ok((output, tag))
}
