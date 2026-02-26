use super::super::abi::HOST_STATUS_BUFFER_TOO_SMALL;
use super::super::tests::{callback_test_lock, register_android_runtime};
use super::{
    ANDROID_HOST_CRYPTO_CALLBACKS_ABI_VERSION, AndroidHostCryptoCallbacks,
    HOST_STATUS_INVALID_ARGUMENT, HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
    clear_android_host_crypto_callbacks, destack_runtime_host_android_crypto_compute_hardware_mac,
    destack_runtime_host_android_crypto_decrypt_hardware_secret_key,
    destack_runtime_host_android_crypto_delete_certificate,
    destack_runtime_host_android_crypto_encrypt_hardware_secret_key,
    destack_runtime_host_android_crypto_export_hardware_public_key,
    destack_runtime_host_android_crypto_generate_hardware_secret_key,
    destack_runtime_host_android_crypto_import_certificate,
    destack_runtime_host_android_crypto_set_callbacks,
    destack_runtime_host_android_crypto_supports_hardware_key,
};
use crate::platform::{NativeSlice, NativeStringRef};

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

#[test]
fn test_default_callbacks_return_not_supported() {
    let _lock = callback_test_lock().lock().unwrap();
    clear_android_host_crypto_callbacks();

    let support_status = unsafe { destack_runtime_host_android_crypto_supports_hardware_key(1, 2) };
    assert_eq!(support_status, HOST_STATUS_NOT_FOUND);
}

#[test]
fn test_set_callbacks_routes_calls() {
    let _lock = callback_test_lock().lock().unwrap();
    clear_android_host_crypto_callbacks();

    // register one temporary android bridge for runtime-id validation
    let (_bridge, _registration, runtime_id) = register_android_runtime();

    // set one callback table with support and export handlers
    let callbacks = AndroidHostCryptoCallbacks {
        abi_version: ANDROID_HOST_CRYPTO_CALLBACKS_ABI_VERSION,
        supports_hardware_key: Some(test_supports_hardware_key),
        export_hardware_public_key: Some(test_export_hardware_public_key),
        ..AndroidHostCryptoCallbacks::default()
    };
    let set_status = unsafe { destack_runtime_host_android_crypto_set_callbacks(callbacks) };
    assert_eq!(set_status, HOST_STATUS_OK);

    // verify that support probing now routes into the callback table
    let support_status =
        unsafe { destack_runtime_host_android_crypto_supports_hardware_key(runtime_id, 2) };
    assert_eq!(support_status, HOST_STATUS_OK);

    // verify two-pass export flow with one null first pass
    let mut required = 0u32;
    let first_status = unsafe {
        destack_runtime_host_android_crypto_export_hardware_public_key(
            runtime_id,
            2,
            NativeStringRef::from("key"),
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut required as *mut u32,
        )
    };
    assert_eq!(first_status, HOST_STATUS_BUFFER_TOO_SMALL);
    assert_eq!(required, 4);

    // verify second pass writes the exported bytes
    let mut output = vec![0u8; required as usize];
    let mut written = output.len() as u32;
    let second_status = unsafe {
        destack_runtime_host_android_crypto_export_hardware_public_key(
            runtime_id,
            2,
            NativeStringRef::from("key"),
            NativeSlice {
                data: output.as_mut_ptr(),
                len: output.len() as u32,
            },
            &mut written as *mut u32,
        )
    };
    assert_eq!(second_status, HOST_STATUS_OK);
    assert_eq!(written, 4);
    assert_eq!(output, vec![1u8, 2, 3, 4]);
}

#[test]
fn test_set_callbacks_rejects_unknown_abi_version() {
    let _lock = callback_test_lock().lock().unwrap();
    clear_android_host_crypto_callbacks();

    let callbacks = AndroidHostCryptoCallbacks {
        abi_version: ANDROID_HOST_CRYPTO_CALLBACKS_ABI_VERSION + 1,
        ..AndroidHostCryptoCallbacks::default()
    };
    let status = unsafe { destack_runtime_host_android_crypto_set_callbacks(callbacks) };

    assert_eq!(status, HOST_STATUS_INVALID_ARGUMENT);
}

