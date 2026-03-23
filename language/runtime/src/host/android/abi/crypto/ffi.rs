use crate::host::HOST_STATUS_OK;
use crate::host::android::abi::crypto::core::{
    call_android_crypto_callback, has_hardware_key_pair_lane, has_hardware_secret_key_lane,
    probe_hardware_support, resolve_android_crypto_callbacks, supports_certificate_write,
    unsupported_lane_status,
};
use crate::runtime::{NativeSlice, NativeStringRef};

/// Probe one Android host lane for hardware-backed key support.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_crypto_supports_hardware_key(
    runtime_id: u64,
    store_kind: u32,
) -> u32 {
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.supports_hardware_key,
        |callback| unsafe { callback(runtime_id, store_kind) },
    )
}

/// Probe one Android host lane for one hardware-backed key-pair algorithm.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_crypto_supports_hardware_key_pair(
    runtime_id: u64,
    store_kind: u32,
    key_algorithm: u32,
) -> u32 {
    let callbacks = match resolve_android_crypto_callbacks(runtime_id) {
        Ok(callbacks) => callbacks,
        Err(status) => return status,
    };

    if !has_hardware_key_pair_lane(&callbacks, key_algorithm) {
        return unsupported_lane_status();
    }

    probe_hardware_support(runtime_id, store_kind, callbacks)
}

/// Probe one Android host lane for one hardware-backed secret-key algorithm.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_crypto_supports_hardware_secret_key(
    runtime_id: u64,
    store_kind: u32,
    key_algorithm: u32,
) -> u32 {
    let callbacks = match resolve_android_crypto_callbacks(runtime_id) {
        Ok(callbacks) => callbacks,
        Err(status) => return status,
    };

    if !has_hardware_secret_key_lane(&callbacks, key_algorithm) {
        return unsupported_lane_status();
    }

    probe_hardware_support(runtime_id, store_kind, callbacks)
}

/// Probe one Android host lane for certificate write support.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_crypto_supports_certificate_write(
    runtime_id: u64,
    store_kind: u32,
) -> u32 {
    let callbacks = match resolve_android_crypto_callbacks(runtime_id) {
        Ok(callbacks) => callbacks,
        Err(status) => return status,
    };

    if !supports_certificate_write(&callbacks, store_kind) {
        return unsupported_lane_status();
    }

    HOST_STATUS_OK
}

/// Generate one Android host hardware-backed key pair.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_crypto_generate_hardware_key_pair(
    runtime_id: u64,
    store_kind: u32,
    key_algorithm: u32,
    named_curve: u32,
    modulus_bits: u32,
    public_exponent: u32,
    key_label: NativeStringRef,
) -> u32 {
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.generate_hardware_key_pair,
        |callback| unsafe {
            callback(
                runtime_id,
                store_kind,
                key_algorithm,
                named_curve,
                modulus_bits,
                public_exponent,
                key_label,
            )
        },
    )
}

/// Generate one Android host hardware-backed secret key.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_crypto_generate_hardware_secret_key(
    runtime_id: u64,
    store_kind: u32,
    key_algorithm: u32,
    digest_algorithm: u32,
    key_size_bits: u32,
    key_usage_mask: u32,
    key_label: NativeStringRef,
) -> u32 {
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.generate_hardware_secret_key,
        |callback| unsafe {
            callback(
                runtime_id,
                store_kind,
                key_algorithm,
                digest_algorithm,
                key_size_bits,
                key_usage_mask,
                key_label,
            )
        },
    )
}

/// Export one Android host hardware-backed public key.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_crypto_export_hardware_public_key(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.export_hardware_public_key,
        |callback| unsafe {
            callback(runtime_id, key_algorithm, key_label, output, output_written)
        },
    )
}

/// Sign one payload with one Android host hardware-backed key.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_crypto_sign_hardware_key(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    signature_algorithm: u32,
    digest_algorithm: u32,
    salt_length_bytes: u32,
    payload: NativeSlice<u8>,
    output_signature: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.sign_hardware_key,
        |callback| unsafe {
            callback(
                runtime_id,
                key_algorithm,
                key_label,
                signature_algorithm,
                digest_algorithm,
                salt_length_bytes,
                payload,
                output_signature,
                output_written,
            )
        },
    )
}

