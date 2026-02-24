use std::sync::Arc;

use openssl::symm::{Cipher, Crypter, Mode};
use parking_lot::Mutex;
use zeroize::{Zeroize, Zeroizing};

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::{
    CryptoCipherAlgorithm, CryptoCipherDirection, CryptoCipherParameters,
};
use crate::platform::resource;
use crate::platform::resource::ResourceEntry;
use crate::runtime::BindingCallContext;

use super::constants::DEFAULT_AEAD_TAG_LENGTH_BYTES;
use super::core::{
    CRYPTO_CIPHER_LABEL, CRYPTO_CIPHER_RESOURCE_KIND, CryptoCipherResource, KEY_USAGE_DECRYPT,
    KEY_USAGE_ENCRYPT, decode_native_bytes, handle_not_found, invalid_argument, openssl_error,
    resolve_cipher_resource,
};
use super::key::{require_key_usage, resolve_secret_key_bytes};

/// Encrypt one payload in one shot.
pub(crate) fn cipher_encrypt(
    context: &BindingCallContext,
    key: resource::CryptoKeyHandle,
    parameters: CryptoCipherParameters,
    payload: &[u8],
) -> RuntimeResult<(Vec<u8>, Vec<u8>)> {
    // enforce key usage policy
    require_key_usage(
        context,
        key,
        KEY_USAGE_ENCRYPT,
        "destack.crypto.cipher.encrypt",
    )?;

    // resolve secret key bytes and keep them zeroized on all paths
    let key_bytes = resolve_secret_key_bytes(context, key, "destack.crypto.cipher.encrypt")?;
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
    context: &BindingCallContext,
    key: resource::CryptoKeyHandle,
    parameters: CryptoCipherParameters,
    payload: &[u8],
) -> RuntimeResult<Vec<u8>> {
    // enforce key usage policy
    require_key_usage(
        context,
        key,
        KEY_USAGE_DECRYPT,
        "destack.crypto.cipher.decrypt",
    )?;

    // resolve secret key bytes and keep them zeroized on all paths
    let key_bytes = resolve_secret_key_bytes(context, key, "destack.crypto.cipher.decrypt")?;
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
    context: &BindingCallContext,
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
    require_key_usage(context, key, required_usage, "destack.crypto.cipher.open")?;

    // resolve key bytes and initialize streaming cipher state
    let key = resolve_secret_key_bytes(context, key, "destack.crypto.cipher.open")?;
    let crypter = build_cipher_state(&key, direction, parameters, "destack.crypto.cipher.open")?;

    // publish cipher resource
    let resource_value = CryptoCipherResource {
        algorithm: parameters.algorithm,
        direction,
        key,
        tag_length_bytes: parameters.tag_length_bytes,
        crypter,
    };
    let entry = ResourceEntry::new(CRYPTO_CIPHER_RESOURCE_KIND)
        .with_label(CRYPTO_CIPHER_LABEL)
        .with_payload(Arc::new(Mutex::new(resource_value)));
    let resource_id = context.runtime().resources.insert(entry);

    Ok(resource::CryptoCipherHandle(resource_id))
}

/// Update one streaming cipher with additional data.
pub(crate) fn cipher_update_additional_data(
    context: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    additional_data: &[u8],
) -> RuntimeResult<()> {
    // resolve and lock cipher resource
    let resource = resolve_cipher_resource(
        context,
        handle,
        "destack.crypto.cipher.updateAdditionalData",
    )?;
    let mut resource = resource.lock();

    // feed additional authenticated data
    resource
        .crypter
        .aad_update(additional_data)
        .map_err(|error| openssl_error("destack.crypto.cipher.updateAdditionalData", error))
}

/// Update one streaming cipher with payload bytes.
pub(crate) fn cipher_update(
    context: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    payload: &[u8],
) -> RuntimeResult<Vec<u8>> {
    // resolve and lock cipher resource
    let resource = resolve_cipher_resource(context, handle, "destack.crypto.cipher.update")?;
    let mut resource = resource.lock();

    // allocate output buffer for this update call
    let cipher = openssl_cipher(resource.algorithm, resource.key.len())?;
    let mut output = vec![0u8; payload.len() + cipher.block_size()];

    // process payload bytes
    let written = resource
        .crypter
        .update(payload, &mut output)
        .map_err(|error| openssl_error("destack.crypto.cipher.update", error))?;
    output.truncate(written);

    Ok(output)
}

