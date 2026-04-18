use std::sync::Arc;

use openssl::hash::{Hasher, MessageDigest, hash};
use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::CryptoDigestAlgorithm;
use crate::platform::resource::ResourceEntry;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core::{
    CRYPTO_DIGEST_LABEL, CRYPTO_DIGEST_RESOURCE_KIND, CryptoDigestResource, openssl_error,
    resolve_digest_resource,
};

/// Compute one digest in one shot.
pub(crate) fn digest_compute(
    algorithm: CryptoDigestAlgorithm,
    payload: &[u8],
) -> RuntimeResult<Vec<u8>> {
    // resolve digest implementation
    let digest = message_digest(algorithm)?;

    // hash payload in one shot
    let digest = hash(digest, payload)
        .map_err(|error| openssl_error("destack.crypto.digest.compute", error))?;

    Ok(digest.to_vec())
}

/// Open one streaming digest context.
pub(crate) fn digest_open(
    binding: &BindingCallContext,
    algorithm: CryptoDigestAlgorithm,
) -> RuntimeResult<resource::CryptoDigestHandle> {
    // allocate hasher state for the selected digest
    let digest = message_digest(algorithm)?;
    let hasher =
        Hasher::new(digest).map_err(|error| openssl_error("destack.crypto.digest.open", error))?;

    // publish digest resource
    let resource_value = CryptoDigestResource { algorithm, hasher };
    let entry = ResourceEntry::new(CRYPTO_DIGEST_RESOURCE_KIND)
        .with_label(CRYPTO_DIGEST_LABEL)
        .with_payload(Arc::new(Mutex::new(resource_value)));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    Ok(resource::CryptoDigestHandle(resource_id))
}

/// Update one streaming digest context.
pub(crate) fn digest_update(
    binding: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
    payload: &[u8],
) -> RuntimeResult<()> {
    // resolve the digest state
    let resource = resolve_digest_resource(binding, handle, "destack.crypto.digest.update")?;
    let mut resource = resource.lock();

    // feed payload bytes
    resource
        .hasher
        .update(payload)
        .map_err(|error| openssl_error("destack.crypto.digest.update", error))
}

/// Finalize one streaming digest context and return digest bytes.
pub(crate) fn digest_finish(
    binding: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<Vec<u8>> {
    // resolve the digest state
    let resource = resolve_digest_resource(binding, handle, "destack.crypto.digest.finish")?;
    let mut resource = resource.lock();

    // finalize and capture digest output
    let digest = resource
        .hasher
        .finish()
        .map_err(|error| openssl_error("destack.crypto.digest.finish", error))?;

    // reinitialize hasher so the handle stays reusable
    let new_hasher = Hasher::new(message_digest(resource.algorithm)?)
        .map_err(|error| openssl_error("destack.crypto.digest.finish", error))?;
    resource.hasher = new_hasher;

    Ok(digest.to_vec())
}

/// Reset one streaming digest context.
pub(crate) fn digest_reset(
    binding: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    // resolve the digest state
    let resource = resolve_digest_resource(binding, handle, "destack.crypto.digest.reset")?;
    let mut resource = resource.lock();

    // replace with a fresh hasher instance
    resource.hasher = Hasher::new(message_digest(resource.algorithm)?)
        .map_err(|error| openssl_error("destack.crypto.digest.reset", error))?;

    Ok(())
}

/// Close one streaming digest context.
pub(crate) fn digest_close(
    binding: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    // remove resource and validate handle kind
    let Some(entry) =
        binding
            .worker()
            .resources
            .remove(&binding.world(), handle.0, Some(binding.engine()))
    else {
        return Err(core_platform::io_not_found(
            "destack.crypto.digest.close",
            format!("unknown crypto digest handle {}", handle.0.0),
        ));
    };
    if entry.kind != CRYPTO_DIGEST_RESOURCE_KIND {
        return Err(core_platform::io_not_found(
            "destack.crypto.digest.close",
            format!("unknown crypto digest handle {}", handle.0.0),
        ));
    }

    Ok(())
}

/// Return one openssl digest object for one digest algorithm.
pub(super) fn message_digest(algorithm: CryptoDigestAlgorithm) -> RuntimeResult<MessageDigest> {
    // map runtime digest lanes to openssl implementations
    match algorithm {
        CryptoDigestAlgorithm::Sha1 => Ok(MessageDigest::sha1()),
        CryptoDigestAlgorithm::Sha224 => Ok(MessageDigest::sha224()),
        CryptoDigestAlgorithm::Sha256 => Ok(MessageDigest::sha256()),
        CryptoDigestAlgorithm::Sha384 => Ok(MessageDigest::sha384()),
        CryptoDigestAlgorithm::Sha512 => Ok(MessageDigest::sha512()),
        CryptoDigestAlgorithm::Sha3_256 => Ok(MessageDigest::sha3_256()),
        CryptoDigestAlgorithm::Sha3_384 => Ok(MessageDigest::sha3_384()),
        CryptoDigestAlgorithm::Sha3_512 => Ok(MessageDigest::sha3_512()),
        CryptoDigestAlgorithm::Blake2b512 => {
            MessageDigest::from_name("blake2b512").ok_or_else(|| {
                core_platform::invalid_argument("algorithm", "blake2b512 digest is not available")
            })
        }
        CryptoDigestAlgorithm::Blake2s256 => {
            MessageDigest::from_name("blake2s256").ok_or_else(|| {
                core_platform::invalid_argument("algorithm", "blake2s256 digest is not available")
            })
        }
        CryptoDigestAlgorithm::Unknown => Err(core_platform::invalid_argument(
            "algorithm",
            "algorithm must not be Unknown",
        )),
    }
}

/// Return fixed digest output size in bytes for one digest algorithm.
pub(crate) fn digest_output_size_bytes(algorithm: CryptoDigestAlgorithm) -> Option<u32> {
    match algorithm {
        CryptoDigestAlgorithm::Sha1 => Some(20),
        CryptoDigestAlgorithm::Sha224 => Some(28),
        CryptoDigestAlgorithm::Sha256 => Some(32),
        CryptoDigestAlgorithm::Sha384 => Some(48),
        CryptoDigestAlgorithm::Sha512 => Some(64),
        CryptoDigestAlgorithm::Sha3_256 => Some(32),
        CryptoDigestAlgorithm::Sha3_384 => Some(48),
        CryptoDigestAlgorithm::Sha3_512 => Some(64),
        CryptoDigestAlgorithm::Blake2b512 => Some(64),
        CryptoDigestAlgorithm::Blake2s256 => Some(32),
        CryptoDigestAlgorithm::Unknown => None,
    }
}