#[test]
fn test_set_callbacks_routes_secret_key_calls() {
    let _lock = callback_test_lock().lock().unwrap();
    clear_android_host_crypto_callbacks();

    // register one temporary android bridge for runtime-id validation
    let (_bridge, _registration, runtime_id) = register_android_runtime();

    // install callbacks for hardware-secret generation and operations
    let callbacks = AndroidHostCryptoCallbacks {
        abi_version: ANDROID_HOST_CRYPTO_CALLBACKS_ABI_VERSION,
        generate_hardware_secret_key: Some(test_generate_hardware_secret_key),
        encrypt_hardware_secret_key: Some(test_encrypt_hardware_secret_key),
        decrypt_hardware_secret_key: Some(test_decrypt_hardware_secret_key),
        compute_hardware_mac: Some(test_compute_hardware_mac),
        ..AndroidHostCryptoCallbacks::default()
    };
    let set_status = unsafe { destack_runtime_host_android_crypto_set_callbacks(callbacks) };
    assert_eq!(set_status, HOST_STATUS_OK);

    // verify hardware-secret generation callback routing
    let generate_status = unsafe {
        destack_runtime_host_android_crypto_generate_hardware_secret_key(
            runtime_id,
            2,
            3,
            3,
            256,
            0x0000_000c,
            NativeStringRef::from("key"),
        )
    };
    assert_eq!(generate_status, HOST_STATUS_OK);

    // verify two-pass secret-key encryption callback routing
    let mut ciphertext_required = 0u32;
    let mut tag_required = 0u32;
    let encrypt_first_status = unsafe {
        destack_runtime_host_android_crypto_encrypt_hardware_secret_key(
            runtime_id,
            3,
            NativeStringRef::from("key"),
            1,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            16,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut ciphertext_required as *mut u32,
            &mut tag_required as *mut u32,
        )
    };
    assert_eq!(encrypt_first_status, HOST_STATUS_BUFFER_TOO_SMALL);
    assert_eq!(ciphertext_required, 3);
    assert_eq!(tag_required, 2);

    // verify second-pass encryption output writes both payload lanes
    let mut ciphertext = vec![0u8; ciphertext_required as usize];
    let mut tag = vec![0u8; tag_required as usize];
    let mut ciphertext_written = ciphertext.len() as u32;
    let mut tag_written = tag.len() as u32;
    let encrypt_second_status = unsafe {
        destack_runtime_host_android_crypto_encrypt_hardware_secret_key(
            runtime_id,
            3,
            NativeStringRef::from("key"),
            1,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            16,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            NativeSlice {
                data: ciphertext.as_mut_ptr(),
                len: ciphertext.len() as u32,
            },
            NativeSlice {
                data: tag.as_mut_ptr(),
                len: tag.len() as u32,
            },
            &mut ciphertext_written as *mut u32,
            &mut tag_written as *mut u32,
        )
    };
    assert_eq!(encrypt_second_status, HOST_STATUS_OK);
    assert_eq!(ciphertext_written, 3);
    assert_eq!(tag_written, 2);
    assert_eq!(ciphertext, vec![7u8, 8, 9]);
    assert_eq!(tag, vec![1u8, 2]);

    // verify two-pass secret-key decrypt callback routing
    let mut plaintext_required = 0u32;
    let decrypt_first_status = unsafe {
        destack_runtime_host_android_crypto_decrypt_hardware_secret_key(
            runtime_id,
            3,
            NativeStringRef::from("key"),
            1,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut plaintext_required as *mut u32,
        )
    };
    assert_eq!(decrypt_first_status, HOST_STATUS_BUFFER_TOO_SMALL);
    assert_eq!(plaintext_required, 4);

    // verify second-pass decrypt output writes plaintext payload
    let mut plaintext = vec![0u8; plaintext_required as usize];
    let mut plaintext_written = plaintext.len() as u32;
    let decrypt_second_status = unsafe {
        destack_runtime_host_android_crypto_decrypt_hardware_secret_key(
            runtime_id,
            3,
            NativeStringRef::from("key"),
            1,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            NativeSlice {
                data: plaintext.as_mut_ptr(),
                len: plaintext.len() as u32,
            },
            &mut plaintext_written as *mut u32,
        )
    };
    assert_eq!(decrypt_second_status, HOST_STATUS_OK);
    assert_eq!(plaintext_written, 4);
    assert_eq!(plaintext, vec![3u8, 4, 5, 6]);

    // verify two-pass hardware-mac callback routing
    let mut mac_required = 0u32;
    let mac_first_status = unsafe {
        destack_runtime_host_android_crypto_compute_hardware_mac(
            runtime_id,
            4,
            NativeStringRef::from("key"),
            1,
            3,
            0,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut mac_required as *mut u32,
        )
    };
    assert_eq!(mac_first_status, HOST_STATUS_BUFFER_TOO_SMALL);
    assert_eq!(mac_required, 3);

    // verify second-pass hardware-mac output writes tag bytes
    let mut mac_output = vec![0u8; mac_required as usize];
    let mut mac_written = mac_output.len() as u32;
    let mac_second_status = unsafe {
        destack_runtime_host_android_crypto_compute_hardware_mac(
            runtime_id,
            4,
            NativeStringRef::from("key"),
            1,
            3,
            0,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            NativeSlice {
                data: mac_output.as_mut_ptr(),
                len: mac_output.len() as u32,
            },
            &mut mac_written as *mut u32,
        )
    };
    assert_eq!(mac_second_status, HOST_STATUS_OK);
    assert_eq!(mac_written, 3);
    assert_eq!(mac_output, vec![9u8, 9, 9]);
}

#[test]
fn test_set_callbacks_routes_certificate_calls() {
    let _lock = callback_test_lock().lock().unwrap();
    clear_android_host_crypto_callbacks();

    // register one temporary android bridge for runtime-id validation
    let (_bridge, _registration, runtime_id) = register_android_runtime();

    // install callbacks for certificate import and delete
    let callbacks = AndroidHostCryptoCallbacks {
        abi_version: ANDROID_HOST_CRYPTO_CALLBACKS_ABI_VERSION,
        import_certificate: Some(test_import_certificate),
        delete_certificate: Some(test_delete_certificate),
        ..AndroidHostCryptoCallbacks::default()
    };
    let set_status = unsafe { destack_runtime_host_android_crypto_set_callbacks(callbacks) };
    assert_eq!(set_status, HOST_STATUS_OK);

    // verify certificate import callback routing
    let import_status = unsafe {
        destack_runtime_host_android_crypto_import_certificate(
            runtime_id,
            2,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
        )
    };
    assert_eq!(import_status, HOST_STATUS_OK);

    // verify certificate delete callback routing
    let delete_status = unsafe {
        destack_runtime_host_android_crypto_delete_certificate(
            runtime_id,
            2,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
        )
    };
    assert_eq!(delete_status, HOST_STATUS_OK);
}
