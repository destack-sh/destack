use argon2::{Algorithm, Argon2, AssociatedData, ParamsBuilder, Version};
use openssl::pkcs5;
use zeroize::Zeroizing;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::crypto::{
    CryptoArgon2idRequest, CryptoDigestAlgorithm, CryptoHkdfRequest, CryptoPbkdf2Request,
    CryptoScryptRequest,
};

use super::core::{decode_native_bytes, invalid_data, openssl_error};
use super::digest::message_digest;
use super::mac::hmac_compute;

/// Derive key bytes with HKDF.
pub(crate) fn kdf_hkdf(request: CryptoHkdfRequest) -> RuntimeResult<Vec<u8>> {
    // decode hkdf inputs
    let ikm = Zeroizing::new(decode_native_bytes(
        request.input_key_material,
        "request.inputKeyMaterial",
    )?);
    let salt = decode_native_bytes(request.salt, "request.salt")?;
    let info = decode_native_bytes(request.info, "request.info")?;

    // derive requested output bytes
    hkdf_expand(
        request.digest,
        &ikm,
        &salt,
        &info,
        request.length as usize,
        "destack.crypto.kdf.hkdf",
    )
}

/// Derive key bytes with PBKDF2.
pub(crate) fn kdf_pbkdf2(request: CryptoPbkdf2Request) -> RuntimeResult<Vec<u8>> {
    // validate cost settings before decoding secret inputs
    if request.iterations == 0 {
        return Err(core_platform::invalid_argument(
            "request.iterations",
            "iterations must be greater than zero",
        ));
    }

    // decode pbkdf2 inputs
    let password = Zeroizing::new(decode_native_bytes(request.password, "request.password")?);
    let salt = decode_native_bytes(request.salt, "request.salt")?;

    // derive requested output bytes
    let mut output = vec![0u8; request.length as usize];
    pkcs5::pbkdf2_hmac(
        &password,
        &salt,
        request.iterations as usize,
        message_digest(request.digest)?,
        &mut output,
    )
    .map_err(|error| openssl_error("destack.crypto.kdf.pbkdf2", error))?;

    Ok(output)
}

/// Derive key bytes with scrypt.
pub(crate) fn kdf_scrypt(request: CryptoScryptRequest) -> RuntimeResult<Vec<u8>> {
    // validate cost settings before decoding secret inputs
    if request.cost < 2 || !request.cost.is_power_of_two() {
        return Err(core_platform::invalid_argument(
            "request.cost",
            "cost must be one power of two greater than 1",
        ));
    }
    if request.block_size == 0 || request.parallelization == 0 {
        return Err(core_platform::invalid_argument(
            "request.blockSize",
            "blockSize and parallelization must be greater than zero",
        ));
    }

    // decode scrypt inputs
    let password = Zeroizing::new(decode_native_bytes(request.password, "request.password")?);
    let salt = decode_native_bytes(request.salt, "request.salt")?;

    // derive requested output bytes
    let mut output = vec![0u8; request.length as usize];
    pkcs5::scrypt(
        &password,
        &salt,
        request.cost as u64,
        request.block_size as u64,
        request.parallelization as u64,
        request.max_memory_bytes,
        &mut output,
    )
    .map_err(|error| openssl_error("destack.crypto.kdf.scrypt", error))?;

    Ok(output)
}

/// Derive key bytes with Argon2id.
pub(crate) fn kdf_argon2id(request: CryptoArgon2idRequest) -> RuntimeResult<Vec<u8>> {
    // decode and validate non-secret parameters first
    let salt = decode_native_bytes(request.salt, "request.salt")?;
    let associated_data = decode_native_bytes(request.associated_data, "request.associatedData")?;

    if salt.len() < 8 {
        return Err(core_platform::invalid_argument(
            "request.salt",
            "salt must be at least 8 bytes",
        ));
    }

    // decode secret inputs after cheap validation checks
    let password = Zeroizing::new(decode_native_bytes(request.password, "request.password")?);
    let secret = Zeroizing::new(decode_native_bytes(request.secret, "request.secret")?);

    // build argon2 configuration
    let mut params_builder = ParamsBuilder::new();
    params_builder
        .m_cost(request.memory_ki_b)
        .t_cost(request.iterations)
        .p_cost(request.parallelism)
        .output_len(request.length as usize);
    if !associated_data.is_empty() {
        let associated_data = AssociatedData::new(&associated_data).map_err(|error| {
            core_platform::invalid_argument(
                "request.associatedData",
                format!("invalid associatedData: {error}"),
            )
        })?;
        params_builder.data(associated_data);
    }
    let params = params_builder.build().map_err(|error| {
        core_platform::invalid_argument("request", format!("invalid argon2 parameters: {error}"))
    })?;

    // build argon2 context with optional secret
    let argon2 = if secret.is_empty() {
        Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
    } else {
        Argon2::new_with_secret(&secret, Algorithm::Argon2id, Version::V0x13, params).map_err(
            |error| {
                core_platform::invalid_argument(
                    "request.secret",
                    format!("invalid argon2 secret: {error}"),
                )
            },
        )?
    };

    // derive requested output bytes
    let mut output = vec![0u8; request.length as usize];
    argon2
        .hash_password_into(&password, &salt, &mut output)
        .map_err(|error| {
            invalid_data(
                "destack.crypto.kdf.argon2id",
                format!("argon2 failure: {error}"),
            )
        })?;

    Ok(output)
}

/// Build one HKDF output.
pub(super) fn hkdf_expand(
    digest: CryptoDigestAlgorithm,
    ikm: &[u8],
    salt: &[u8],
    info: &[u8],
    length: usize,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // validate hkdf output length against digest limits
    let message_digest = message_digest(digest)?;
    let digest_size = message_digest.size();
    let max_length = 255usize.checked_mul(digest_size).ok_or_else(|| {
        core_platform::invalid_argument("request.length", "requested length overflowed")
    })?;
    if length > max_length {
        return Err(core_platform::invalid_argument(
            "request.length",
            format!("requested output exceeds hkdf maximum of {max_length} bytes"),
        ));
    }

    // run extract stage with explicit zero-salt fallback
    let extraction_salt = if salt.is_empty() {
        vec![0u8; digest_size]
    } else {
        salt.to_vec()
    };
    let extraction_salt = Zeroizing::new(extraction_salt);
    let pseudorandom_key = hmac_compute(digest, &extraction_salt, ikm, operation)?;
    let pseudorandom_key = Zeroizing::new(pseudorandom_key);

    // run expand stage
    let mut output = Vec::with_capacity(length);
    let mut block = Zeroizing::new(Vec::new());
    let mut counter: u8 = 1;
    while output.len() < length {
        let mut payload = Zeroizing::new(Vec::with_capacity(block.len() + info.len() + 1));
        payload.extend_from_slice(&block);
        payload.extend_from_slice(info);
        payload.push(counter);
        block = Zeroizing::new(hmac_compute(
            digest,
            &pseudorandom_key,
            &payload,
            operation,
        )?);
        output.extend_from_slice(&block);
        counter = counter.checked_add(1).ok_or_else(|| {
            core_platform::invalid_argument(
                "request.length",
                "requested output exceeded hkdf counter range",
            )
        })?;
    }
    output.truncate(length);

    Ok(output)
}
