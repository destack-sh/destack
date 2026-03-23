use crate::host::android::abi::crypto::callbacks::AndroidHostCryptoCallbacks;
use crate::host::{HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK};
use crate::runtime::{NativeSlice, NativeStringRef};

/// Report one supported hardware lane in callback tests.
unsafe extern "C" fn test_supports_hardware_key(_runtime_id: u64, _store_kind: u32) -> u32 {
    HOST_STATUS_OK
}

/// Export one fixed payload for callback tests.
unsafe extern "C" fn test_export_hardware_public_key(
    _runtime_id: u64,
    _key_algorithm: u32,
    _key_label: NativeStringRef,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    let payload = [1u8, 2, 3, 4];
    if output_written.is_null() {
        return HOST_STATUS_NOT_SUPPORTED;
    }

    // report required size when the caller has no writable buffer
    if output.data.is_null() || output.len < payload.len() as u32 {
        unsafe {
            *output_written = payload.len() as u32;
        }

        return HOST_STATUS_BUFFER_TOO_SMALL;
    }

    // copy the payload into the output buffer
    let output_bytes = unsafe { std::slice::from_raw_parts_mut(output.data, output.len as usize) };
    output_bytes[..payload.len()].copy_from_slice(&payload);

    // publish final output length
    unsafe {
        *output_written = payload.len() as u32;
    }

    HOST_STATUS_OK
}

/// Report one successful hardware-secret generation in callback tests.
unsafe extern "C" fn test_generate_hardware_secret_key(
    _runtime_id: u64,
    _store_kind: u32,
    _key_algorithm: u32,
    _digest_algorithm: u32,
    _key_size_bits: u32,
    _key_usage_mask: u32,
    _key_label: NativeStringRef,
) -> u32 {
    HOST_STATUS_OK
}

/// Report one successful hardware-keypair generation in callback tests.
unsafe extern "C" fn test_generate_hardware_key_pair(
    _runtime_id: u64,
    _store_kind: u32,
    _key_algorithm: u32,
    _named_curve: u32,
    _modulus_bits: u32,
    _public_exponent: u32,
    _key_label: NativeStringRef,
) -> u32 {
    HOST_STATUS_OK
}

/// Report one successful hardware signature operation in callback tests.
unsafe extern "C" fn test_sign_hardware_key(
    _runtime_id: u64,
    _key_algorithm: u32,
    _key_label: NativeStringRef,
    _signature_algorithm: u32,
    _digest_algorithm: u32,
    _salt_length_bytes: u32,
    _payload: NativeSlice<u8>,
    _output_signature: NativeSlice<u8>,
    _output_written: *mut u32,
) -> u32 {
    HOST_STATUS_OK
}

/// Report one successful hardware decrypt operation in callback tests.
unsafe extern "C" fn test_decrypt_hardware_key(
    _runtime_id: u64,
    _key_algorithm: u32,
    _key_label: NativeStringRef,
    _encryption_algorithm: u32,
    _digest_algorithm: u32,
    _label: NativeSlice<u8>,
    _payload: NativeSlice<u8>,
    _output_plaintext: NativeSlice<u8>,
    _output_written: *mut u32,
) -> u32 {
    HOST_STATUS_OK
}

/// Report one successful hardware key delete operation in callback tests.
unsafe extern "C" fn test_delete_hardware_key(
    _runtime_id: u64,
    _key_algorithm: u32,
    _key_label: NativeStringRef,
) -> u32 {
    HOST_STATUS_OK
}

