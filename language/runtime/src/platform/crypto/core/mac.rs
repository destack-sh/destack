use std::sync::Arc;

use openssl::hash::{Hasher, hash};
use openssl::memcmp;
use parking_lot::Mutex;
use zeroize::{Zeroize, Zeroizing};

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::{
    CryptoDigestAlgorithm, CryptoMacAlgorithm, CryptoMacParameters, host as crypto_host,
};
use crate::platform::resource::ResourceEntry;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core::{
    CRYPTO_MAC_LABEL, CRYPTO_MAC_RESOURCE_KIND, CryptoMacResource, CryptoMacState, KEY_USAGE_SIGN,
    KEY_USAGE_VERIFY, openssl_error, resolve_mac_resource,
};
use super::digest::message_digest;
use super::key::{require_key_usage, resolve_host_secret_key_material, resolve_secret_key_bytes};

/// Return the effective mac tag length in bytes.
fn tag_length_bytes(parameters: CryptoMacParameters) -> u32 {
    parameters.tag_length_bytes.unwrap_or(0)
}

/// Compute one mac in one shot.
fn mac_compute_internal(
    binding: &BindingCallContext,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
    payload: &[u8],
) -> RuntimeResult<Vec<u8>> {
    // validate mac algorithm lane
    if parameters.algorithm != CryptoMacAlgorithm::Hmac {
        return Err(core_platform::invalid_argument(
            "parameters.algorithm",
            "only HMAC is currently supported",
        ));
    }

    // route host-managed secret-key lanes through host mac primitives
    if let Some((host_key, store_kind, key_algorithm)) =
        resolve_host_secret_key_material(binding, key, "destack.crypto.mac.compute")?
    {
        return crypto_host::host_key_mac_compute(
            binding,
            &host_key,
            store_kind,
            key_algorithm,
            parameters,
            payload,
            "destack.crypto.mac.compute",
        );
    }

    // resolve secret key bytes and keep them zeroized on all paths
    let key = resolve_secret_key_bytes(binding, key, "destack.crypto.mac.compute")?;
    let key = Zeroizing::new(key);

    // compute hmac output
    let mut output = hmac_compute(
        parameters.digest,
        &key,
        payload,
        "destack.crypto.mac.compute",
    )?;

    // apply optional output truncation
    let tag_length_bytes = tag_length_bytes(parameters);
    if tag_length_bytes != 0 {
        let length = tag_length_bytes as usize;
        if length > output.len() {
            return Err(core_platform::invalid_argument(
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
    binding: &BindingCallContext,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
    payload: &[u8],
) -> RuntimeResult<Vec<u8>> {
    // enforce key usage policy
    require_key_usage(binding, key, KEY_USAGE_SIGN, "destack.crypto.mac.compute")?;

    // compute one-shot mac output
    mac_compute_internal(binding, key, parameters, payload)
}

/// Verify one mac in one shot.
pub(crate) fn mac_verify(
    binding: &BindingCallContext,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
    payload: &[u8],
    tag: &[u8],
) -> RuntimeResult<bool> {
    // enforce key usage policy
    require_key_usage(binding, key, KEY_USAGE_VERIFY, "destack.crypto.mac.verify")?;

    // compute expected tag for payload
    let computed = mac_compute_internal(binding, key, parameters, payload)?;

    // compare tags in constant time
    Ok(memcmp::eq(&computed, tag))
}

/// Open one streaming mac context.
pub(crate) fn mac_open(
    binding: &BindingCallContext,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
) -> RuntimeResult<resource::CryptoMacHandle> {
    // enforce key usage policy
    require_key_usage(binding, key, KEY_USAGE_SIGN, "destack.crypto.mac.open")?;

    // validate mac algorithm lane
    if parameters.algorithm != CryptoMacAlgorithm::Hmac {
        return Err(core_platform::invalid_argument(
            "parameters.algorithm",
            "only HMAC is currently supported",
        ));
    }

    // build host-secret stream state when this key is host managed
    if let Some((material, store_kind, key_algorithm)) =
        resolve_host_secret_key_material(binding, key, "destack.crypto.mac.open")?
    {
        let resource_value = CryptoMacResource {
            parameters,
            state: CryptoMacState::HostSecret {
                material,
                store_kind,
                key_algorithm,
                payload: Vec::new(),
            },
        };
        let entry = ResourceEntry::new(CRYPTO_MAC_RESOURCE_KIND)
            .with_label(CRYPTO_MAC_LABEL)
            .with_payload(Arc::new(Mutex::new(resource_value)));
        let resource_id =
            binding
                .worker()
                .resources
                .insert(&binding.world(), entry, Some(binding.engine()));

        return Ok(resource::CryptoMacHandle(resource_id));
    }

    // resolve secret key bytes and keep them zeroized on all paths
    let key = resolve_secret_key_bytes(binding, key, "destack.crypto.mac.open")?;
    let key = Zeroizing::new(key);

    // initialize incremental hmac state
    let (inner_key, outer_key, hasher) =
        hmac_state_open(parameters.digest, &key, "destack.crypto.mac.open")?;

    // publish mac resource
    let resource_value = CryptoMacResource {
        parameters,
        state: CryptoMacState::Software {
            inner_key,
            outer_key,
            hasher,
        },
    };
    let entry = ResourceEntry::new(CRYPTO_MAC_RESOURCE_KIND)
        .with_label(CRYPTO_MAC_LABEL)
        .with_payload(Arc::new(Mutex::new(resource_value)));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    Ok(resource::CryptoMacHandle(resource_id))
}

/// Update one streaming mac context.
pub(crate) fn mac_update(
    binding: &BindingCallContext,
    handle: resource::CryptoMacHandle,
    payload: &[u8],
) -> RuntimeResult<()> {
    // resolve and lock mac resource
    let resource = resolve_mac_resource(binding, handle, "destack.crypto.mac.update")?;
    let mut resource = resource.lock();

    // feed payload bytes for the active stream state
    match &mut resource.state {
        CryptoMacState::Software { hasher, .. } => hasher
            .update(payload)
            .map_err(|error| openssl_error("destack.crypto.mac.update", error)),
        CryptoMacState::HostSecret {
            payload: buffered_payload,
            ..
        } => {
            buffered_payload.extend_from_slice(payload);
            Ok(())
        }
    }
}

/// Finalize one streaming mac context.
pub(crate) fn mac_finish(
    binding: &BindingCallContext,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<Vec<u8>> {
    // resolve and lock mac resource
    let resource = resolve_mac_resource(binding, handle, "destack.crypto.mac.finish")?;
    let mut resource = resource.lock();

    // cache parameter lanes before mutable state match
    let parameters = resource.parameters;

    // finalize the active stream state
    match &mut resource.state {
        CryptoMacState::Software {
            inner_key,
            outer_key,
            hasher,
        } => {
            let digest_algorithm = parameters.digest;
            let mut next_hasher = Hasher::new(message_digest(digest_algorithm)?)
                .map_err(|error| openssl_error("destack.crypto.mac.finish", error))?;
            std::mem::swap(&mut next_hasher, hasher);
            let mut output = hmac_state_finish(
                digest_algorithm,
                inner_key,
                outer_key,
                &mut next_hasher,
                "destack.crypto.mac.finish",
            )?;
            *hasher = next_hasher;

            let tag_length_bytes = tag_length_bytes(parameters);
            if tag_length_bytes != 0 {
                let length = tag_length_bytes as usize;
                if length > output.len() {
                    return Err(core_platform::invalid_argument(
                        "parameters.tagLengthBytes",
                        "tag length must be at most digest size",
                    ));
                }
                output.truncate(length);
            }

            Ok(output)
        }
        CryptoMacState::HostSecret {
            material,
            store_kind,
            key_algorithm,
            payload,
        } => crypto_host::host_key_mac_compute(
            binding,
            material,
            *store_kind,
            *key_algorithm,
            parameters,
            payload,
            "destack.crypto.mac.finish",
        ),
    }
}

/// Reset one streaming mac context.
pub(crate) fn mac_reset(
    binding: &BindingCallContext,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    // resolve and lock mac resource
    let resource = resolve_mac_resource(binding, handle, "destack.crypto.mac.reset")?;
    let mut resource = resource.lock();

    // cache parameter lanes before mutable state match
    let digest_algorithm = resource.parameters.digest;

    // reset the active stream state
    match &mut resource.state {
        CryptoMacState::Software {
            inner_key, hasher, ..
        } => {
            let mut next_hasher = Hasher::new(message_digest(digest_algorithm)?)
                .map_err(|error| openssl_error("destack.crypto.mac.reset", error))?;
            next_hasher
                .update(inner_key)
                .map_err(|error| openssl_error("destack.crypto.mac.reset", error))?;
            *hasher = next_hasher;
        }
        CryptoMacState::HostSecret {
            payload: buffered_payload,
            ..
        } => {
            buffered_payload.clear();
        }
    }

    Ok(())
}

/// Close one streaming mac context.
pub(crate) fn mac_close(
    binding: &BindingCallContext,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    // remove mac resource entry
    let Some(entry) =
        binding
            .worker()
            .resources
            .remove(&binding.world(), handle.0, Some(binding.engine()))
    else {
        return Err(core_platform::io_not_found(
            "destack.crypto.mac.close",
            format!("unknown crypto mac handle {}", handle.0.0),
        ));
    };

    // validate handle kind
    if entry.kind != CRYPTO_MAC_RESOURCE_KIND {
        return Err(core_platform::io_not_found(
            "destack.crypto.mac.close",
            format!("unknown crypto mac handle {}", handle.0.0),
        ));
    }

    // wipe sensitive stream state before releasing the final resource entry
    let payload = entry.payload_cloned::<Arc<Mutex<CryptoMacResource>>>();
    if let Some(resource) = payload {
        let mut resource = resource.lock();
        match &mut resource.state {
            CryptoMacState::Software {
                inner_key,
                outer_key,
                ..
            } => {
                inner_key.zeroize();
                outer_key.zeroize();
            }
            CryptoMacState::HostSecret { payload, .. } => {
                payload.zeroize();
            }
        }
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
