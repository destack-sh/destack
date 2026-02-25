use std::os::raw::c_void;

use core_foundation_sys::base::{CFRelease, CFTypeRef, kCFAllocatorDefault};
use core_foundation_sys::dictionary::{
    CFDictionaryCreate, kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks,
};
use core_foundation_sys::number::{CFNumberCreate, kCFBooleanTrue, kCFNumberSInt32Type};
use security_framework_sys::base::SecKeyRef;
use security_framework_sys::item::{
    kSecAttrIsPermanent, kSecAttrKeySizeInBits, kSecAttrKeyType, kSecAttrKeyTypeECSECPrimeRandom,
    kSecAttrKeyTypeRSA, kSecAttrLabel, kSecPrivateKeyAttrs, kSecUseAuthenticationUI,
    kSecUseAuthenticationUISkip,
};
use security_framework_sys::key::SecKeyCreateRandomKey;

use crate::diagnostic::RuntimeResult;

use super::core::{create_cf_string, invalid_data, security_operation_error};
pub(super) use crate::platform::crypto::host::unix::apple::{
    copy_key_external_representation, copy_private_key_by_label,
    create_keychain_private_key_from_der, create_secure_enclave_private_key,
    delete_private_key_by_label_if_present, probe_secure_enclave_support,
};

/// Create one keychain-backed RSA private key.
pub(super) fn create_keychain_rsa_private_key(
    key_label: &str,
    modulus_bits: u32,
    operation: &'static str,
) -> RuntimeResult<SecKeyRef> {
    // convert modulus size for cfnumber payload
    let modulus_bits = i32::try_from(modulus_bits).map_err(|_| {
        invalid_data(
            operation,
            "rsa modulus bits do not fit one macOS keychain size field",
        )
    })?;
    let label = create_cf_string(key_label, operation)?;
    let key_size = unsafe {
        CFNumberCreate(
            kCFAllocatorDefault,
            kCFNumberSInt32Type,
            (&modulus_bits as *const i32).cast(),
        )
    };
    if key_size.is_null() {
        unsafe {
            CFRelease(label as CFTypeRef);
        }
        return Err(invalid_data(
            operation,
            "failed to encode one keychain rsa key size",
        ));
    }

    // build private key attrs
    let private_keys = unsafe { [kSecAttrIsPermanent as *const c_void] };
    let private_values = unsafe { [kCFBooleanTrue as *const c_void] };
    let private_attributes = unsafe {
        CFDictionaryCreate(
            kCFAllocatorDefault,
            private_keys.as_ptr(),
            private_values.as_ptr(),
            private_keys.len() as isize,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };
    if private_attributes.is_null() {
        unsafe {
            CFRelease(label as CFTypeRef);
            CFRelease(key_size as CFTypeRef);
        }
        return Err(invalid_data(
            operation,
            "failed to encode one keychain rsa private-key attributes",
        ));
    }

    // build key attrs
    let keys = unsafe {
        [
            kSecAttrKeyType as *const c_void,
            kSecAttrKeySizeInBits as *const c_void,
            kSecAttrLabel as *const c_void,
            kSecPrivateKeyAttrs as *const c_void,
            kSecUseAuthenticationUI as *const c_void,
        ]
    };
    let values = unsafe {
        [
            kSecAttrKeyTypeRSA as *const c_void,
            key_size as *const c_void,
            label as *const c_void,
            private_attributes as *const c_void,
            kSecUseAuthenticationUISkip as *const c_void,
        ]
    };
    let attributes = unsafe {
        CFDictionaryCreate(
            kCFAllocatorDefault,
            keys.as_ptr(),
            values.as_ptr(),
            keys.len() as isize,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };
    unsafe {
        CFRelease(private_attributes as CFTypeRef);
        CFRelease(label as CFTypeRef);
        CFRelease(key_size as CFTypeRef);
    }
    if attributes.is_null() {
        return Err(invalid_data(
            operation,
            "failed to encode one keychain rsa key attributes",
        ));
    }

    // create key and release attribute dictionary
    let mut error = std::ptr::null_mut();
    let key = unsafe { SecKeyCreateRandomKey(attributes, &mut error) };
    unsafe {
        CFRelease(attributes as CFTypeRef);
    }
    if key.is_null() {
        return Err(security_operation_error(
            operation,
            "create one keychain rsa private key",
            error as CFTypeRef,
        ));
    }

    Ok(key)
}

/// Create one keychain-backed EC private key.
pub(super) fn create_keychain_ec_private_key(
    key_label: &str,
    key_size_bits: i32,
    operation: &'static str,
) -> RuntimeResult<SecKeyRef> {
    // encode key label and ec key size
    let label = create_cf_string(key_label, operation)?;
    let key_size = unsafe {
        CFNumberCreate(
            kCFAllocatorDefault,
            kCFNumberSInt32Type,
            (&key_size_bits as *const i32).cast(),
        )
    };
    if key_size.is_null() {
        unsafe {
            CFRelease(label as CFTypeRef);
        }
        return Err(invalid_data(
            operation,
            "failed to encode one keychain ec key size",
        ));
    }

    // build private key attrs
    let private_keys = unsafe { [kSecAttrIsPermanent as *const c_void] };
    let private_values = unsafe { [kCFBooleanTrue as *const c_void] };
    let private_attributes = unsafe {
        CFDictionaryCreate(
            kCFAllocatorDefault,
            private_keys.as_ptr(),
            private_values.as_ptr(),
            private_keys.len() as isize,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };
    if private_attributes.is_null() {
        unsafe {
            CFRelease(label as CFTypeRef);
            CFRelease(key_size as CFTypeRef);
        }
        return Err(invalid_data(
            operation,
            "failed to encode one keychain ec private-key attributes",
        ));
    }

    // build key attrs
    let keys = unsafe {
        [
            kSecAttrKeyType as *const c_void,
            kSecAttrKeySizeInBits as *const c_void,
            kSecAttrLabel as *const c_void,
            kSecPrivateKeyAttrs as *const c_void,
            kSecUseAuthenticationUI as *const c_void,
        ]
    };
    let values = unsafe {
        [
            kSecAttrKeyTypeECSECPrimeRandom as *const c_void,
            key_size as *const c_void,
            label as *const c_void,
            private_attributes as *const c_void,
            kSecUseAuthenticationUISkip as *const c_void,
        ]
    };
    let attributes = unsafe {
        CFDictionaryCreate(
            kCFAllocatorDefault,
            keys.as_ptr(),
            values.as_ptr(),
            keys.len() as isize,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };
    unsafe {
        CFRelease(private_attributes as CFTypeRef);
        CFRelease(label as CFTypeRef);
        CFRelease(key_size as CFTypeRef);
    }
    if attributes.is_null() {
        return Err(invalid_data(
            operation,
            "failed to encode one keychain ec key attributes",
        ));
    }

    // create key and release attribute dictionary
    let mut error = std::ptr::null_mut();
    let key = unsafe { SecKeyCreateRandomKey(attributes, &mut error) };
    unsafe {
        CFRelease(attributes as CFTypeRef);
    }
    if key.is_null() {
        return Err(security_operation_error(
            operation,
            "create one keychain ec private key",
            error as CFTypeRef,
        ));
    }

    Ok(key)
}