/// Export one fixed ciphertext and tag payload for callback tests.
unsafe extern "C" fn test_encrypt_hardware_secret_key(
    _runtime_id: u64,
    _key_algorithm: u32,
    _key_label: NativeStringRef,
    _cipher_algorithm: u32,
    _nonce: NativeSlice<u8>,
    _additional_data: NativeSlice<u8>,
    _tag_length_bytes: u32,
    _payload: NativeSlice<u8>,
    output_ciphertext: NativeSlice<u8>,
    output_tag: NativeSlice<u8>,
    output_ciphertext_written: *mut u32,
    output_tag_written: *mut u32,
) -> u32 {
    let ciphertext = [7u8, 8, 9];
    let tag = [1u8, 2];
    if output_ciphertext_written.is_null() || output_tag_written.is_null() {
        return HOST_STATUS_NOT_SUPPORTED;
    }

    // report required sizes when caller has no writable buffers
    if output_ciphertext.data.is_null()
        || output_ciphertext.len < ciphertext.len() as u32
        || output_tag.data.is_null()
        || output_tag.len < tag.len() as u32
    {
        unsafe {
            *output_ciphertext_written = ciphertext.len() as u32;
            *output_tag_written = tag.len() as u32;
        }

        return HOST_STATUS_BUFFER_TOO_SMALL;
    }

    // copy ciphertext and tag payloads into output buffers
    let output_ciphertext_bytes = unsafe {
        std::slice::from_raw_parts_mut(output_ciphertext.data, output_ciphertext.len as usize)
    };
    let output_tag_bytes =
        unsafe { std::slice::from_raw_parts_mut(output_tag.data, output_tag.len as usize) };
    output_ciphertext_bytes[..ciphertext.len()].copy_from_slice(&ciphertext);
    output_tag_bytes[..tag.len()].copy_from_slice(&tag);

    // publish final output lengths
    unsafe {
        *output_ciphertext_written = ciphertext.len() as u32;
        *output_tag_written = tag.len() as u32;
    }

    HOST_STATUS_OK
}

/// Export one fixed plaintext payload for callback tests.
unsafe extern "C" fn test_decrypt_hardware_secret_key(
    _runtime_id: u64,
    _key_algorithm: u32,
    _key_label: NativeStringRef,
    _cipher_algorithm: u32,
    _nonce: NativeSlice<u8>,
    _additional_data: NativeSlice<u8>,
    _tag: NativeSlice<u8>,
    _payload: NativeSlice<u8>,
    output_plaintext: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    let plaintext = [3u8, 4, 5, 6];
    if output_written.is_null() {
        return HOST_STATUS_NOT_SUPPORTED;
    }

    // report required size when caller has no writable buffer
    if output_plaintext.data.is_null() || output_plaintext.len < plaintext.len() as u32 {
        unsafe {
            *output_written = plaintext.len() as u32;
        }

        return HOST_STATUS_BUFFER_TOO_SMALL;
    }

    // copy plaintext payload into output buffer
    let output_plaintext_bytes = unsafe {
        std::slice::from_raw_parts_mut(output_plaintext.data, output_plaintext.len as usize)
    };
    output_plaintext_bytes[..plaintext.len()].copy_from_slice(&plaintext);

    // publish final output length
    unsafe {
        *output_written = plaintext.len() as u32;
    }

    HOST_STATUS_OK
}

/// Export one fixed MAC payload for callback tests.
unsafe extern "C" fn test_compute_hardware_mac(
    _runtime_id: u64,
    _key_algorithm: u32,
    _key_label: NativeStringRef,
    _mac_algorithm: u32,
    _digest_algorithm: u32,
    _tag_length_bytes: u32,
    _payload: NativeSlice<u8>,
    output_tag: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    let tag = [9u8, 9, 9];
    if output_written.is_null() {
        return HOST_STATUS_NOT_SUPPORTED;
    }

    // report required size when caller has no writable buffer
    if output_tag.data.is_null() || output_tag.len < tag.len() as u32 {
        unsafe {
            *output_written = tag.len() as u32;
        }

        return HOST_STATUS_BUFFER_TOO_SMALL;
    }

    // copy tag payload into output buffer
    let output_tag_bytes =
        unsafe { std::slice::from_raw_parts_mut(output_tag.data, output_tag.len as usize) };
    output_tag_bytes[..tag.len()].copy_from_slice(&tag);

    // publish final output length
    unsafe {
        *output_written = tag.len() as u32;
    }

    HOST_STATUS_OK
}

/// Report one successful certificate import in callback tests.
unsafe extern "C" fn test_import_certificate(
    _runtime_id: u64,
    _store_kind: u32,
    _certificate_der: NativeSlice<u8>,
) -> u32 {
    HOST_STATUS_OK
}

