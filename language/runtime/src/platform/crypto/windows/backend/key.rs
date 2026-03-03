use std::ffi::c_void;
use std::mem::size_of;
use std::ptr;

use openssl::bn::{BigNum, BigNumContext};
use openssl::ec::{EcGroup, EcPoint, PointConversionForm};
use openssl::ecdsa::EcdsaSig;
use openssl::hash::{MessageDigest, hash};
use openssl::pkey::{PKey, Private, Public};
use openssl::rsa::Rsa;
use windows_sys::Win32::Foundation::{
    NTE_BAD_KEYSET, NTE_BAD_KEYSET_PARAM, NTE_INVALID_PARAMETER, NTE_NO_KEY, NTE_NOT_SUPPORTED,
};
use windows_sys::Win32::Security::Cryptography::{
    BCRYPT_CHAIN_MODE_CBC, BCRYPT_ECCKEY_BLOB, BCRYPT_ECCPUBLIC_BLOB,
    BCRYPT_ECDH_PUBLIC_P256_MAGIC, BCRYPT_ECDH_PUBLIC_P384_MAGIC, BCRYPT_ECDH_PUBLIC_P521_MAGIC,
    BCRYPT_ECDSA_PUBLIC_P256_MAGIC, BCRYPT_ECDSA_PUBLIC_P384_MAGIC, BCRYPT_ECDSA_PUBLIC_P521_MAGIC,
    BCRYPT_KDF_RAW_SECRET, BCRYPT_OAEP_PADDING_INFO, BCRYPT_PKCS1_PADDING_INFO,
    BCRYPT_PSS_PADDING_INFO, BCRYPT_RSAKEY_BLOB, BCRYPT_RSAPUBLIC_BLOB, BCRYPT_RSAPUBLIC_MAGIC,
    BCryptBuffer, BCryptBufferDesc, MS_KEY_STORAGE_PROVIDER, MS_PLATFORM_CRYPTO_PROVIDER,
    NCRYPT_AES_ALGORITHM, NCRYPT_CHAINING_MODE_PROPERTY, NCRYPT_CIPHER_BLOCK_PADDING_FLAG,
    NCRYPT_CIPHER_PADDING_INFO, NCRYPT_ECDH_P256_ALGORITHM, NCRYPT_ECDH_P384_ALGORITHM,
    NCRYPT_ECDH_P521_ALGORITHM, NCRYPT_ECDSA_P256_ALGORITHM, NCRYPT_ECDSA_P384_ALGORITHM,
    NCRYPT_ECDSA_P521_ALGORITHM, NCRYPT_EXPORT_POLICY_PROPERTY, NCRYPT_FLAGS,
    NCRYPT_HMAC_SHA256_ALGORITHM, NCRYPT_KEY_HANDLE, NCRYPT_LENGTH_PROPERTY,
    NCRYPT_MACHINE_KEY_FLAG, NCRYPT_OVERWRITE_KEY_FLAG, NCRYPT_PAD_CIPHER_FLAG,
    NCRYPT_PAD_OAEP_FLAG, NCRYPT_PAD_PKCS1_FLAG, NCRYPT_PAD_PSS_FLAG,
    NCRYPT_PKCS8_PRIVATE_KEY_BLOB, NCRYPT_PROV_HANDLE, NCRYPT_RSA_ALGORITHM, NCRYPT_SECRET_HANDLE,
    NCRYPT_SHA1_ALGORITHM, NCRYPT_SHA256_ALGORITHM, NCRYPT_SHA384_ALGORITHM,
    NCRYPT_SHA512_ALGORITHM, NCRYPT_SILENT_FLAG, NCRYPTBUFFER_PKCS_KEY_NAME, NCRYPTBUFFER_VERSION,
    NCryptCreatePersistedKey, NCryptDecrypt, NCryptDeleteKey, NCryptDeriveKey, NCryptEncrypt,
    NCryptExportKey, NCryptFinalizeKey, NCryptFreeObject, NCryptImportKey, NCryptOpenKey,
    NCryptOpenStorageProvider, NCryptSecretAgreement, NCryptSetProperty, NCryptSignHash,
};
use windows_sys::core::PCWSTR;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::crypto::core::{
    self as crypto_core, HostGeneratedKeyPair, HostKeyBackend, HostKeyMaterial,
};
use crate::platform::crypto::{
    CryptoAsymmetricEncryptionAlgorithm, CryptoAsymmetricEncryptionParameters,
    CryptoCipherAlgorithm, CryptoCipherParameters, CryptoDigestAlgorithm, CryptoKeyAlgorithm,
    CryptoKeyUsageMask, CryptoMacAlgorithm, CryptoMacParameters, CryptoNamedCurve,
    CryptoSignatureAlgorithm, CryptoSignatureParameters, CryptoStoreKind,
};
use crate::runtime::BindingCallContext;

use super::core::{invalid_data, permission_denied};
use crate::platform::core as core_platform;

/// Usage-bit mask for sign operations.
const KEY_USAGE_SIGN: u32 = 0x0000_0001;

/// Usage-bit mask for verify operations.
const KEY_USAGE_VERIFY: u32 = 0x0000_0002;

/// Usage-bit mask for derive-bits operations.
const KEY_USAGE_DERIVE_BITS: u32 = 0x0000_0040;

/// Usage-bit mask for derive-key operations.
const KEY_USAGE_DERIVE_KEYS: u32 = 0x0000_0080;

/// AES block size in bytes.
const AES_BLOCK_SIZE_BYTES: usize = 16;

/// Probe payload size for host HMAC operations.
const HMAC_PROBE_PAYLOAD_SIZE_BYTES: usize = 32;

/// Host provider lanes used for persisted-key operations.
#[derive(Clone, Copy, PartialEq, Eq)]
enum WindowsProviderKind {
    /// Software key-storage provider.
    Software,
    /// Platform key-storage provider.
    Platform,
}

/// Return whether one status code is one successful HRESULT.
fn status_is_success(status: i32) -> bool {
    status >= 0
}

/// Return whether one status code means one missing persisted key.
fn status_is_key_not_found(status: i32) -> bool {
    status == NTE_NO_KEY || status == NTE_BAD_KEYSET
}

/// Return one mapped runtime error for one CNG status code.
fn status_error(operation: &'static str, action: &'static str, status: i32) -> Box<RuntimeError> {
    // map missing-key statuses into ioNotFound
    if status_is_key_not_found(status) {
        return core_platform::io_not_found(
            operation,
            format!("{action} failed because one host key was not found"),
        );
    }

    // map explicit unsupported statuses into notSupported
    if status == NTE_NOT_SUPPORTED {
        return core_platform::not_supported(operation);
    }

    // map argument failures into ioInvalidData
    if status == NTE_INVALID_PARAMETER || status == NTE_BAD_KEYSET_PARAM {
        return invalid_data(
            operation,
            format!("{action} failed with status code {status}"),
        );
    }

    // map all other failures into ioPermissionDenied
    permission_denied(
        operation,
        format!("{action} failed with status code {status}"),
    )
}

/// Close one CNG object handle when present.
fn close_handle(handle: usize) {
    if handle == 0 {
        return;
    }

    unsafe {
        let _ = NCryptFreeObject(handle);
    }
}

/// Return the NCrypt provider constant for one provider lane.
fn provider_name(provider_kind: WindowsProviderKind) -> PCWSTR {
    match provider_kind {
        WindowsProviderKind::Software => MS_KEY_STORAGE_PROVIDER,
        WindowsProviderKind::Platform => MS_PLATFORM_CRYPTO_PROVIDER,
    }
}

/// Return one host backend lane for one provider and key algorithm.
fn backend_for_provider_and_algorithm(
    provider_kind: WindowsProviderKind,
    algorithm: CryptoKeyAlgorithm,
) -> Option<HostKeyBackend> {
    match (provider_kind, algorithm) {
        (WindowsProviderKind::Software, CryptoKeyAlgorithm::Rsa) => {
            Some(HostKeyBackend::WindowsSoftwareKeyStorageRsa)
        }
        (WindowsProviderKind::Software, CryptoKeyAlgorithm::Ec) => {
            Some(HostKeyBackend::WindowsSoftwareKeyStorageEc)
        }
        (WindowsProviderKind::Platform, CryptoKeyAlgorithm::Rsa) => {
            Some(HostKeyBackend::WindowsPlatformKeyStorageRsa)
        }
        (WindowsProviderKind::Platform, CryptoKeyAlgorithm::Ec) => {
            Some(HostKeyBackend::WindowsPlatformKeyStorageEc)
        }
        (WindowsProviderKind::Platform, CryptoKeyAlgorithm::Aes) => {
            Some(HostKeyBackend::WindowsPlatformKeyStorageAes)
        }
        (WindowsProviderKind::Platform, CryptoKeyAlgorithm::Hmac) => {
            Some(HostKeyBackend::WindowsPlatformKeyStorageHmac)
        }
        _ => None,
    }
}

/// Return one provider lane for one host backend.
fn provider_kind_from_backend(backend: HostKeyBackend) -> Option<WindowsProviderKind> {
    match backend {
        HostKeyBackend::WindowsSoftwareKeyStorageRsa
        | HostKeyBackend::WindowsSoftwareKeyStorageEc => Some(WindowsProviderKind::Software),
        HostKeyBackend::WindowsPlatformKeyStorageRsa
        | HostKeyBackend::WindowsPlatformKeyStorageEc
        | HostKeyBackend::WindowsPlatformKeyStorageAes
        | HostKeyBackend::WindowsPlatformKeyStorageHmac => Some(WindowsProviderKind::Platform),
        _ => None,
    }
}

/// Return one host backend lane for one platform secret-key algorithm.
fn secret_backend_for_algorithm(algorithm: CryptoKeyAlgorithm) -> Option<HostKeyBackend> {
    match algorithm {
        CryptoKeyAlgorithm::Aes => Some(HostKeyBackend::WindowsPlatformKeyStorageAes),
        CryptoKeyAlgorithm::Hmac => Some(HostKeyBackend::WindowsPlatformKeyStorageHmac),
        _ => None,
    }
}

