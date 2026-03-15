use crate::host::android::abi::{
    HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
};
use crate::host::android::bridge::crypto::tests::core::{
    complete_support_callbacks, mac_callbacks, probe_callbacks, secret_decryption_callbacks,
    secret_encryption_callbacks, secret_generation_callbacks, support_only_callbacks,
};
use crate::host::android::bridge::crypto::{
    destack_host_android_crypto_compute_hardware_mac,
    destack_host_android_crypto_decrypt_hardware_key,
    destack_host_android_crypto_decrypt_hardware_secret_key,
    destack_host_android_crypto_delete_hardware_key,
    destack_host_android_crypto_derive_hardware_shared_secret,
    destack_host_android_crypto_encrypt_hardware_secret_key,
    destack_host_android_crypto_export_hardware_public_key,
    destack_host_android_crypto_generate_hardware_key_pair,
    destack_host_android_crypto_generate_hardware_secret_key,
    destack_host_android_crypto_sign_hardware_key,
    destack_host_android_crypto_supports_hardware_key,
    destack_host_android_crypto_supports_hardware_key_pair,
    destack_host_android_crypto_supports_hardware_secret_key,
};
use crate::host::android::tests::{
    callback_test_lock, register_android_bindings_crypto, register_android_runtime,
};
use crate::runtime::{NativeSlice, NativeStringRef};

