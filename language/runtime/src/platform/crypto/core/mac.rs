use std::sync::Arc;

use openssl::hash::{Hasher, hash};
use openssl::memcmp;
use parking_lot::Mutex;
use zeroize::{Zeroize, Zeroizing};

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::{CryptoDigestAlgorithm, CryptoMacAlgorithm, CryptoMacParameters};
use crate::platform::resource;
use crate::platform::resource::ResourceEntry;
use crate::runtime::BindingCallContext;

use super::core::{
    CRYPTO_MAC_LABEL, CRYPTO_MAC_RESOURCE_KIND, CryptoMacResource, KEY_USAGE_SIGN,
    KEY_USAGE_VERIFY, handle_not_found, invalid_argument, openssl_error, resolve_mac_resource,
};
use super::digest::message_digest;
use super::key::{require_key_usage, resolve_secret_key_bytes};

/// Compute one mac in one shot.
fn mac_compute_internal(
    context: &BindingCallContext,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
    payload: &[u8],
) -> RuntimeResult<Vec<u8>> {
    // validate mac algorithm lane
    if parameters.algorithm != CryptoMacAlgorithm::Hmac {
        return Err(invalid_argument(
            "parameters.algorithm",
            "only HMAC is currently supported",
        ));
    }

    // resolve secret key bytes and keep them zeroized on all paths
    let key = resolve_secret_key_bytes(context, key, "destack.crypto.mac.compute")?;
    let key = Zeroizing::new(key);

    // compute hmac output
    let mut output = hmac_compute(
        parameters.digest,
        &key,
        payload,
        "destack.crypto.mac.compute",
    )?;

    // apply optional output truncation
    if parameters.tag_length_bytes != 0 {
        let length = parameters.tag_length_bytes as usize;
        if length > output.len() {
            return Err(invalid_argument(
                "parameters.tagLengthBytes",
                "tag length must be at most digest size",
            ));
        }
        output.truncate(length);
    }

    Ok(output)
}

/// Compute one mac in one shot.
pub(crate) fn mac_compute(
    context: &BindingCallContext,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
    payload: &[u8],
) -> RuntimeResult<Vec<u8>> {
    // enforce key usage policy
    require_key_usage(context, key, KEY_USAGE_SIGN, "destack.crypto.mac.compute")?;

    // compute one-shot mac output
    mac_compute_internal(context, key, parameters, payload)
}

/// Verify one mac in one shot.
pub(crate) fn mac_verify(
    context: &BindingCallContext,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
    payload: &[u8],
    tag: &[u8],
) -> RuntimeResult<bool> {
    // enforce key usage policy
    require_key_usage(context, key, KEY_USAGE_VERIFY, "destack.crypto.mac.verify")?;

    // compute expected tag for payload
    let computed = mac_compute_internal(context, key, parameters, payload)?;

    // compare tags in constant time
    Ok(memcmp::eq(&computed, tag))
}

/// Open one streaming mac context.
pub(crate) fn mac_open(
    context: &BindingCallContext,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
) -> RuntimeResult<resource::CryptoMacHandle> {
    // enforce key usage policy
    require_key_usage(context, key, KEY_USAGE_SIGN, "destack.crypto.mac.open")?;

    // validate mac algorithm lane
    if parameters.algorithm != CryptoMacAlgorithm::Hmac {
        return Err(invalid_argument(
            "parameters.algorithm",
            "only HMAC is currently supported",
        ));
    }

    // resolve secret key bytes and keep them zeroized on all paths
    let key = resolve_secret_key_bytes(context, key, "destack.crypto.mac.open")?;
    let key = Zeroizing::new(key);

    // initialize incremental hmac state
    let (inner_key, outer_key, hasher) =
        hmac_state_open(parameters.digest, &key, "destack.crypto.mac.open")?;

    // publish mac resource
    let resource_value = CryptoMacResource {
        parameters,
        inner_key,
        outer_key,
        hasher,
    };
    let entry = ResourceEntry::new(CRYPTO_MAC_RESOURCE_KIND)
        .with_label(CRYPTO_MAC_LABEL)
        .with_payload(Arc::new(Mutex::new(resource_value)));
    let resource_id = context.runtime().resources.insert(entry);

    Ok(resource::CryptoMacHandle(resource_id))
}