/// Return one NCrypt algorithm id for one platform secret-key lane.
fn secret_algorithm_name(
    algorithm: CryptoKeyAlgorithm,
    digest: CryptoDigestAlgorithm,
    operation: &'static str,
) -> RuntimeResult<PCWSTR> {
    // map AES secret-key generation
    if algorithm == CryptoKeyAlgorithm::Aes {
        return Ok(NCRYPT_AES_ALGORITHM);
    }

    // map HMAC secret-key generation and enforce digest lane support
    if algorithm == CryptoKeyAlgorithm::Hmac {
        if digest == CryptoDigestAlgorithm::Unknown || digest == CryptoDigestAlgorithm::Sha256 {
            return Ok(NCRYPT_HMAC_SHA256_ALGORITHM);
        }

        return Err(core_platform::not_supported(operation));
    }

    Err(core_platform::not_supported(operation))
}

/// Return one UTF-16 property value length in bytes.
fn utf16_property_byte_length(value: PCWSTR) -> u32 {
    // count UTF-16 code units including trailing terminator
    let mut length = 0usize;
    unsafe {
        while *value.add(length) != 0 {
            length += 1;
        }
    }

    ((length + 1) * size_of::<u16>()) as u32
}

/// Set one u32 NCrypt property on one key.
fn set_key_u32_property(
    key: NCRYPT_KEY_HANDLE,
    property: PCWSTR,
    value: u32,
    action: &'static str,
    operation: &'static str,
) -> RuntimeResult<()> {
    let status = unsafe {
        NCryptSetProperty(
            key,
            property,
            (&value as *const u32).cast(),
            size_of::<u32>() as u32,
            0,
        )
    };
    if !status_is_success(status) {
        return Err(status_error(operation, action, status));
    }

    Ok(())
}

/// Set one UTF-16 NCrypt property on one key.
fn set_key_wstring_property(
    key: NCRYPT_KEY_HANDLE,
    property: PCWSTR,
    value: PCWSTR,
    action: &'static str,
    operation: &'static str,
) -> RuntimeResult<()> {
    let value_size = utf16_property_byte_length(value);
    let status =
        unsafe { NCryptSetProperty(key, property, value.cast_mut().cast(), value_size, 0) };
    if !status_is_success(status) {
        return Err(status_error(operation, action, status));
    }

    Ok(())
}

/// Return whether one usage-mask requests one key-derivation lane.
fn usage_requests_derive(usage_mask: CryptoKeyUsageMask) -> bool {
    (usage_mask.0 & (KEY_USAGE_DERIVE_BITS | KEY_USAGE_DERIVE_KEYS)) != 0
}

/// Return whether one usage-mask requests one signature lane.
fn usage_requests_sign(usage_mask: CryptoKeyUsageMask) -> bool {
    (usage_mask.0 & (KEY_USAGE_SIGN | KEY_USAGE_VERIFY)) != 0
}

/// Return one key flag mask for one store lane.
fn key_flags_for_store_kind(kind: CryptoStoreKind) -> NCRYPT_FLAGS {
    let mut flags = NCRYPT_SILENT_FLAG;
    if kind == CryptoStoreKind::Machine {
        flags |= NCRYPT_MACHINE_KEY_FLAG;
    }

    flags
}

/// Convert one key label into one UTF-16 key name.
fn key_name_utf16(key_label: &str, operation: &'static str) -> RuntimeResult<Vec<u16>> {
    // reject empty labels before creating one persisted key name
    if key_label.is_empty() {
        return Err(invalid_data(operation, "host key label must not be empty"));
    }

    // encode key name as UTF-16 with one trailing null
    let mut key_name = key_label.encode_utf16().collect::<Vec<u16>>();
    key_name.push(0);

    Ok(key_name)
}

/// Open one key-storage provider lane.
fn open_provider_with_kind(
    provider_kind: WindowsProviderKind,
    operation: &'static str,
) -> RuntimeResult<NCRYPT_PROV_HANDLE> {
    // open one key-storage provider lane
    let mut provider = 0usize;
    let status =
        unsafe { NCryptOpenStorageProvider(&mut provider, provider_name(provider_kind), 0) };
    if !status_is_success(status) {
        return Err(status_error(
            operation,
            "open one key-storage provider",
            status,
        ));
    }

    Ok(provider)
}

/// Open one persisted key from one store lane and provider lane.
fn open_persisted_key_with_kind(
    provider_kind: WindowsProviderKind,
    kind: CryptoStoreKind,
    key_label: &str,
    operation: &'static str,
) -> RuntimeResult<(NCRYPT_PROV_HANDLE, NCRYPT_KEY_HANDLE)> {
    // open one provider and derive key-open flags for the lane
    let provider = open_provider_with_kind(provider_kind, operation)?;
    let flags = key_flags_for_store_kind(kind);

    // open one persisted key by label
    let key_name = key_name_utf16(key_label, operation)?;
    let mut key = 0usize;
    let status = unsafe { NCryptOpenKey(provider, &mut key, key_name.as_ptr(), 0, flags) };
    if !status_is_success(status) {
        close_handle(provider);
        return Err(status_error(
            operation,
            "open one persisted host key",
            status,
        ));
    }

    Ok((provider, key))
}

/// Open one persisted key and return none when the key label is absent in the selected lane.
fn try_open_persisted_key_with_kind(
    provider_kind: WindowsProviderKind,
    kind: CryptoStoreKind,
    key_label: &str,
    operation: &'static str,
) -> RuntimeResult<Option<(NCRYPT_PROV_HANDLE, NCRYPT_KEY_HANDLE)>> {
    // open one provider and derive key-open flags for the lane
    let provider = open_provider_with_kind(provider_kind, operation)?;
    let flags = key_flags_for_store_kind(kind);

    // open one persisted key by label
    let key_name = key_name_utf16(key_label, operation)?;
    let mut key = 0usize;
    let status = unsafe { NCryptOpenKey(provider, &mut key, key_name.as_ptr(), 0, flags) };
    if !status_is_success(status) {
        close_handle(provider);
        if status_is_key_not_found(status) {
            return Ok(None);
        }

        return Err(status_error(
            operation,
            "open one persisted host key",
            status,
        ));
    }

    Ok(Some((provider, key)))
}

/// Open one persisted key from one backend lane.
fn open_persisted_key_for_backend(
    backend: HostKeyBackend,
    kind: CryptoStoreKind,
    key_label: &str,
    operation: &'static str,
) -> RuntimeResult<(NCRYPT_PROV_HANDLE, NCRYPT_KEY_HANDLE)> {
    let Some(provider_kind) = provider_kind_from_backend(backend) else {
        return Err(core_platform::not_supported(operation));
    };

    // enforce explicit user or machine lane provenance for persisted windows host keys
    if kind != CryptoStoreKind::User && kind != CryptoStoreKind::Machine {
        return Err(core_platform::not_supported(operation));
    }

    open_persisted_key_with_kind(provider_kind, kind, key_label, operation)
}

/// Return digest metadata for one signature request.
fn signature_digest_metadata(
    digest: CryptoDigestAlgorithm,
    operation: &'static str,
) -> RuntimeResult<(MessageDigest, PCWSTR)> {
    // map runtime digest lanes into openssl and CNG identifiers
    let metadata = match digest {
        CryptoDigestAlgorithm::Sha1 => (MessageDigest::sha1(), NCRYPT_SHA1_ALGORITHM),
        CryptoDigestAlgorithm::Sha256 => (MessageDigest::sha256(), NCRYPT_SHA256_ALGORITHM),
        CryptoDigestAlgorithm::Sha384 => (MessageDigest::sha384(), NCRYPT_SHA384_ALGORITHM),
        CryptoDigestAlgorithm::Sha512 => (MessageDigest::sha512(), NCRYPT_SHA512_ALGORITHM),
        _ => return Err(core_platform::not_supported(operation)),
    };

    Ok(metadata)
}

/// Decode one big-endian exponent value into one u32.
fn decode_exponent(exponent_bytes: &[u8], operation: &'static str) -> RuntimeResult<u32> {
    // reject exponents that exceed one u32 payload
    if exponent_bytes.is_empty() || exponent_bytes.len() > 4 {
        return Err(invalid_data(
            operation,
            "host key exponent does not fit one u32 value",
        ));
    }

    // decode one big-endian integer into one u32 accumulator
    let mut exponent = 0u32;
    for byte in exponent_bytes {
        exponent = (exponent << 8) | u32::from(*byte);
    }

    Ok(exponent)
}

/// Parse one RSA public blob into one OpenSSL public key.
fn parse_rsa_public_blob(
    blob: &[u8],
    operation: &'static str,
) -> RuntimeResult<(PKey<Public>, u32, u32)> {
    // validate blob header size and decode the rsa public header
    if blob.len() < size_of::<BCRYPT_RSAKEY_BLOB>() {
        return Err(invalid_data(operation, "host rsa public blob is truncated"));
    }
    let header = unsafe { ptr::read_unaligned(blob.as_ptr().cast::<BCRYPT_RSAKEY_BLOB>()) };
    if header.Magic != BCRYPT_RSAPUBLIC_MAGIC {
        return Err(invalid_data(
            operation,
            "host key blob is not one rsa public key payload",
        ));
    }

    // split exponent and modulus slices according to header lengths
    let header_bytes = size_of::<BCRYPT_RSAKEY_BLOB>();
    let exponent_end = header_bytes
        .checked_add(header.cbPublicExp as usize)
        .ok_or_else(|| invalid_data(operation, "host rsa exponent length overflow"))?;
    let modulus_end = exponent_end
        .checked_add(header.cbModulus as usize)
        .ok_or_else(|| invalid_data(operation, "host rsa modulus length overflow"))?;
    if modulus_end > blob.len() {
        return Err(invalid_data(operation, "host rsa public blob is truncated"));
    }
    let exponent_bytes = &blob[header_bytes..exponent_end];
    let modulus_bytes = &blob[exponent_end..modulus_end];

    // convert exponent and modulus into one OpenSSL public key object
    let exponent = decode_exponent(exponent_bytes, operation)?;
    let modulus = BigNum::from_slice(modulus_bytes)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let exponent =
        BigNum::from_u32(exponent).map_err(|error| invalid_data(operation, format!("{error}")))?;
    let rsa = Rsa::from_public_components(modulus, exponent)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let public_key =
        PKey::from_rsa(rsa).map_err(|error| invalid_data(operation, format!("{error}")))?;
    let public_exponent = decode_exponent(exponent_bytes, operation)?;

    Ok((public_key, header.BitLength, public_exponent))
}