/// Report one successful certificate delete in callback tests.
unsafe extern "C" fn test_delete_certificate(
    _runtime_id: u64,
    _store_kind: u32,
    _certificate_der: NativeSlice<u8>,
) -> u32 {
    HOST_STATUS_OK
}

/// Return one default callback table for Android crypto tests.
pub(super) fn default_callbacks() -> AndroidHostCryptoCallbacks {
    AndroidHostCryptoCallbacks::default()
}

/// Return one callback table with support and export handlers.
pub(super) fn probe_callbacks() -> AndroidHostCryptoCallbacks {
    AndroidHostCryptoCallbacks {
        supports_hardware_key: Some(test_supports_hardware_key),
        export_hardware_public_key: Some(test_export_hardware_public_key),
        ..AndroidHostCryptoCallbacks::default()
    }
}

/// Return one callback table with complete pair and secret probes.
pub(super) fn complete_support_callbacks() -> AndroidHostCryptoCallbacks {
    AndroidHostCryptoCallbacks {
        supports_hardware_key: Some(test_supports_hardware_key),
        generate_hardware_key_pair: Some(test_generate_hardware_key_pair),
        export_hardware_public_key: Some(test_export_hardware_public_key),
        sign_hardware_key: Some(test_sign_hardware_key),
        decrypt_hardware_key: Some(test_decrypt_hardware_key),
        generate_hardware_secret_key: Some(test_generate_hardware_secret_key),
        encrypt_hardware_secret_key: Some(test_encrypt_hardware_secret_key),
        decrypt_hardware_secret_key: Some(test_decrypt_hardware_secret_key),
        delete_hardware_key: Some(test_delete_hardware_key),
        ..AndroidHostCryptoCallbacks::default()
    }
}

/// Return one callback table with secret-generation support.
pub(super) fn secret_generation_callbacks() -> AndroidHostCryptoCallbacks {
    AndroidHostCryptoCallbacks {
        generate_hardware_secret_key: Some(test_generate_hardware_secret_key),
        ..AndroidHostCryptoCallbacks::default()
    }
}

/// Return one callback table with secret-encryption support.
pub(super) fn secret_encryption_callbacks() -> AndroidHostCryptoCallbacks {
    AndroidHostCryptoCallbacks {
        encrypt_hardware_secret_key: Some(test_encrypt_hardware_secret_key),
        ..AndroidHostCryptoCallbacks::default()
    }
}

/// Return one callback table with secret-decryption support.
pub(super) fn secret_decryption_callbacks() -> AndroidHostCryptoCallbacks {
    AndroidHostCryptoCallbacks {
        decrypt_hardware_secret_key: Some(test_decrypt_hardware_secret_key),
        ..AndroidHostCryptoCallbacks::default()
    }
}

/// Return one callback table with hardware-MAC support.
pub(super) fn mac_callbacks() -> AndroidHostCryptoCallbacks {
    AndroidHostCryptoCallbacks {
        compute_hardware_mac: Some(test_compute_hardware_mac),
        ..AndroidHostCryptoCallbacks::default()
    }
}

/// Return one callback table with certificate import and delete support.
pub(super) fn certificate_callbacks() -> AndroidHostCryptoCallbacks {
    AndroidHostCryptoCallbacks {
        import_certificate: Some(test_import_certificate),
        delete_certificate: Some(test_delete_certificate),
        ..AndroidHostCryptoCallbacks::default()
    }
}

/// Return one callback table with only certificate import support.
pub(super) fn partial_certificate_callbacks() -> AndroidHostCryptoCallbacks {
    AndroidHostCryptoCallbacks {
        import_certificate: Some(test_import_certificate),
        ..AndroidHostCryptoCallbacks::default()
    }
}

/// Return one callback table with only the base support lane.
pub(super) fn support_only_callbacks() -> AndroidHostCryptoCallbacks {
    AndroidHostCryptoCallbacks {
        supports_hardware_key: Some(test_supports_hardware_key),
        ..AndroidHostCryptoCallbacks::default()
    }
}