/// Update one streaming mac context.
pub(crate) fn mac_update(
    context: &BindingCallContext,
    handle: resource::CryptoMacHandle,
    payload: &[u8],
) -> RuntimeResult<()> {
    // resolve and lock mac resource
    let resource = resolve_mac_resource(context, handle, "destack.crypto.mac.update")?;
    let mut resource = resource.lock();

    // feed payload bytes into streaming hmac state
    resource
        .hasher
        .update(payload)
        .map_err(|error| openssl_error("destack.crypto.mac.update", error))?;

    Ok(())
}

/// Finalize one streaming mac context.
pub(crate) fn mac_finish(
    context: &BindingCallContext,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<Vec<u8>> {
    // resolve and lock mac resource
    let resource = resolve_mac_resource(context, handle, "destack.crypto.mac.finish")?;
    let mut resource = resource.lock();

    // swap active hasher out to avoid partial state loss on failure
    let digest_algorithm = resource.parameters.digest;
    let mut hasher = Hasher::new(message_digest(digest_algorithm)?)
        .map_err(|error| openssl_error("destack.crypto.mac.finish", error))?;
    std::mem::swap(&mut hasher, &mut resource.hasher);
    let mut output = hmac_state_finish(
        digest_algorithm,
        &resource.inner_key,
        &resource.outer_key,
        &mut hasher,
        "destack.crypto.mac.finish",
    )?;
    resource.hasher = hasher;

    // apply optional output truncation
    if resource.parameters.tag_length_bytes != 0 {
        let length = resource.parameters.tag_length_bytes as usize;
        if length > output.len() {
            return Err(invalid_argument(
                "parameters.tagLengthBytes",
                "tag length must be at most digest size",
            ));
        }
        output.truncate(length);
    }
    Ok(output)
}

/// Reset one streaming mac context.
pub(crate) fn mac_reset(
    context: &BindingCallContext,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    // resolve and lock mac resource
    let resource = resolve_mac_resource(context, handle, "destack.crypto.mac.reset")?;
    let mut resource = resource.lock();

    // rebuild hasher and preload ipad block
    let mut hasher = Hasher::new(message_digest(resource.parameters.digest)?)
        .map_err(|error| openssl_error("destack.crypto.mac.reset", error))?;
    hasher
        .update(&resource.inner_key)
        .map_err(|error| openssl_error("destack.crypto.mac.reset", error))?;
    resource.hasher = hasher;

    Ok(())
}

/// Close one streaming mac context.
pub(crate) fn mac_close(
    context: &BindingCallContext,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    // remove mac resource entry
    let Some(mut entry) = context.runtime().resources.remove(handle.0) else {
        return Err(handle_not_found(
            "destack.crypto.mac.close",
            "crypto mac",
            handle.0.0,
        ));
    };

    // validate handle kind
    if entry.kind != CRYPTO_MAC_RESOURCE_KIND {
        return Err(handle_not_found(
            "destack.crypto.mac.close",
            "crypto mac",
            handle.0.0,
        ));
    }

    // wipe hmac key pads eagerly before resource drop
    if let Some(payload) = entry.payload.as_mut()
        && let Some(mac_resource) = payload.downcast_mut::<Arc<Mutex<CryptoMacResource>>>()
    {
        let mut mac_resource = mac_resource.lock();
        mac_resource.inner_key.zeroize();
        mac_resource.outer_key.zeroize();
    }

    Ok(())
}

/// Compute one HMAC.
pub(super) fn hmac_compute(
    algorithm: CryptoDigestAlgorithm,
    key: &[u8],
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // resolve digest implementation and key schedule dimensions
    let message_digest = message_digest(algorithm)?;
    let block_size = message_digest.block_size();
    let digest_size = message_digest.size();

    // normalize key to digest block size
    let normalized_key = if key.len() > block_size {
        let digest = hash(message_digest, key).map_err(|error| openssl_error(operation, error))?;
        digest.to_vec()
    } else {
        key.to_vec()
    };
    let mut normalized_key = Zeroizing::new(normalized_key);
    if normalized_key.len() < block_size {
        normalized_key.resize(block_size, 0);
    }

    // derive hmac inner and outer pads
    let mut inner_key = Zeroizing::new(vec![0x36u8; block_size]);
    let mut outer_key = Zeroizing::new(vec![0x5cu8; block_size]);
    for index in 0..block_size {
        inner_key[index] ^= normalized_key[index];
        outer_key[index] ^= normalized_key[index];
    }

    // hash inner payload
    let mut inner_payload = Zeroizing::new(Vec::with_capacity(block_size + payload.len()));
    inner_payload.extend_from_slice(&inner_key);
    inner_payload.extend_from_slice(payload);
    let inner_digest =
        hash(message_digest, &inner_payload).map_err(|error| openssl_error(operation, error))?;

    // hash outer payload
    let mut outer_payload = Zeroizing::new(Vec::with_capacity(block_size + digest_size));
    outer_payload.extend_from_slice(&outer_key);
    outer_payload.extend_from_slice(inner_digest.as_ref());
    let outer_digest =
        hash(message_digest, &outer_payload).map_err(|error| openssl_error(operation, error))?;

    Ok(outer_digest.to_vec())
}

/// Initialize one incremental HMAC state.
pub(super) fn hmac_state_open(
    algorithm: CryptoDigestAlgorithm,
    key: &[u8],
    operation: &'static str,
) -> RuntimeResult<(Vec<u8>, Vec<u8>, Hasher)> {
    // resolve digest implementation and key schedule dimensions
    let message_digest = message_digest(algorithm)?;
    let block_size = message_digest.block_size();

    // normalize key to digest block size
    let normalized_key = if key.len() > block_size {
        let digest = hash(message_digest, key).map_err(|error| openssl_error(operation, error))?;
        digest.to_vec()
    } else {
        key.to_vec()
    };
    let mut normalized_key = Zeroizing::new(normalized_key);
    if normalized_key.len() < block_size {
        normalized_key.resize(block_size, 0);
    }

    // derive hmac inner and outer pads
    let mut inner_key = vec![0x36u8; block_size];
    let mut outer_key = vec![0x5cu8; block_size];
    for index in 0..block_size {
        inner_key[index] ^= normalized_key[index];
        outer_key[index] ^= normalized_key[index];
    }

    // initialize hasher with ipad prefix
    let mut hasher =
        Hasher::new(message_digest).map_err(|error| openssl_error(operation, error))?;
    hasher
        .update(&inner_key)
        .map_err(|error| openssl_error(operation, error))?;

    Ok((inner_key, outer_key, hasher))
}

/// Finalize one incremental HMAC state.
pub(super) fn hmac_state_finish(
    algorithm: CryptoDigestAlgorithm,
    inner_key: &[u8],
    outer_key: &[u8],
    hasher: &mut Hasher,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // resolve digest implementation for outer and next states
    let message_digest = message_digest(algorithm)?;

    // finalize inner digest
    let inner_digest = hasher
        .finish()
        .map_err(|error| openssl_error(operation, error))?;

    // compute outer hmac digest
    let mut outer_hasher =
        Hasher::new(message_digest).map_err(|error| openssl_error(operation, error))?;
    outer_hasher
        .update(outer_key)
        .map_err(|error| openssl_error(operation, error))?;
    outer_hasher
        .update(inner_digest.as_ref())
        .map_err(|error| openssl_error(operation, error))?;
    let digest = outer_hasher
        .finish()
        .map_err(|error| openssl_error(operation, error))?;

    // reinitialize hasher for next update cycle
    let mut next_hasher =
        Hasher::new(message_digest).map_err(|error| openssl_error(operation, error))?;
    next_hasher
        .update(inner_key)
        .map_err(|error| openssl_error(operation, error))?;
    *hasher = next_hasher;

    Ok(digest.to_vec())
}