/// Return one windows ECDSA algorithm constant for one named curve.
fn ecdsa_algorithm_for_curve(named_curve: CryptoNamedCurve) -> Option<PCWSTR> {
    match named_curve {
        CryptoNamedCurve::P256 => Some(NCRYPT_ECDSA_P256_ALGORITHM),
        CryptoNamedCurve::P384 => Some(NCRYPT_ECDSA_P384_ALGORITHM),
        CryptoNamedCurve::P521 => Some(NCRYPT_ECDSA_P521_ALGORITHM),
        _ => None,
    }
}

/// Return one windows ECDH algorithm constant for one named curve.
fn ecdh_algorithm_for_curve(named_curve: CryptoNamedCurve) -> Option<PCWSTR> {
    match named_curve {
        CryptoNamedCurve::P256 => Some(NCRYPT_ECDH_P256_ALGORITHM),
        CryptoNamedCurve::P384 => Some(NCRYPT_ECDH_P384_ALGORITHM),
        CryptoNamedCurve::P521 => Some(NCRYPT_ECDH_P521_ALGORITHM),
        _ => None,
    }
}

/// Return one windows ECDH public-blob magic for one named curve.
fn ecdh_public_magic_for_curve(named_curve: CryptoNamedCurve) -> Option<u32> {
    match named_curve {
        CryptoNamedCurve::P256 => Some(BCRYPT_ECDH_PUBLIC_P256_MAGIC),
        CryptoNamedCurve::P384 => Some(BCRYPT_ECDH_PUBLIC_P384_MAGIC),
        CryptoNamedCurve::P521 => Some(BCRYPT_ECDH_PUBLIC_P521_MAGIC),
        _ => None,
    }
}

/// Return one named curve from one windows EC public-blob magic value.
fn named_curve_from_ec_public_magic(magic: u32) -> Option<CryptoNamedCurve> {
    match magic {
        BCRYPT_ECDSA_PUBLIC_P256_MAGIC | BCRYPT_ECDH_PUBLIC_P256_MAGIC => {
            Some(CryptoNamedCurve::P256)
        }
        BCRYPT_ECDSA_PUBLIC_P384_MAGIC | BCRYPT_ECDH_PUBLIC_P384_MAGIC => {
            Some(CryptoNamedCurve::P384)
        }
        BCRYPT_ECDSA_PUBLIC_P521_MAGIC | BCRYPT_ECDH_PUBLIC_P521_MAGIC => {
            Some(CryptoNamedCurve::P521)
        }
        _ => None,
    }
}

/// Parse one EC public blob into one OpenSSL public key.
fn parse_ec_public_blob(
    blob: &[u8],
    operation: &'static str,
) -> RuntimeResult<(PKey<Public>, CryptoNamedCurve, u32)> {
    // decode and validate the EC public blob header
    if blob.len() < size_of::<BCRYPT_ECCKEY_BLOB>() {
        return Err(invalid_data(operation, "host ec public blob is truncated"));
    }
    let header = unsafe { ptr::read_unaligned(blob.as_ptr().cast::<BCRYPT_ECCKEY_BLOB>()) };
    let Some(named_curve) = named_curve_from_ec_public_magic(header.dwMagic) else {
        return Err(invalid_data(
            operation,
            "host key blob is not one supported ec public key payload",
        ));
    };
    let (_, _, curve_nid) = crypto_core::resolve_nist_p_curve(named_curve).ok_or_else(|| {
        invalid_data(
            operation,
            "host key blob references one unsupported ec named curve",
        )
    })?;

    // split x and y coordinates from the blob payload
    let coordinate_size = header.cbKey as usize;
    let header_size = size_of::<BCRYPT_ECCKEY_BLOB>();
    let x_end = header_size
        .checked_add(coordinate_size)
        .ok_or_else(|| invalid_data(operation, "host ec x-coordinate length overflow"))?;
    let y_end = x_end
        .checked_add(coordinate_size)
        .ok_or_else(|| invalid_data(operation, "host ec y-coordinate length overflow"))?;
    if y_end > blob.len() {
        return Err(invalid_data(operation, "host ec public blob is truncated"));
    }
    let x = &blob[header_size..x_end];
    let y = &blob[x_end..y_end];

    // build one uncompressed SEC1 point and convert to OpenSSL public key
    let mut uncompressed = Vec::with_capacity(1 + x.len() + y.len());
    uncompressed.push(0x04);
    uncompressed.extend_from_slice(x);
    uncompressed.extend_from_slice(y);
    let group = EcGroup::from_curve_name(curve_nid)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let mut context =
        BigNumContext::new().map_err(|error| invalid_data(operation, format!("{error}")))?;
    let point = EcPoint::from_bytes(&group, &uncompressed, &mut context)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let ec_key = openssl::ec::EcKey::from_public_key(&group, &point)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let public_key =
        PKey::from_ec_key(ec_key).map_err(|error| invalid_data(operation, format!("{error}")))?;

    Ok((public_key, named_curve, coordinate_size as u32 * 8))
}

/// Export one persisted EC key into one public-key blob.
fn export_ec_public_blob(
    key: NCRYPT_KEY_HANDLE,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // query one output length before exporting the blob
    let mut blob_size = 0u32;
    let size_status = unsafe {
        NCryptExportKey(
            key,
            0,
            BCRYPT_ECCPUBLIC_BLOB,
            ptr::null(),
            ptr::null_mut(),
            0,
            &mut blob_size,
            0,
        )
    };
    if !status_is_success(size_status) {
        return Err(status_error(
            operation,
            "query one host ec public blob size",
            size_status,
        ));
    }

    // export one public-key blob into one owned vector
    let mut blob = vec![0u8; blob_size as usize];
    let export_status = unsafe {
        NCryptExportKey(
            key,
            0,
            BCRYPT_ECCPUBLIC_BLOB,
            ptr::null(),
            blob.as_mut_ptr(),
            blob.len() as u32,
            &mut blob_size,
            0,
        )
    };
    if !status_is_success(export_status) {
        return Err(status_error(
            operation,
            "export one host ec public key",
            export_status,
        ));
    }
    blob.truncate(blob_size as usize);

    Ok(blob)
}

/// Import one persisted PKCS#8 private key into one provider lane.
fn import_persistent_private_key_from_pkcs8(
    provider: NCRYPT_PROV_HANDLE,
    kind: CryptoStoreKind,
    key_label: &str,
    private_key_pkcs8_der: &[u8],
    operation: &'static str,
) -> RuntimeResult<NCRYPT_KEY_HANDLE> {
    // build one PKCS key-name parameter list for persistent import
    let mut key_name = key_name_utf16(key_label, operation)?;
    let mut key_name_buffer = BCryptBuffer {
        cbBuffer: (key_name.len() * size_of::<u16>()) as u32,
        BufferType: NCRYPTBUFFER_PKCS_KEY_NAME,
        pvBuffer: key_name.as_mut_ptr().cast(),
    };
    let mut parameter_list = BCryptBufferDesc {
        ulVersion: NCRYPTBUFFER_VERSION,
        cBuffers: 1,
        pBuffers: (&mut key_name_buffer as *mut BCryptBuffer),
    };

    // import and persist one PKCS#8 private key payload
    let flags = key_flags_for_store_kind(kind) | NCRYPT_OVERWRITE_KEY_FLAG;
    let mut key = 0usize;
    let status = unsafe {
        NCryptImportKey(
            provider,
            0,
            NCRYPT_PKCS8_PRIVATE_KEY_BLOB,
            (&mut parameter_list as *mut BCryptBufferDesc).cast(),
            &mut key,
            private_key_pkcs8_der.as_ptr(),
            private_key_pkcs8_der.len() as u32,
            flags,
        )
    };
    if !status_is_success(status) {
        return Err(status_error(
            operation,
            "import one persisted host private key",
            status,
        ));
    }

    // enforce non-exportable host policy on imported persistent keys
    let export_policy = 0u32;
    let set_export_status = unsafe {
        NCryptSetProperty(
            key,
            NCRYPT_EXPORT_POLICY_PROPERTY,
            (&export_policy as *const u32).cast(),
            size_of::<u32>() as u32,
            0,
        )
    };
    if !status_is_success(set_export_status) {
        close_handle(key);
        return Err(status_error(
            operation,
            "set one host key export policy",
            set_export_status,
        ));
    }

    Ok(key)
}