/// Finalize one streaming cipher context.
pub(crate) fn cipher_finish(
    context: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    final_payload: &[u8],
) -> RuntimeResult<(Vec<u8>, Vec<u8>)> {
    // resolve and lock cipher resource
    let resource = resolve_cipher_resource(context, handle, "destack.crypto.cipher.finish")?;
    let mut resource = resource.lock();

    // process final payload and finalize cryptographic state
    let cipher = openssl_cipher(resource.algorithm, resource.key.len())?;
    let mut output = vec![0u8; final_payload.len() + cipher.block_size()];
    let mut written = resource
        .crypter
        .update(final_payload, &mut output)
        .map_err(|error| openssl_error("destack.crypto.cipher.finish", error))?;
    written += resource
        .crypter
        .finalize(&mut output[written..])
        .map_err(|error| openssl_error("destack.crypto.cipher.finish", error))?;
    output.truncate(written);

    // emit authentication tag for AEAD encryption streams
    let tag = if resource.direction == CryptoCipherDirection::Encrypt
        && is_aead_cipher(resource.algorithm)
    {
        let tag_length = resolve_aead_tag_length(
            resource.algorithm,
            resource.tag_length_bytes,
            "parameters.tagLengthBytes",
        )?;
        let mut tag = vec![0u8; tag_length];
        resource
            .crypter
            .get_tag(&mut tag)
            .map_err(|error| openssl_error("destack.crypto.cipher.finish", error))?;
        tag
    } else {
        Vec::new()
    };

    Ok((output, tag))
}

/// Reset one streaming cipher context with new parameters.
pub(crate) fn cipher_reset(
    context: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    parameters: CryptoCipherParameters,
) -> RuntimeResult<()> {
    // resolve and lock cipher resource
    let resource = resolve_cipher_resource(context, handle, "destack.crypto.cipher.reset")?;
    let mut resource = resource.lock();

    // replace active algorithm parameters
    resource.algorithm = parameters.algorithm;
    resource.tag_length_bytes = parameters.tag_length_bytes;

    // rebuild cipher state with existing key and direction
    resource.crypter = build_cipher_state(
        &resource.key,
        resource.direction,
        parameters,
        "destack.crypto.cipher.reset",
    )?;

    Ok(())
}

/// Close one streaming cipher context.
pub(crate) fn cipher_close(
    context: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
) -> RuntimeResult<()> {
    // remove cipher resource entry
    let Some(mut entry) = context.runtime().resources.remove(handle.0) else {
        return Err(handle_not_found(
            "destack.crypto.cipher.close",
            "crypto cipher",
            handle.0.0,
        ));
    };

    // validate handle kind
    if entry.kind != CRYPTO_CIPHER_RESOURCE_KIND {
        return Err(handle_not_found(
            "destack.crypto.cipher.close",
            "crypto cipher",
            handle.0.0,
        ));
    }

    // wipe key bytes eagerly before resource drop
    if let Some(payload) = entry.payload.as_mut()
        && let Some(cipher_resource) = payload.downcast_mut::<Arc<Mutex<CryptoCipherResource>>>()
    {
        let mut cipher_resource = cipher_resource.lock();
        cipher_resource.key.zeroize();
    }

    Ok(())
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

    cipher.ok_or_else(|| invalid_argument("algorithm", "unsupported algorithm for key length"))
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
        return Err(invalid_argument(
            field,
            "tag length must be 16 bytes for chacha20poly1305",
        ));
    }

    // enforce aes gcm allowed tag range
    if algorithm == CryptoCipherAlgorithm::AesGcm && !(12..=16).contains(&tag_length) {
        return Err(invalid_argument(
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
        invalid_argument(
            "parameters.nonce",
            "cipher does not expose one nonce/iv length",
        )
    })?;
    if nonce.len() != expected_nonce_length {
        return Err(invalid_argument(
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
                parameters.tag_length_bytes,
                "parameters.tagLengthBytes",
            )?;
        } else {
            let tag = decode_native_bytes(parameters.tag, "parameters.tag")?;
            if tag.is_empty() {
                return Err(invalid_argument(
                    "parameters.tag",
                    "authentication tag is required for decrypt",
                ));
            }
            if parameters.tag_length_bytes != 0 && parameters.tag_length_bytes as usize != tag.len()
            {
                return Err(invalid_argument(
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
            parameters.tag_length_bytes,
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