/// Decrypt one payload with one Android host hardware-backed key.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_crypto_decrypt_hardware_key(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    encryption_algorithm: u32,
    digest_algorithm: u32,
    label: NativeSlice<u8>,
    payload: NativeSlice<u8>,
    output_plaintext: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.decrypt_hardware_key,
        |callback| unsafe {
            callback(
                runtime_id,
                key_algorithm,
                key_label,
                encryption_algorithm,
                digest_algorithm,
                label,
                payload,
                output_plaintext,
                output_written,
            )
        },
    )
}

/// Encrypt one payload with one Android host hardware-backed secret key.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_crypto_encrypt_hardware_secret_key(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    cipher_algorithm: u32,
    nonce: NativeSlice<u8>,
    additional_data: NativeSlice<u8>,
    tag_length_bytes: u32,
    payload: NativeSlice<u8>,
    output_ciphertext: NativeSlice<u8>,
    output_tag: NativeSlice<u8>,
    output_ciphertext_written: *mut u32,
    output_tag_written: *mut u32,
) -> u32 {
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.encrypt_hardware_secret_key,
        |callback| unsafe {
            callback(
                runtime_id,
                key_algorithm,
                key_label,
                cipher_algorithm,
                nonce,
                additional_data,
                tag_length_bytes,
                payload,
                output_ciphertext,
                output_tag,
                output_ciphertext_written,
                output_tag_written,
            )
        },
    )
}

/// Decrypt one payload with one Android host hardware-backed secret key.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_crypto_decrypt_hardware_secret_key(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    cipher_algorithm: u32,
    nonce: NativeSlice<u8>,
    additional_data: NativeSlice<u8>,
    tag: NativeSlice<u8>,
    payload: NativeSlice<u8>,
    output_plaintext: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.decrypt_hardware_secret_key,
        |callback| unsafe {
            callback(
                runtime_id,
                key_algorithm,
                key_label,
                cipher_algorithm,
                nonce,
                additional_data,
                tag,
                payload,
                output_plaintext,
                output_written,
            )
        },
    )
}

/// Compute one MAC with one Android host hardware-backed secret key.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_crypto_compute_hardware_mac(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    mac_algorithm: u32,
    digest_algorithm: u32,
    tag_length_bytes: u32,
    payload: NativeSlice<u8>,
    output_tag: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.compute_hardware_mac,
        |callback| unsafe {
            callback(
                runtime_id,
                key_algorithm,
                key_label,
                mac_algorithm,
                digest_algorithm,
                tag_length_bytes,
                payload,
                output_tag,
                output_written,
            )
        },
    )
}

/// Derive one shared secret with one Android host hardware-backed key.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_crypto_derive_hardware_shared_secret(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    named_curve: u32,
    peer_public_spki: NativeSlice<u8>,
    output_shared_secret: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.derive_hardware_shared_secret,
        |callback| unsafe {
            callback(
                runtime_id,
                key_algorithm,
                key_label,
                named_curve,
                peer_public_spki,
                output_shared_secret,
                output_written,
            )
        },
    )
}

/// Delete one Android host hardware-backed key.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_crypto_delete_hardware_key(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
) -> u32 {
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.delete_hardware_key,
        |callback| unsafe { callback(runtime_id, key_algorithm, key_label) },
    )
}

/// Import one certificate into one Android host lane.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_crypto_import_certificate(
    runtime_id: u64,
    store_kind: u32,
    certificate_der: NativeSlice<u8>,
) -> u32 {
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.import_certificate,
        |callback| unsafe { callback(runtime_id, store_kind, certificate_der) },
    )
}

/// Delete one certificate from one Android host lane.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_crypto_delete_certificate(
    runtime_id: u64,
    store_kind: u32,
    certificate_der: NativeSlice<u8>,
) -> u32 {
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.delete_certificate,
        |callback| unsafe { callback(runtime_id, store_kind, certificate_der) },
    )
}