/// Generate one persisted host key pair on one provider lane.
#[allow(clippy::too_many_arguments)]
fn generate_persistent_key_pair_with_provider(
    provider_kind: WindowsProviderKind,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    usage_mask: CryptoKeyUsageMask,
    modulus_bits: u32,
    public_exponent: u32,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<Option<HostGeneratedKeyPair>> {
    // only user and machine host lanes expose persisted key-store keys
    if !matches!(kind, CryptoStoreKind::User | CryptoStoreKind::Machine) {
        return Ok(None);
    }

    // generate one persisted RSA key pair on the selected provider lane
    if algorithm == CryptoKeyAlgorithm::Rsa {
        if named_curve != CryptoNamedCurve::Unknown {
            return Ok(None);
        }

        let resolved_public_exponent = if public_exponent == 0 {
            65537
        } else {
            public_exponent
        };
        if resolved_public_exponent != 65537 {
            return Ok(None);
        }
        let resolved_modulus_bits = if modulus_bits == 0 {
            2048
        } else {
            modulus_bits
        };

        let mut provider = 0usize;
        let mut key = 0usize;
        let mut should_delete_key = false;
        let result = (|| -> RuntimeResult<Option<HostGeneratedKeyPair>> {
            provider = open_provider_with_kind(provider_kind, operation)?;
            let key_name = key_name_utf16(persistent_key_label, operation)?;
            let store_flags = key_flags_for_store_kind(kind);
            let create_flags = store_flags | NCRYPT_OVERWRITE_KEY_FLAG;

            let create_status = unsafe {
                NCryptCreatePersistedKey(
                    provider,
                    &mut key,
                    NCRYPT_RSA_ALGORITHM,
                    key_name.as_ptr(),
                    0,
                    create_flags,
                )
            };
            if !status_is_success(create_status) {
                return Err(status_error(
                    operation,
                    "create one persisted host rsa key",
                    create_status,
                ));
            }
            should_delete_key = true;

            let mut key_bits = resolved_modulus_bits;
            let set_length_status = unsafe {
                NCryptSetProperty(
                    key,
                    NCRYPT_LENGTH_PROPERTY,
                    (&mut key_bits as *mut u32).cast(),
                    size_of::<u32>() as u32,
                    0,
                )
            };
            if !status_is_success(set_length_status) {
                return Err(status_error(
                    operation,
                    "set one host rsa key size",
                    set_length_status,
                ));
            }

            let export_policy = 0u32;
            let set_export_status = unsafe {
                NCryptSetProperty(
                    key,
                    NCRYPT_EXPORT_POLICY_PROPERTY,
                    (&export_policy as *const u32).cast(),
                    size_of::<u32>() as u32,
                    0,
                )
            };
            if !status_is_success(set_export_status) {
                return Err(status_error(
                    operation,
                    "set one host rsa export policy",
                    set_export_status,
                ));
            }

            let finalize_status = unsafe { NCryptFinalizeKey(key, store_flags) };
            if !status_is_success(finalize_status) {
                return Err(status_error(
                    operation,
                    "finalize one persisted host rsa key",
                    finalize_status,
                ));
            }

            // reject lanes that cannot satisfy host-signing requirements
            if !persisted_rsa_sign_is_supported(key)
                || !persisted_rsa_sign_is_supported_after_reopen(
                    provider_kind,
                    kind,
                    persistent_key_label,
                    operation,
                )
            {
                return Ok(None);
            }

            let public_blob = export_rsa_public_blob(key, operation)?;
            let (public_key, size_bits, public_exponent) =
                parse_rsa_public_blob(&public_blob, operation)?;
            let public_key_spki_der = public_key
                .public_key_to_der()
                .map_err(|error| invalid_data(operation, format!("{error}")))?;
            let backend =
                backend_for_provider_and_algorithm(provider_kind, CryptoKeyAlgorithm::Rsa)
                    .ok_or_else(|| core_platform::not_supported(operation))?;
            let private_material = HostKeyMaterial {
                backend,
                key_label: persistent_key_label.to_string(),
                public_key_spki_der: public_key_spki_der.clone(),
                private_key_der: Vec::new(),
            };
            should_delete_key = false;

            Ok(Some(HostGeneratedKeyPair {
                private_material: crypto_core::CryptoKeyMaterial::Host(private_material),
                public_key,
                algorithm: CryptoKeyAlgorithm::Rsa,
                named_curve: CryptoNamedCurve::Unknown,
                size_bits,
                modulus_bits: size_bits,
                public_exponent,
            }))
        })();

        // rollback partially-created keys when generation fails before finalize
        if should_delete_key && key != 0 {
            unsafe {
                let _ = NCryptDeleteKey(key, 0);
            }
            key = 0;
        }

        // release CNG handles in all control-flow paths
        close_handle(key);
        close_handle(provider);

        return result;
    }

    // generate one persisted EC key pair on the selected provider lane
    if algorithm == CryptoKeyAlgorithm::Ec {
        if modulus_bits != 0 || public_exponent != 0 {
            return Ok(None);
        }

        let Some((resolved_curve, _, _)) = crypto_core::resolve_nist_p_curve(named_curve) else {
            return Ok(None);
        };
        let requests_derive = usage_requests_derive(usage_mask);
        let requests_sign = usage_requests_sign(usage_mask);
        if requests_derive && requests_sign {
            return Ok(None);
        }

        let key_algorithm = if requests_derive {
            ecdh_algorithm_for_curve(resolved_curve)
        } else {
            ecdsa_algorithm_for_curve(resolved_curve)
        };
        let Some(key_algorithm) = key_algorithm else {
            return Ok(None);
        };

        let mut provider = 0usize;
        let mut key = 0usize;
        let mut should_delete_key = false;
        let result = (|| -> RuntimeResult<Option<HostGeneratedKeyPair>> {
            provider = open_provider_with_kind(provider_kind, operation)?;
            let key_name = key_name_utf16(persistent_key_label, operation)?;
            let store_flags = key_flags_for_store_kind(kind);
            let create_flags = store_flags | NCRYPT_OVERWRITE_KEY_FLAG;

            let create_status = unsafe {
                NCryptCreatePersistedKey(
                    provider,
                    &mut key,
                    key_algorithm,
                    key_name.as_ptr(),
                    0,
                    create_flags,
                )
            };
            if !status_is_success(create_status) {
                return Err(status_error(
                    operation,
                    "create one persisted host ec key",
                    create_status,
                ));
            }
            should_delete_key = true;

            let export_policy = 0u32;
            let set_export_status = unsafe {
                NCryptSetProperty(
                    key,
                    NCRYPT_EXPORT_POLICY_PROPERTY,
                    (&export_policy as *const u32).cast(),
                    size_of::<u32>() as u32,
                    0,
                )
            };
            if !status_is_success(set_export_status) {
                return Err(status_error(
                    operation,
                    "set one host ec export policy",
                    set_export_status,
                ));
            }

            let finalize_status = unsafe { NCryptFinalizeKey(key, store_flags) };
            if !status_is_success(finalize_status) {
                return Err(status_error(
                    operation,
                    "finalize one persisted host ec key",
                    finalize_status,
                ));
            }

            // reject lanes that cannot satisfy requested host EC capabilities
            if requests_sign
                && (!persisted_ec_sign_is_supported(key)
                    || !persisted_ec_sign_is_supported_after_reopen(
                        provider_kind,
                        kind,
                        persistent_key_label,
                        operation,
                    ))
            {
                return Ok(None);
            }

            let public_blob = export_ec_public_blob(key, operation)?;
            let (public_key, public_curve, size_bits) =
                parse_ec_public_blob(&public_blob, operation)?;
            if public_curve != resolved_curve {
                return Ok(None);
            }
            let public_key_spki_der = public_key
                .public_key_to_der()
                .map_err(|error| invalid_data(operation, format!("{error}")))?;
            let backend = backend_for_provider_and_algorithm(provider_kind, CryptoKeyAlgorithm::Ec)
                .ok_or_else(|| core_platform::not_supported(operation))?;
            let private_material = HostKeyMaterial {
                backend,
                key_label: persistent_key_label.to_string(),
                public_key_spki_der: public_key_spki_der.clone(),
                private_key_der: Vec::new(),
            };
            should_delete_key = false;

            Ok(Some(HostGeneratedKeyPair {
                private_material: crypto_core::CryptoKeyMaterial::Host(private_material),
                public_key,
                algorithm: CryptoKeyAlgorithm::Ec,
                named_curve: public_curve,
                size_bits,
                modulus_bits: 0,
                public_exponent: 0,
            }))
        })();

        // rollback partially-created keys when generation fails before finalize
        if should_delete_key && key != 0 {
            unsafe {
                let _ = NCryptDeleteKey(key, 0);
            }
            key = 0;
        }

        // release CNG handles in all control-flow paths
        close_handle(key);
        close_handle(provider);

        return result;
    }

    Ok(None)
}

/// Encode one peer SPKI EC public key into one windows ECDH public blob.
fn encode_ecdh_public_blob_from_spki(
    peer_public_spki_der: &[u8],
    named_curve: CryptoNamedCurve,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // parse one peer public key and enforce one EC family
    let public_key = PKey::public_key_from_der(peer_public_spki_der)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let ec_key = public_key
        .ec_key()
        .map_err(|error| invalid_data(operation, format!("{error}")))?;

    // resolve one supported named curve and encode SEC1 point bytes
    let Some((resolved_curve, _, curve_nid)) = crypto_core::resolve_nist_p_curve(named_curve)
    else {
        return Err(core_platform::not_supported(operation));
    };
    let group = EcGroup::from_curve_name(curve_nid)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let mut context =
        BigNumContext::new().map_err(|error| invalid_data(operation, format!("{error}")))?;
    let uncompressed = ec_key
        .public_key()
        .to_bytes(&group, PointConversionForm::UNCOMPRESSED, &mut context)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    if uncompressed.is_empty() || uncompressed[0] != 0x04 {
        return Err(invalid_data(
            operation,
            "peer ec public key is not one uncompressed point payload",
        ));
    }

    // split coordinates and build one windows ECC public blob
    let coordinates = &uncompressed[1..];
    if coordinates.len() % 2 != 0 {
        return Err(invalid_data(
            operation,
            "peer ec public key has one malformed coordinate payload",
        ));
    }
    let coordinate_size = coordinates.len() / 2;
    let Some(public_magic) = ecdh_public_magic_for_curve(resolved_curve) else {
        return Err(core_platform::not_supported(operation));
    };
    let header = BCRYPT_ECCKEY_BLOB {
        dwMagic: public_magic,
        cbKey: coordinate_size as u32,
    };
    let mut blob = vec![0u8; size_of::<BCRYPT_ECCKEY_BLOB>() + coordinates.len()];
    unsafe {
        ptr::write_unaligned(blob.as_mut_ptr().cast::<BCRYPT_ECCKEY_BLOB>(), header);
    }
    blob[size_of::<BCRYPT_ECCKEY_BLOB>()..].copy_from_slice(coordinates);

    Ok(blob)
}

/// Convert one raw ECDSA signature payload into DER encoding.
fn ecdsa_signature_der_from_raw(
    signature: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // split raw signature into equal r and s halves
    if signature.is_empty() || !signature.len().is_multiple_of(2) {
        return Err(invalid_data(
            operation,
            "host ecdsa signature payload has one invalid length",
        ));
    }
    let component_size = signature.len() / 2;
    let r = &signature[..component_size];
    let s = &signature[component_size..];

    // build one DER-encoded ASN.1 ECDSA signature object
    let r = BigNum::from_slice(r).map_err(|error| invalid_data(operation, format!("{error}")))?;
    let s = BigNum::from_slice(s).map_err(|error| invalid_data(operation, format!("{error}")))?;
    let signature = EcdsaSig::from_private_components(r, s)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    signature
        .to_der()
        .map_err(|error| invalid_data(operation, format!("{error}")))
}

/// Return whether one persisted host RSA key supports PKCS#1 v1.5 signing.
fn persisted_rsa_sign_is_supported(key: NCRYPT_KEY_HANDLE) -> bool {
    let digest = [0u8; 32];
    let mut pkcs1_padding = BCRYPT_PKCS1_PADDING_INFO {
        pszAlgId: NCRYPT_SHA256_ALGORITHM,
    };
    let mut signature_size = 0u32;
    let status = unsafe {
        NCryptSignHash(
            key,
            (&mut pkcs1_padding as *mut BCRYPT_PKCS1_PADDING_INFO).cast(),
            digest.as_ptr().cast_mut(),
            digest.len() as u32,
            ptr::null_mut(),
            0,
            &mut signature_size,
            NCRYPT_PAD_PKCS1_FLAG,
        )
    };

    status_is_success(status)
}

/// Return whether one persisted host RSA key supports signing after reopen.
fn persisted_rsa_sign_is_supported_after_reopen(
    provider_kind: WindowsProviderKind,
    kind: CryptoStoreKind,
    key_label: &str,
    operation: &'static str,
) -> bool {
    let Ok((provider, key)) =
        open_persisted_key_with_kind(provider_kind, kind, key_label, operation)
    else {
        return false;
    };

    let is_supported = persisted_rsa_sign_is_supported(key);
    close_handle(key);
    close_handle(provider);

    is_supported
}

/// Return whether one persisted host EC key supports ECDSA signing.
fn persisted_ec_sign_is_supported(key: NCRYPT_KEY_HANDLE) -> bool {
    let digest = [0u8; 32];
    let mut signature_size = 0u32;
    let status = unsafe {
        NCryptSignHash(
            key,
            ptr::null(),
            digest.as_ptr().cast_mut(),
            digest.len() as u32,
            ptr::null_mut(),
            0,
            &mut signature_size,
            0,
        )
    };

    status_is_success(status)
}

/// Return whether one persisted host EC key supports signing after reopen.
fn persisted_ec_sign_is_supported_after_reopen(
    provider_kind: WindowsProviderKind,
    kind: CryptoStoreKind,
    key_label: &str,
    operation: &'static str,
) -> bool {
    let Ok((provider, key)) =
        open_persisted_key_with_kind(provider_kind, kind, key_label, operation)
    else {
        return false;
    };

    let is_supported = persisted_ec_sign_is_supported(key);
    close_handle(key);
    close_handle(provider);

    is_supported
}

/// Export one persisted RSA key into one public-key blob.
fn export_rsa_public_blob(
    key: NCRYPT_KEY_HANDLE,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // query one output length before exporting the blob
    let mut blob_size = 0u32;
    let size_status = unsafe {
        NCryptExportKey(
            key,
            0,
            BCRYPT_RSAPUBLIC_BLOB,
            ptr::null(),
            ptr::null_mut(),
            0,
            &mut blob_size,
            0,
        )
    };
    if !status_is_success(size_status) {
        return Err(status_error(
            operation,
            "query one host rsa public blob size",
            size_status,
        ));
    }

    // export one public-key blob into one owned vector
    let mut blob = vec![0u8; blob_size as usize];
    let export_status = unsafe {
        NCryptExportKey(
            key,
            0,
            BCRYPT_RSAPUBLIC_BLOB,
            ptr::null(),
            blob.as_mut_ptr(),
            blob.len() as u32,
            &mut blob_size,
            0,
        )
    };
    if !status_is_success(export_status) {
        return Err(status_error(
            operation,
            "export one host rsa public key",
            export_status,
        ));
    }
    blob.truncate(blob_size as usize);

    Ok(blob)
}

/// Return one default probe size for one hardware-backed secret-key lane.
fn secret_probe_key_size_bits(algorithm: CryptoKeyAlgorithm) -> Option<u32> {
    match algorithm {
        CryptoKeyAlgorithm::Aes | CryptoKeyAlgorithm::Hmac => Some(256),
        _ => None,
    }
}

/// Configure one persisted secret key before finalize.
fn configure_secret_key_properties(
    key: NCRYPT_KEY_HANDLE,
    algorithm: CryptoKeyAlgorithm,
    size_bits: u32,
    operation: &'static str,
) -> RuntimeResult<()> {
    // enforce one non-zero key length for persisted secret keys
    if size_bits == 0 {
        return Err(invalid_data(
            operation,
            "host secret key size must be non-zero",
        ));
    }

    // configure key size and disable export for host-managed lanes
    set_key_u32_property(
        key,
        NCRYPT_LENGTH_PROPERTY,
        size_bits,
        "set one host secret key length",
        operation,
    )?;
    set_key_u32_property(
        key,
        NCRYPT_EXPORT_POLICY_PROPERTY,
        0,
        "set one host secret key export policy",
        operation,
    )?;

    // configure AES keys for CBC chaining mode used by host cipher lanes
    if algorithm == CryptoKeyAlgorithm::Aes {
        set_key_wstring_property(
            key,
            NCRYPT_CHAINING_MODE_PROPERTY,
            BCRYPT_CHAIN_MODE_CBC,
            "set one host aes chaining mode",
            operation,
        )?;
    }

    Ok(())
}

/// Encrypt one payload with one persisted AES key.
fn encrypt_with_persisted_aes_key(
    key: NCRYPT_KEY_HANDLE,
    payload: &[u8],
    nonce: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // enforce one AES-CBC IV length
    if nonce.len() != AES_BLOCK_SIZE_BYTES {
        return Err(core_platform::invalid_argument(
            "parameters.nonce",
            "aes-cbc host key nonce must be 16 bytes",
        ));
    }

    // prepare NCrypt cipher padding metadata with one mutable IV buffer
    let mut iv = nonce.to_vec();
    let mut padding_info = NCRYPT_CIPHER_PADDING_INFO {
        cbSize: size_of::<NCRYPT_CIPHER_PADDING_INFO>() as u32,
        dwFlags: NCRYPT_CIPHER_BLOCK_PADDING_FLAG,
        pbIV: iv.as_mut_ptr(),
        cbIV: iv.len() as u32,
        pbOtherInfo: ptr::null_mut(),
        cbOtherInfo: 0,
    };

    // query ciphertext size with NCrypt symmetric padding metadata
    let mut output_size = 0u32;
    let size_status = unsafe {
        NCryptEncrypt(
            key,
            payload.as_ptr(),
            payload.len() as u32,
            (&mut padding_info as *mut NCRYPT_CIPHER_PADDING_INFO).cast(),
            ptr::null_mut(),
            0,
            &mut output_size,
            NCRYPT_PAD_CIPHER_FLAG,
        )
    };
    if !status_is_success(size_status) {
        return Err(status_error(
            operation,
            "query one host aes ciphertext size",
            size_status,
        ));
    }

    // encrypt payload bytes into one owned output vector
    let mut output = vec![0u8; output_size as usize];
    let encrypt_status = unsafe {
        NCryptEncrypt(
            key,
            payload.as_ptr(),
            payload.len() as u32,
            (&mut padding_info as *mut NCRYPT_CIPHER_PADDING_INFO).cast(),
            output.as_mut_ptr(),
            output.len() as u32,
            &mut output_size,
            NCRYPT_PAD_CIPHER_FLAG,
        )
    };
    if !status_is_success(encrypt_status) {
        return Err(status_error(
            operation,
            "encrypt one payload with one host aes key",
            encrypt_status,
        ));
    }
    output.truncate(output_size as usize);

    Ok(output)
}

/// Decrypt one payload with one persisted AES key.
fn decrypt_with_persisted_aes_key(
    key: NCRYPT_KEY_HANDLE,
    payload: &[u8],
    nonce: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // enforce one AES-CBC IV length
    if nonce.len() != AES_BLOCK_SIZE_BYTES {
        return Err(core_platform::invalid_argument(
            "parameters.nonce",
            "aes-cbc host key nonce must be 16 bytes",
        ));
    }

    // prepare NCrypt cipher padding metadata with one mutable IV buffer
    let mut iv = nonce.to_vec();
    let mut padding_info = NCRYPT_CIPHER_PADDING_INFO {
        cbSize: size_of::<NCRYPT_CIPHER_PADDING_INFO>() as u32,
        dwFlags: NCRYPT_CIPHER_BLOCK_PADDING_FLAG,
        pbIV: iv.as_mut_ptr(),
        cbIV: iv.len() as u32,
        pbOtherInfo: ptr::null_mut(),
        cbOtherInfo: 0,
    };

    // query plaintext size with NCrypt symmetric padding metadata
    let mut output_size = 0u32;
    let size_status = unsafe {
        NCryptDecrypt(
            key,
            payload.as_ptr(),
            payload.len() as u32,
            (&mut padding_info as *mut NCRYPT_CIPHER_PADDING_INFO).cast(),
            ptr::null_mut(),
            0,
            &mut output_size,
            NCRYPT_PAD_CIPHER_FLAG,
        )
    };
    if !status_is_success(size_status) {
        return Err(status_error(
            operation,
            "query one host aes plaintext size",
            size_status,
        ));
    }

    // decrypt payload bytes into one owned output vector
    let mut output = vec![0u8; output_size as usize];
    let decrypt_status = unsafe {
        NCryptDecrypt(
            key,
            payload.as_ptr(),
            payload.len() as u32,
            (&mut padding_info as *mut NCRYPT_CIPHER_PADDING_INFO).cast(),
            output.as_mut_ptr(),
            output.len() as u32,
            &mut output_size,
            NCRYPT_PAD_CIPHER_FLAG,
        )
    };
    if !status_is_success(decrypt_status) {
        return Err(status_error(
            operation,
            "decrypt one payload with one host aes key",
            decrypt_status,
        ));
    }
    output.truncate(output_size as usize);

    Ok(output)
}

/// Compute one HMAC tag with one persisted host key.
fn compute_hmac_with_persisted_key(
    key: NCRYPT_KEY_HANDLE,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // query output size for one HMAC operation
    let mut output_size = 0u32;
    let size_status = unsafe {
        NCryptSignHash(
            key,
            ptr::null(),
            payload.as_ptr().cast_mut(),
            payload.len() as u32,
            ptr::null_mut(),
            0,
            &mut output_size,
            0,
        )
    };
    if !status_is_success(size_status) {
        return Err(status_error(
            operation,
            "query one host hmac tag size",
            size_status,
        ));
    }

    // compute one HMAC tag payload
    let mut output = vec![0u8; output_size as usize];
    let sign_status = unsafe {
        NCryptSignHash(
            key,
            ptr::null(),
            payload.as_ptr().cast_mut(),
            payload.len() as u32,
            output.as_mut_ptr(),
            output.len() as u32,
            &mut output_size,
            0,
        )
    };
    if !status_is_success(sign_status) {
        return Err(status_error(
            operation,
            "compute one host hmac tag",
            sign_status,
        ));
    }
    output.truncate(output_size as usize);

    Ok(output)
}

/// Probe one generated platform AES key by running one encrypt-decrypt roundtrip.
fn probe_generated_platform_aes_key(
    key: NCRYPT_KEY_HANDLE,
    operation: &'static str,
) -> RuntimeResult<()> {
    let plaintext = [0u8; AES_BLOCK_SIZE_BYTES];
    let nonce = [0u8; AES_BLOCK_SIZE_BYTES];
    let ciphertext = encrypt_with_persisted_aes_key(key, &plaintext, &nonce, operation)?;
    let decrypted = decrypt_with_persisted_aes_key(key, &ciphertext, &nonce, operation)?;
    if decrypted != plaintext {
        return Err(invalid_data(
            operation,
            "host aes probe roundtrip produced one mismatched plaintext",
        ));
    }

    Ok(())
}

/// Probe one generated platform HMAC key by computing one MAC tag.
fn probe_generated_platform_hmac_key(
    key: NCRYPT_KEY_HANDLE,
    operation: &'static str,
) -> RuntimeResult<()> {
    let payload = [0x5au8; HMAC_PROBE_PAYLOAD_SIZE_BYTES];
    let tag = compute_hmac_with_persisted_key(key, &payload, operation)?;
    if tag.is_empty() {
        return Err(invalid_data(
            operation,
            "host hmac probe produced one empty tag",
        ));
    }

    Ok(())
}

/// Probe one platform secret-key lane by creating one ephemeral key and running one operation.
fn probe_platform_secret_key_support(kind: CryptoStoreKind, algorithm: CryptoKeyAlgorithm) -> bool {
    // resolve fixed probe metadata for the requested algorithm lane
    let operation = "destack.crypto.store.probeCapability";
    let Some(size_bits) = secret_probe_key_size_bits(algorithm) else {
        return false;
    };
    let digest = if algorithm == CryptoKeyAlgorithm::Hmac {
        CryptoDigestAlgorithm::Sha256
    } else {
        CryptoDigestAlgorithm::Unknown
    };
    let Ok(algorithm_name) = secret_algorithm_name(algorithm, digest, operation) else {
        return false;
    };

    // create and finalize one ephemeral platform key and probe key operations
    let Ok(provider) = open_provider_with_kind(WindowsProviderKind::Platform, operation) else {
        return false;
    };
    let create_flags = key_flags_for_store_kind(kind);
    let mut key = 0usize;
    let probe_result = (|| -> RuntimeResult<()> {
        let create_status = unsafe {
            NCryptCreatePersistedKey(
                provider,
                &mut key,
                algorithm_name,
                ptr::null(),
                0,
                create_flags,
            )
        };
        if !status_is_success(create_status) {
            return Err(status_error(
                operation,
                "create one platform secret-key probe object",
                create_status,
            ));
        }

        configure_secret_key_properties(key, algorithm, size_bits, operation)?;

        let finalize_status = unsafe { NCryptFinalizeKey(key, create_flags) };
        if !status_is_success(finalize_status) {
            return Err(status_error(
                operation,
                "finalize one platform secret-key probe object",
                finalize_status,
            ));
        }

        if algorithm == CryptoKeyAlgorithm::Aes {
            probe_generated_platform_aes_key(key, operation)?;
        } else if algorithm == CryptoKeyAlgorithm::Hmac {
            probe_generated_platform_hmac_key(key, operation)?;
        } else {
            return Err(core_platform::not_supported(operation));
        }

        Ok(())
    })();

    close_handle(key);
    close_handle(provider);

    probe_result.is_ok()
}

/// Return whether one host store lane supports hardware-backed keys.
pub(crate) fn host_store_supports_hardware_backed_key(
    _context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    // hardware-backed lanes are exposed on persistent user and machine stores
    if !matches!(kind, CryptoStoreKind::User | CryptoStoreKind::Machine) {
        return false;
    }

    // probe platform provider availability for this process
    let provider_result = open_provider_with_kind(
        WindowsProviderKind::Platform,
        "destack.crypto.store.probeCapability",
    );
    let Ok(provider) = provider_result else {
        return false;
    };
    close_handle(provider);

    true
}

/// Return whether one host store lane supports one hardware-backed pair algorithm.
pub(crate) fn host_store_supports_hardware_backed_pair_algorithm(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
) -> bool {
    // hardware-backed pair lanes are currently rsa and ec on windows
    if !matches!(algorithm, CryptoKeyAlgorithm::Rsa | CryptoKeyAlgorithm::Ec) {
        return false;
    }

    host_store_supports_hardware_backed_key(context, kind)
}

/// Return whether one host store lane supports hardware-backed secret keys.
pub(crate) fn host_store_supports_hardware_backed_secret_key(
    _context: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
) -> bool {
    // hardware-backed secret lanes are exposed on persistent user and machine stores
    if !matches!(kind, CryptoStoreKind::User | CryptoStoreKind::Machine) {
        return false;
    }

    probe_platform_secret_key_support(kind, algorithm)
}

/// Generate one host-backed hardware key pair.
pub(crate) fn host_generate_hardware_backed_key_pair(
    _context: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<HostGeneratedKeyPair> {
    // hardware-backed lanes are exposed on persistent user and machine stores
    if !matches!(kind, CryptoStoreKind::User | CryptoStoreKind::Machine) {
        return Err(core_platform::not_supported(operation));
    }

    // hardware-backed pair generation currently supports RSA and EC families
    if !matches!(algorithm, CryptoKeyAlgorithm::Rsa | CryptoKeyAlgorithm::Ec) {
        return Err(core_platform::not_supported(operation));
    }

    let generated = generate_persistent_key_pair_with_provider(
        WindowsProviderKind::Platform,
        kind,
        algorithm,
        named_curve,
        CryptoKeyUsageMask(KEY_USAGE_SIGN | KEY_USAGE_VERIFY),
        0,
        0,
        persistent_key_label,
        operation,
    )?;
    let Some(generated) = generated else {
        return Err(core_platform::not_supported(operation));
    };

    Ok(generated)
}

/// Generate one host-backed hardware secret key.
pub(crate) fn host_generate_hardware_backed_secret_key(
    _context: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    digest: CryptoDigestAlgorithm,
    size_bits: u32,
    usage_mask: CryptoKeyUsageMask,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<HostKeyMaterial> {
    // hardware-backed secret lanes are exposed on persistent user and machine stores
    if !matches!(kind, CryptoStoreKind::User | CryptoStoreKind::Machine) {
        return Err(core_platform::not_supported(operation));
    }

    // resolve backend and CNG algorithm selectors for the requested lane
    let Some(backend) = secret_backend_for_algorithm(algorithm) else {
        return Err(core_platform::not_supported(operation));
    };
    let algorithm_name = secret_algorithm_name(algorithm, digest, operation)?;

    let _ = usage_mask;

    // create one persisted platform secret key and enforce non-exportability
    let mut provider = 0usize;
    let mut key = 0usize;
    let mut should_delete_key = false;
    let result = (|| -> RuntimeResult<HostKeyMaterial> {
        provider = open_provider_with_kind(WindowsProviderKind::Platform, operation)?;
        let key_name = key_name_utf16(persistent_key_label, operation)?;
        let create_flags = key_flags_for_store_kind(kind) | NCRYPT_OVERWRITE_KEY_FLAG;

        let create_status = unsafe {
            NCryptCreatePersistedKey(
                provider,
                &mut key,
                algorithm_name,
                key_name.as_ptr(),
                0,
                create_flags,
            )
        };
        if !status_is_success(create_status) {
            return Err(status_error(
                operation,
                "create one persisted host secret key",
                create_status,
            ));
        }
        should_delete_key = true;

        configure_secret_key_properties(key, algorithm, size_bits, operation)?;

        let finalize_status = unsafe { NCryptFinalizeKey(key, create_flags) };
        if !status_is_success(finalize_status) {
            return Err(status_error(
                operation,
                "finalize one persisted host secret key",
                finalize_status,
            ));
        }

        if algorithm == CryptoKeyAlgorithm::Aes {
            probe_generated_platform_aes_key(key, operation)?;
        } else if algorithm == CryptoKeyAlgorithm::Hmac {
            probe_generated_platform_hmac_key(key, operation)?;
        } else {
            return Err(core_platform::not_supported(operation));
        }

        should_delete_key = false;
        Ok(HostKeyMaterial {
            backend,
            key_label: persistent_key_label.to_string(),
            public_key_spki_der: Vec::new(),
            private_key_der: Vec::new(),
        })
    })();

    // roll back partially-created persisted keys when generation failed
    if should_delete_key && key != 0 {
        unsafe {
            let _ = NCryptDeleteKey(key, 0);
        }
        key = 0;
    }

    close_handle(key);
    close_handle(provider);

    result
}

/// Generate one host-managed persistent key pair when available.
pub(crate) fn host_generate_persistent_key_pair(
    _context: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    usage_mask: CryptoKeyUsageMask,
    modulus_bits: u32,
    public_exponent: u32,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<Option<HostGeneratedKeyPair>> {
    generate_persistent_key_pair_with_provider(
        WindowsProviderKind::Software,
        kind,
        algorithm,
        named_curve,
        usage_mask,
        modulus_bits,
        public_exponent,
        persistent_key_label,
        operation,
    )
}

/// Import one persistent host-managed private key when available.
pub(crate) fn host_import_persistent_private_key(
    _context: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    usage_mask: CryptoKeyUsageMask,
    private_key: &PKey<Private>,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<Option<HostKeyMaterial>> {
    // host-managed persistent imports currently target user and machine lanes
    if !matches!(kind, CryptoStoreKind::User | CryptoStoreKind::Machine) {
        return Ok(None);
    }

    // enforce one stable persistent key label
    if persistent_key_label.is_empty() {
        return Err(invalid_data(
            operation,
            "persistent key label must not be empty",
        ));
    }

    // this backend supports persistent import for rsa and ec key families
    if !matches!(algorithm, CryptoKeyAlgorithm::Rsa | CryptoKeyAlgorithm::Ec) {
        return Ok(None);
    }

    // resolve one PKCS#8 payload from the supplied provider key
    let private_key_pkcs8_der = private_key
        .private_key_to_der()
        .map_err(|error| invalid_data(operation, format!("{error}")))?;

    // choose EC backend mode from usage lanes, and reject mixed derive and sign requests
    let requests_derive = usage_requests_derive(usage_mask);
    let requests_sign = usage_requests_sign(usage_mask);
    if algorithm == CryptoKeyAlgorithm::Ec && requests_derive && requests_sign {
        return Ok(None);
    }

    // import and persist one host private key
    let provider = open_provider_with_kind(WindowsProviderKind::Software, operation)?;
    let key_result = import_persistent_private_key_from_pkcs8(
        provider,
        kind,
        persistent_key_label,
        &private_key_pkcs8_der,
        operation,
    );
    let key = match key_result {
        Ok(key) => key,
        Err(error) => {
            close_handle(provider);
            return Err(error);
        }
    };

    // export public key material and translate into one host descriptor payload
    let host_material = (|| -> RuntimeResult<Option<HostKeyMaterial>> {
        if algorithm == CryptoKeyAlgorithm::Rsa {
            if named_curve != CryptoNamedCurve::Unknown {
                return Ok(None);
            }

            let public_blob = export_rsa_public_blob(key, operation)?;
            let (public_key, _, _) = parse_rsa_public_blob(&public_blob, operation)?;
            let public_key_spki_der = public_key
                .public_key_to_der()
                .map_err(|error| invalid_data(operation, format!("{error}")))?;
            return Ok(Some(HostKeyMaterial {
                backend: HostKeyBackend::WindowsSoftwareKeyStorageRsa,
                key_label: persistent_key_label.to_string(),
                public_key_spki_der,
                private_key_der: Vec::new(),
            }));
        }

        let Some((resolved_curve, _, _)) = crypto_core::resolve_nist_p_curve(named_curve) else {
            return Ok(None);
        };
        let public_blob = export_ec_public_blob(key, operation)?;
        let (public_key, public_curve, _) = parse_ec_public_blob(&public_blob, operation)?;
        if public_curve != resolved_curve {
            return Ok(None);
        }
        let public_key_spki_der = public_key
            .public_key_to_der()
            .map_err(|error| invalid_data(operation, format!("{error}")))?;

        Ok(Some(HostKeyMaterial {
            backend: HostKeyBackend::WindowsSoftwareKeyStorageEc,
            key_label: persistent_key_label.to_string(),
            public_key_spki_der,
            private_key_der: Vec::new(),
        }))
    })();

    close_handle(key);
    close_handle(provider);

    host_material
}

/// Sign one payload with one host-managed key.
pub(crate) fn host_key_sign(
    _context: &BindingCallContext,
    key: &HostKeyMaterial,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoSignatureParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // enforce one supported windows host-key backend lane
    if provider_kind_from_backend(key.backend).is_none() {
        return Err(core_platform::not_supported(operation));
    }

    // resolve digest metadata and hash the payload before signing
    let (message_digest, digest_algorithm) =
        signature_digest_metadata(parameters.digest, operation)?;
    let hashed_payload = hash(message_digest, payload)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;

    // open one persisted key and call NCryptSignHash in two passes
    let (provider, persisted_key) =
        open_persisted_key_for_backend(key.backend, kind, &key.key_label, operation)?;
    let sign_result = (|| -> RuntimeResult<Vec<u8>> {
        // rsa signing
        if algorithm == CryptoKeyAlgorithm::Rsa {
            let mut pkcs1_padding = BCRYPT_PKCS1_PADDING_INFO {
                pszAlgId: digest_algorithm,
            };
            let mut pss_padding = BCRYPT_PSS_PADDING_INFO {
                pszAlgId: digest_algorithm,
                cbSalt: if parameters.salt_length_bytes == 0 {
                    message_digest.size() as u32
                } else {
                    parameters.salt_length_bytes
                },
            };
            let (padding_info, flags): (*const c_void, NCRYPT_FLAGS) = match parameters.algorithm {
                CryptoSignatureAlgorithm::RsaPkcs1v15 => (
                    (&mut pkcs1_padding as *mut BCRYPT_PKCS1_PADDING_INFO).cast(),
                    NCRYPT_PAD_PKCS1_FLAG,
                ),
                CryptoSignatureAlgorithm::RsaPss => (
                    (&mut pss_padding as *mut BCRYPT_PSS_PADDING_INFO).cast(),
                    NCRYPT_PAD_PSS_FLAG,
                ),
                _ => return Err(core_platform::not_supported(operation)),
            };

            let mut signature_size = 0u32;
            let size_status = unsafe {
                NCryptSignHash(
                    persisted_key,
                    padding_info,
                    hashed_payload.as_ptr(),
                    hashed_payload.len() as u32,
                    ptr::null_mut(),
                    0,
                    &mut signature_size,
                    flags,
                )
            };
            if !status_is_success(size_status) {
                return Err(status_error(
                    operation,
                    "query one host signature size",
                    size_status,
                ));
            }

            let mut signature = vec![0u8; signature_size as usize];
            let sign_status = unsafe {
                NCryptSignHash(
                    persisted_key,
                    padding_info,
                    hashed_payload.as_ptr(),
                    hashed_payload.len() as u32,
                    signature.as_mut_ptr(),
                    signature.len() as u32,
                    &mut signature_size,
                    flags,
                )
            };
            if !status_is_success(sign_status) {
                return Err(status_error(
                    operation,
                    "sign one payload with one host key",
                    sign_status,
                ));
            }
            signature.truncate(signature_size as usize);

            return Ok(signature);
        }

        // ec signing
        if algorithm == CryptoKeyAlgorithm::Ec {
            if parameters.algorithm != CryptoSignatureAlgorithm::Ecdsa {
                return Err(core_platform::not_supported(operation));
            }

            let mut signature_size = 0u32;
            let size_status = unsafe {
                NCryptSignHash(
                    persisted_key,
                    ptr::null(),
                    hashed_payload.as_ptr().cast_mut(),
                    hashed_payload.len() as u32,
                    ptr::null_mut(),
                    0,
                    &mut signature_size,
                    0,
                )
            };
            if !status_is_success(size_status) {
                return Err(status_error(
                    operation,
                    "query one host ecdsa signature size",
                    size_status,
                ));
            }

            let mut signature = vec![0u8; signature_size as usize];
            let sign_status = unsafe {
                NCryptSignHash(
                    persisted_key,
                    ptr::null(),
                    hashed_payload.as_ptr().cast_mut(),
                    hashed_payload.len() as u32,
                    signature.as_mut_ptr(),
                    signature.len() as u32,
                    &mut signature_size,
                    0,
                )
            };
            if !status_is_success(sign_status) {
                return Err(status_error(
                    operation,
                    "sign one payload with one host ec key",
                    sign_status,
                ));
            }
            signature.truncate(signature_size as usize);

            let signature = ecdsa_signature_der_from_raw(&signature, operation)?;
            return Ok(signature);
        }

        Err(core_platform::not_supported(operation))
    })();

    // release both key and provider handles before returning
    close_handle(persisted_key);
    close_handle(provider);

    sign_result
}

/// Decrypt one payload with one host-managed key.
pub(crate) fn host_key_decrypt(
    _context: &BindingCallContext,
    key: &HostKeyMaterial,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoAsymmetricEncryptionParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // enforce one supported host key backend and key family
    if !matches!(
        key.backend,
        HostKeyBackend::WindowsSoftwareKeyStorageRsa | HostKeyBackend::WindowsPlatformKeyStorageRsa
    ) || algorithm != CryptoKeyAlgorithm::Rsa
    {
        return Err(core_platform::not_supported(operation));
    }

    // decode the optional OAEP label now so pointers stay valid for CNG calls
    let mut oaep_label = crypto_core::decode_native_bytes(parameters.label, "parameters.label")?;
    let mut oaep_padding = BCRYPT_OAEP_PADDING_INFO {
        pszAlgId: ptr::null(),
        pbLabel: ptr::null_mut(),
        cbLabel: 0,
    };
    let (padding_info, flags): (*const c_void, NCRYPT_FLAGS) = match parameters.algorithm {
        CryptoAsymmetricEncryptionAlgorithm::RsaPkcs1v15 => (ptr::null(), NCRYPT_PAD_PKCS1_FLAG),
        CryptoAsymmetricEncryptionAlgorithm::RsaOaep => {
            let (_, digest_algorithm) = signature_digest_metadata(parameters.digest, operation)?;
            oaep_padding.pszAlgId = digest_algorithm;
            oaep_padding.cbLabel = oaep_label.len() as u32;
            oaep_padding.pbLabel = if oaep_label.is_empty() {
                ptr::null_mut()
            } else {
                oaep_label.as_mut_ptr()
            };

            (
                (&mut oaep_padding as *mut BCRYPT_OAEP_PADDING_INFO).cast(),
                NCRYPT_PAD_OAEP_FLAG,
            )
        }
        _ => return Err(core_platform::not_supported(operation)),
    };

    // open one persisted key and call NCryptDecrypt in two passes
    let (provider, persisted_key) =
        open_persisted_key_for_backend(key.backend, kind, &key.key_label, operation)?;
    let decrypt_result = (|| -> RuntimeResult<Vec<u8>> {
        let mut plaintext_size = 0u32;
        let size_status = unsafe {
            NCryptDecrypt(
                persisted_key,
                payload.as_ptr(),
                payload.len() as u32,
                padding_info,
                ptr::null_mut(),
                0,
                &mut plaintext_size,
                flags,
            )
        };
        if !status_is_success(size_status) {
            return Err(status_error(
                operation,
                "query one host decrypt output size",
                size_status,
            ));
        }

        let mut plaintext = vec![0u8; plaintext_size as usize];
        let decrypt_status = unsafe {
            NCryptDecrypt(
                persisted_key,
                payload.as_ptr(),
                payload.len() as u32,
                padding_info,
                plaintext.as_mut_ptr(),
                plaintext.len() as u32,
                &mut plaintext_size,
                flags,
            )
        };
        if !status_is_success(decrypt_status) {
            return Err(status_error(
                operation,
                "decrypt one payload with one host key",
                decrypt_status,
            ));
        }
        plaintext.truncate(plaintext_size as usize);

        Ok(plaintext)
    })();

    // release both key and provider handles before returning
    close_handle(persisted_key);
    close_handle(provider);

    decrypt_result
}

/// Delete one host-managed key.
pub(crate) fn host_key_delete(
    _context: &BindingCallContext,
    key: &HostKeyMaterial,
    kind: CryptoStoreKind,
    operation: &'static str,
) -> RuntimeResult<()> {
    // enforce one supported windows host-key backend lane
    let Some(provider_kind) = provider_kind_from_backend(key.backend) else {
        return Err(core_platform::not_supported(operation));
    };
    if kind != CryptoStoreKind::User && kind != CryptoStoreKind::Machine {
        return Err(core_platform::not_supported(operation));
    }

    // open one persisted key with explicit store provenance
    let selected_key =
        try_open_persisted_key_with_kind(provider_kind, kind, &key.key_label, operation)?;
    let Some((provider, mut persisted_key)) = selected_key else {
        return Ok(());
    };

    // delete the resolved persisted key and ignore already-deleted outcomes
    let delete_status = unsafe { NCryptDeleteKey(persisted_key, 0) };
    if status_is_success(delete_status) || status_is_key_not_found(delete_status) {
        persisted_key = 0;
    }
    close_handle(persisted_key);
    close_handle(provider);
    if status_is_success(delete_status) || status_is_key_not_found(delete_status) {
        return Ok(());
    }

    Err(status_error(
        operation,
        "delete one persisted host key",
        delete_status,
    ))
}

/// Derive one shared secret with one host-managed private key.
pub(crate) fn host_key_derive_shared_secret(
    _context: &BindingCallContext,
    key: &HostKeyMaterial,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    peer_public_spki_der: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // enforce one supported windows host-key backend lane and EC family
    if !matches!(
        key.backend,
        HostKeyBackend::WindowsSoftwareKeyStorageEc | HostKeyBackend::WindowsPlatformKeyStorageEc
    ) || algorithm != CryptoKeyAlgorithm::Ec
    {
        return Err(core_platform::not_supported(operation));
    }
    if crypto_core::resolve_nist_p_curve(named_curve).is_none() {
        return Err(core_platform::not_supported(operation));
    }

    // open one persisted host private key and import peer public key
    let (provider, private_key) =
        open_persisted_key_for_backend(key.backend, kind, &key.key_label, operation)?;
    let derive_result = (|| -> RuntimeResult<Vec<u8>> {
        let peer_public_blob =
            encode_ecdh_public_blob_from_spki(peer_public_spki_der, named_curve, operation)?;
        let mut peer_public_key = 0usize;
        let import_status = unsafe {
            NCryptImportKey(
                provider,
                0,
                BCRYPT_ECCPUBLIC_BLOB,
                ptr::null(),
                &mut peer_public_key,
                peer_public_blob.as_ptr(),
                peer_public_blob.len() as u32,
                0,
            )
        };
        if !status_is_success(import_status) {
            return Err(status_error(
                operation,
                "import one peer ec public key",
                import_status,
            ));
        }

        // derive one agreed secret handle and then derive raw bytes
        let mut agreed_secret: NCRYPT_SECRET_HANDLE = 0;
        let agreement_status =
            unsafe { NCryptSecretAgreement(private_key, peer_public_key, &mut agreed_secret, 0) };
        if !status_is_success(agreement_status) {
            close_handle(peer_public_key);
            return Err(status_error(
                operation,
                "derive one host shared secret agreement",
                agreement_status,
            ));
        }

        let mut shared_size = 0u32;
        let shared_size_status = unsafe {
            NCryptDeriveKey(
                agreed_secret,
                BCRYPT_KDF_RAW_SECRET,
                ptr::null(),
                ptr::null_mut(),
                0,
                &mut shared_size,
                0,
            )
        };
        if !status_is_success(shared_size_status) {
            close_handle(agreed_secret);
            close_handle(peer_public_key);
            return Err(status_error(
                operation,
                "query one host shared secret size",
                shared_size_status,
            ));
        }

        let mut shared_secret = vec![0u8; shared_size as usize];
        let derive_status = unsafe {
            NCryptDeriveKey(
                agreed_secret,
                BCRYPT_KDF_RAW_SECRET,
                ptr::null(),
                shared_secret.as_mut_ptr(),
                shared_secret.len() as u32,
                &mut shared_size,
                0,
            )
        };
        close_handle(agreed_secret);
        close_handle(peer_public_key);
        if !status_is_success(derive_status) {
            return Err(status_error(
                operation,
                "derive one host shared secret bytes",
                derive_status,
            ));
        }
        shared_secret.truncate(shared_size as usize);

        Ok(shared_secret)
    })();

    close_handle(private_key);
    close_handle(provider);

    derive_result
}

/// Encrypt one payload with one host-managed secret key.
pub(crate) fn host_key_cipher_encrypt(
    _context: &BindingCallContext,
    key: &HostKeyMaterial,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoCipherParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<(Vec<u8>, Vec<u8>)> {
    // enforce one supported windows host secret-key lane
    if key.backend != HostKeyBackend::WindowsPlatformKeyStorageAes
        || algorithm != CryptoKeyAlgorithm::Aes
        || !matches!(kind, CryptoStoreKind::User | CryptoStoreKind::Machine)
    {
        return Err(core_platform::not_supported(operation));
    }

    // enforce AES-CBC lane semantics for this backend
    if parameters.algorithm != CryptoCipherAlgorithm::AesCbc {
        return Err(core_platform::not_supported(operation));
    }
    if parameters.tag_length_bytes != 0 {
        return Err(core_platform::invalid_argument(
            "parameters.tagLengthBytes",
            "host aes-cbc does not produce authentication tags",
        ));
    }
    if parameters.tag.len != 0 {
        return Err(core_platform::invalid_argument(
            "parameters.tag",
            "host aes-cbc requires one empty decryption tag",
        ));
    }
    if parameters.additional_data.len != 0 {
        return Err(core_platform::invalid_argument(
            "parameters.additionalData",
            "host aes-cbc does not support additional authenticated data",
        ));
    }

    // decode nonce and encrypt payload with one persisted host key
    let nonce = crypto_core::decode_native_bytes(parameters.nonce, "parameters.nonce")?;
    let (provider, persisted_key) =
        open_persisted_key_for_backend(key.backend, kind, &key.key_label, operation)?;
    let result = encrypt_with_persisted_aes_key(persisted_key, payload, &nonce, operation);

    close_handle(persisted_key);
    close_handle(provider);

    let ciphertext = result?;

    Ok((ciphertext, Vec::new()))
}

/// Decrypt one payload with one host-managed secret key.
pub(crate) fn host_key_cipher_decrypt(
    _context: &BindingCallContext,
    key: &HostKeyMaterial,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoCipherParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // enforce one supported windows host secret-key lane
    if key.backend != HostKeyBackend::WindowsPlatformKeyStorageAes
        || algorithm != CryptoKeyAlgorithm::Aes
        || !matches!(kind, CryptoStoreKind::User | CryptoStoreKind::Machine)
    {
        return Err(core_platform::not_supported(operation));
    }

    // enforce AES-CBC lane semantics for this backend
    if parameters.algorithm != CryptoCipherAlgorithm::AesCbc {
        return Err(core_platform::not_supported(operation));
    }
    if parameters.tag_length_bytes != 0 {
        return Err(core_platform::invalid_argument(
            "parameters.tagLengthBytes",
            "host aes-cbc does not consume authentication tag-length selectors",
        ));
    }
    if parameters.tag.len != 0 {
        return Err(core_platform::invalid_argument(
            "parameters.tag",
            "host aes-cbc requires one empty decryption tag",
        ));
    }
    if parameters.additional_data.len != 0 {
        return Err(core_platform::invalid_argument(
            "parameters.additionalData",
            "host aes-cbc does not support additional authenticated data",
        ));
    }

    // decode nonce and decrypt payload with one persisted host key
    let nonce = crypto_core::decode_native_bytes(parameters.nonce, "parameters.nonce")?;
    let (provider, persisted_key) =
        open_persisted_key_for_backend(key.backend, kind, &key.key_label, operation)?;
    let result = decrypt_with_persisted_aes_key(persisted_key, payload, &nonce, operation);

    close_handle(persisted_key);
    close_handle(provider);

    result
}

/// Compute one MAC with one host-managed secret key.
pub(crate) fn host_key_mac_compute(
    _context: &BindingCallContext,
    key: &HostKeyMaterial,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoMacParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // enforce one supported windows host secret-key lane
    if key.backend != HostKeyBackend::WindowsPlatformKeyStorageHmac
        || algorithm != CryptoKeyAlgorithm::Hmac
        || !matches!(kind, CryptoStoreKind::User | CryptoStoreKind::Machine)
    {
        return Err(core_platform::not_supported(operation));
    }

    // enforce HMAC-SHA256 lane semantics for this backend
    if parameters.algorithm != CryptoMacAlgorithm::Hmac {
        return Err(core_platform::not_supported(operation));
    }
    if parameters.digest != CryptoDigestAlgorithm::Unknown
        && parameters.digest != CryptoDigestAlgorithm::Sha256
    {
        return Err(core_platform::not_supported(operation));
    }

    // compute tag bytes from one persisted host key
    let (provider, persisted_key) =
        open_persisted_key_for_backend(key.backend, kind, &key.key_label, operation)?;
    let result = compute_hmac_with_persisted_key(persisted_key, payload, operation);

    close_handle(persisted_key);
    close_handle(provider);

    let mut tag = result?;
    if parameters.tag_length_bytes != 0 {
        let length = parameters.tag_length_bytes as usize;
        if length > tag.len() {
            return Err(core_platform::invalid_argument(
                "parameters.tagLengthBytes",
                "tag length must be at most host hmac output size",
            ));
        }
        tag.truncate(length);
    }

    Ok(tag)
}