/// Route hardware probe and export calls through the registered callback table.
#[test]
fn test_register_bindings_routes_probe_and_public_export_calls() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();
    let register_status = register_android_bindings_crypto(runtime_id, probe_callbacks());
    assert_eq!(register_status, HOST_STATUS_OK);

    let support_status =
        unsafe { destack_host_android_crypto_supports_hardware_key(runtime_id, 2) };
    assert_eq!(support_status, HOST_STATUS_OK);

    let mut required = 0u32;
    let first_status = unsafe {
        destack_host_android_crypto_export_hardware_public_key(
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

    let mut output = vec![0u8; required as usize];
    let mut written = output.len() as u32;
    let second_status = unsafe {
        destack_host_android_crypto_export_hardware_public_key(
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

/// Report not-supported for hardware lanes that have no registered callback.
#[test]
fn test_register_bindings_missing_advanced_callbacks_report_not_supported() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();
    let register_status = register_android_bindings_crypto(runtime_id, support_only_callbacks());
    assert_eq!(register_status, HOST_STATUS_OK);

    let not_supported_key_pair = unsafe {
        destack_host_android_crypto_generate_hardware_key_pair(
            runtime_id,
            2,
            1,
            23,
            0,
            0,
            NativeStringRef::from("key"),
        )
    };
    assert_eq!(not_supported_key_pair, HOST_STATUS_NOT_SUPPORTED);

    let not_supported_sign = unsafe {
        destack_host_android_crypto_sign_hardware_key(
            runtime_id,
            1,
            NativeStringRef::from("key"),
            2,
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
            std::ptr::null_mut(),
        )
    };
    assert_eq!(not_supported_sign, HOST_STATUS_NOT_SUPPORTED);

    let not_supported_decrypt = unsafe {
        destack_host_android_crypto_decrypt_hardware_key(
            runtime_id,
            1,
            NativeStringRef::from("key"),
            1,
            3,
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
            std::ptr::null_mut(),
        )
    };
    assert_eq!(not_supported_decrypt, HOST_STATUS_NOT_SUPPORTED);

    let not_supported_derive = unsafe {
        destack_host_android_crypto_derive_hardware_shared_secret(
            runtime_id,
            1,
            NativeStringRef::from("key"),
            23,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            std::ptr::null_mut(),
        )
    };
    assert_eq!(not_supported_derive, HOST_STATUS_NOT_SUPPORTED);

    let not_supported_delete = unsafe {
        destack_host_android_crypto_delete_hardware_key(runtime_id, 1, NativeStringRef::from("key"))
    };
    assert_eq!(not_supported_delete, HOST_STATUS_NOT_SUPPORTED);
}

/// Report not-supported for missing hardware-secret generation callbacks.
#[test]
fn test_register_bindings_missing_callback_reports_not_supported() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();
    let register_status = register_android_bindings_crypto(runtime_id, support_only_callbacks());
    assert_eq!(register_status, HOST_STATUS_OK);

    let generate_status = unsafe {
        destack_host_android_crypto_generate_hardware_secret_key(
            runtime_id,
            2,
            3,
            3,
            256,
            0x0000_000c,
            NativeStringRef::from("key"),
        )
    };
    assert_eq!(generate_status, HOST_STATUS_NOT_SUPPORTED);
}

/// Route hardware-secret generation through the registered callback table.
#[test]
fn test_register_bindings_routes_generate_secret_key_calls() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();
    let register_status =
        register_android_bindings_crypto(runtime_id, secret_generation_callbacks());
    assert_eq!(register_status, HOST_STATUS_OK);

    let generate_status = unsafe {
        destack_host_android_crypto_generate_hardware_secret_key(
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
}

/// Route hardware-secret encryption through the registered callback table.
#[test]
fn test_register_bindings_routes_encrypt_secret_key_calls() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();
    let register_status =
        register_android_bindings_crypto(runtime_id, secret_encryption_callbacks());
    assert_eq!(register_status, HOST_STATUS_OK);

    let mut ciphertext_required = 0u32;
    let mut tag_required = 0u32;
    let encrypt_first_status = unsafe {
        destack_host_android_crypto_encrypt_hardware_secret_key(
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

    let mut ciphertext = vec![0u8; ciphertext_required as usize];
    let mut tag = vec![0u8; tag_required as usize];
    let mut ciphertext_written = ciphertext.len() as u32;
    let mut tag_written = tag.len() as u32;
    let encrypt_second_status = unsafe {
        destack_host_android_crypto_encrypt_hardware_secret_key(
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
}

/// Route hardware-secret decryption through the registered callback table.
#[test]
fn test_register_bindings_routes_decrypt_secret_key_calls() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();
    let register_status =
        register_android_bindings_crypto(runtime_id, secret_decryption_callbacks());
    assert_eq!(register_status, HOST_STATUS_OK);

    let mut plaintext_required = 0u32;
    let decrypt_first_status = unsafe {
        destack_host_android_crypto_decrypt_hardware_secret_key(
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

    let mut plaintext = vec![0u8; plaintext_required as usize];
    let mut plaintext_written = plaintext.len() as u32;
    let decrypt_second_status = unsafe {
        destack_host_android_crypto_decrypt_hardware_secret_key(
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
}

/// Route hardware MAC computation through the registered callback table.
#[test]
fn test_register_bindings_routes_compute_hardware_mac_calls() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();
    let register_status = register_android_bindings_crypto(runtime_id, mac_callbacks());
    assert_eq!(register_status, HOST_STATUS_OK);

    let mut mac_required = 0u32;
    let mac_first_status = unsafe {
        destack_host_android_crypto_compute_hardware_mac(
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

    let mut mac_output = vec![0u8; mac_required as usize];
    let mut mac_written = mac_output.len() as u32;
    let mac_second_status = unsafe {
        destack_host_android_crypto_compute_hardware_mac(
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

/// Require complete callback coverage for support probes.
#[test]
fn test_support_probes_require_complete_callback_sets() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();
    let register_status =
        register_android_bindings_crypto(runtime_id, complete_support_callbacks());
    assert_eq!(register_status, HOST_STATUS_OK);

    let rsa_pair_status =
        unsafe { destack_host_android_crypto_supports_hardware_key_pair(runtime_id, 2, 1) };
    assert_eq!(rsa_pair_status, HOST_STATUS_OK);

    let ec_pair_status =
        unsafe { destack_host_android_crypto_supports_hardware_key_pair(runtime_id, 2, 2) };
    assert_eq!(ec_pair_status, HOST_STATUS_NOT_SUPPORTED);

    let aes_secret_status =
        unsafe { destack_host_android_crypto_supports_hardware_secret_key(runtime_id, 2, 3) };
    assert_eq!(aes_secret_status, HOST_STATUS_OK);

    let hmac_secret_status =
        unsafe { destack_host_android_crypto_supports_hardware_secret_key(runtime_id, 2, 4) };
    assert_eq!(hmac_secret_status, HOST_STATUS_NOT_SUPPORTED);
}
