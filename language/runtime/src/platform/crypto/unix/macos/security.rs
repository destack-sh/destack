use std::sync::OnceLock;

use core_foundation_sys::base::{CFRelease, CFTypeRef, kCFAllocatorDefault};
use core_foundation_sys::data::CFDataCreate;
use core_foundation_sys::dictionary::{
    CFDictionaryCreate, kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks,
};
use core_foundation_sys::number::{
    CFNumberCreate, kCFBooleanFalse, kCFBooleanTrue, kCFNumberSInt32Type,
};
use security_framework_sys::base::{SecKeyRef, errSecItemNotFound, errSecSuccess};
use security_framework_sys::item::{
    kSecAttrIsPermanent, kSecAttrKeyClass, kSecAttrKeyClassPrivate, kSecAttrKeySizeInBits,
    kSecAttrKeyType, kSecAttrKeyTypeECSECPrimeRandom, kSecAttrKeyTypeRSA, kSecAttrLabel,
    kSecAttrTokenID, kSecAttrTokenIDSecureEnclave, kSecClass, kSecClassKey, kSecPrivateKeyAttrs,
    kSecReturnRef, kSecUseAuthenticationUI, kSecUseAuthenticationUISkip,
};
use security_framework_sys::key::{
    SecKeyCopyExternalRepresentation, SecKeyCreateRandomKey, SecKeyCreateWithData,
};
use security_framework_sys::keychain_item::{SecItemCopyMatching, SecItemDelete};

use crate::diagnostic::RuntimeResult;

use super::constants::{MACOS_SECURE_ENCLAVE_KEY_SIZE_BITS, MACOS_SECURE_ENCLAVE_PROBE_LABEL};
use super::core::{
    copy_cf_data_bytes, create_cf_string, invalid_data, not_found, permission_denied,
    security_operation_error,
};

/// Cached secure-enclave support probe result.
static SECURE_ENCLAVE_SUPPORT: OnceLock<bool> = OnceLock::new();

/// Probe secure-enclave support.
pub(super) fn probe_secure_enclave_support() -> bool {
    let is_supported = SECURE_ENCLAVE_SUPPORT.get_or_init(|| {
        let key_result = create_secure_enclave_private_key(
            MACOS_SECURE_ENCLAVE_PROBE_LABEL,
            false,
            "destack.crypto.store.probeCapability",
        );
        let Ok(key) = key_result else {
            return false;
        };

        unsafe {
            CFRelease(key as CFTypeRef);
        }

        true
    });

    *is_supported
}

/// Create one secure-enclave private key.
pub(super) fn create_secure_enclave_private_key(
    key_label: &str,
    is_permanent: bool,
    operation: &'static str,
) -> RuntimeResult<SecKeyRef> {
    use std::os::raw::c_void;

    let label = create_cf_string(key_label, operation)?;
    let key_size_bits = MACOS_SECURE_ENCLAVE_KEY_SIZE_BITS;
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
            "failed to encode one secure enclave key size",
        ));
    }

    // build private key attrs
    let private_keys = unsafe { [kSecAttrIsPermanent as *const c_void] };
    let private_values = unsafe {
        [if is_permanent {
            kCFBooleanTrue as *const c_void
        } else {
            kCFBooleanFalse as *const c_void
        }]
    };
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
            "failed to encode one secure enclave private-key attribute dictionary",
        ));
    }

    // build key attrs
    let keys = unsafe {
        [
            kSecAttrKeyType as *const c_void,
            kSecAttrKeySizeInBits as *const c_void,
            kSecAttrLabel as *const c_void,
            kSecAttrTokenID as *const c_void,
            kSecPrivateKeyAttrs as *const c_void,
            kSecUseAuthenticationUI as *const c_void,
        ]
    };
    let values = unsafe {
        [
            kSecAttrKeyTypeECSECPrimeRandom as *const c_void,
            key_size as *const c_void,
            label as *const c_void,
            kSecAttrTokenIDSecureEnclave as *const c_void,
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
            "failed to encode one secure enclave key attribute dictionary",
        ));
    }

    let mut error = std::ptr::null_mut();
    let key = unsafe { SecKeyCreateRandomKey(attributes, &mut error) };
    unsafe {
        CFRelease(attributes as CFTypeRef);
    }
    if key.is_null() {
        return Err(security_operation_error(
            operation,
            "create one secure enclave private key",
            error as CFTypeRef,
        ));
    }

    Ok(key)
}

/// Delete one private key with one keychain label when it exists.
pub(super) fn delete_private_key_by_label_if_present(
    key_label: &str,
    operation: &'static str,
) -> RuntimeResult<()> {
    use std::os::raw::c_void;

    // build one label-scoped private-key deletion query
    let label = create_cf_string(key_label, operation)?;
    let query_keys = unsafe {
        [
            kSecClass as *const c_void,
            kSecAttrLabel as *const c_void,
            kSecAttrKeyClass as *const c_void,
            kSecUseAuthenticationUI as *const c_void,
        ]
    };
    let query_values = unsafe {
        [
            kSecClassKey as *const c_void,
            label as *const c_void,
            kSecAttrKeyClassPrivate as *const c_void,
            kSecUseAuthenticationUISkip as *const c_void,
        ]
    };
    let query = unsafe {
        CFDictionaryCreate(
            kCFAllocatorDefault,
            query_keys.as_ptr(),
            query_values.as_ptr(),
            query_keys.len() as isize,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };
    unsafe {
        CFRelease(label as CFTypeRef);
    }
    if query.is_null() {
        return Err(invalid_data(
            operation,
            "failed to create one private-key delete query",
        ));
    }

    // delete one private-key entry and treat not-found as success
    let status = unsafe { SecItemDelete(query) };
    unsafe {
        CFRelease(query as CFTypeRef);
    }
    if status == errSecSuccess || status == errSecItemNotFound {
        return Ok(());
    }

    Err(permission_denied(
        operation,
        format!("SecItemDelete failed with status code {status}"),
    ))
}

/// Import one keychain private key from DER bytes.
pub(super) fn create_keychain_private_key_from_der(
    private_key_der_bytes: &[u8],
    key_type: *const std::os::raw::c_void,
    key_size_bits: i32,
    key_label: &str,
    operation: &'static str,
) -> RuntimeResult<SecKeyRef> {
    use std::os::raw::c_void;

    // encode private-key bytes, key size, and label payloads
    let data = unsafe {
        CFDataCreate(
            kCFAllocatorDefault,
            private_key_der_bytes.as_ptr(),
            private_key_der_bytes.len() as isize,
        )
    };
    if data.is_null() {
        return Err(invalid_data(
            operation,
            "failed to encode one private-key import payload",
        ));
    }
    let key_size = unsafe {
        CFNumberCreate(
            kCFAllocatorDefault,
            kCFNumberSInt32Type,
            (&key_size_bits as *const i32).cast(),
        )
    };
    if key_size.is_null() {
        unsafe {
            CFRelease(data as CFTypeRef);
        }
        return Err(invalid_data(
            operation,
            "failed to encode one private-key size",
        ));
    }
    let label = create_cf_string(key_label, operation)?;

    // build sec-key attributes for private-key import
    let attribute_keys = unsafe {
        [
            kSecAttrKeyType as *const c_void,
            kSecAttrKeyClass as *const c_void,
            kSecAttrKeySizeInBits as *const c_void,
            kSecAttrIsPermanent as *const c_void,
            kSecAttrLabel as *const c_void,
            kSecUseAuthenticationUI as *const c_void,
        ]
    };
    let attribute_values = unsafe {
        [
            key_type,
            kSecAttrKeyClassPrivate as *const c_void,
            key_size as *const c_void,
            kCFBooleanTrue as *const c_void,
            label as *const c_void,
            kSecUseAuthenticationUISkip as *const c_void,
        ]
    };
    let attributes = unsafe {
        CFDictionaryCreate(
            kCFAllocatorDefault,
            attribute_keys.as_ptr(),
            attribute_values.as_ptr(),
            attribute_keys.len() as isize,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };
    unsafe {
        CFRelease(label as CFTypeRef);
        CFRelease(key_size as CFTypeRef);
    }
    if attributes.is_null() {
        unsafe {
            CFRelease(data as CFTypeRef);
        }
        return Err(invalid_data(
            operation,
            "failed to create one private-key import attributes dictionary",
        ));
    }

    // create one permanent keychain private key from encoded bytes
    let mut error = std::ptr::null_mut();
    let key = unsafe { SecKeyCreateWithData(data, attributes, &mut error) };
    unsafe {
        CFRelease(attributes as CFTypeRef);
        CFRelease(data as CFTypeRef);
    }
    if key.is_null() {
        return Err(security_operation_error(
            operation,
            "import one keychain private key",
            error as CFTypeRef,
        ));
    }

    Ok(key)
}

/// Create one keychain-backed RSA private key.
pub(super) fn create_keychain_rsa_private_key(
    key_label: &str,
    modulus_bits: u32,
    operation: &'static str,
) -> RuntimeResult<SecKeyRef> {
    use std::os::raw::c_void;

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
    use std::os::raw::c_void;

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

/// Resolve one private key by label.
pub(super) fn copy_private_key_by_label(
    key_label: &str,
    operation: &'static str,
) -> RuntimeResult<SecKeyRef> {
    use std::os::raw::c_void;
    use std::ptr;

    let label = create_cf_string(key_label, operation)?;
    let query_keys = unsafe {
        [
            kSecClass as *const c_void,
            kSecAttrLabel as *const c_void,
            kSecAttrKeyClass as *const c_void,
            kSecReturnRef as *const c_void,
            kSecUseAuthenticationUI as *const c_void,
        ]
    };
    let query_values = unsafe {
        [
            kSecClassKey as *const c_void,
            label as *const c_void,
            kSecAttrKeyClassPrivate as *const c_void,
            kCFBooleanTrue as *const c_void,
            kSecUseAuthenticationUISkip as *const c_void,
        ]
    };
    let query = unsafe {
        CFDictionaryCreate(
            kCFAllocatorDefault,
            query_keys.as_ptr(),
            query_values.as_ptr(),
            query_keys.len() as isize,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };
    unsafe {
        CFRelease(label as CFTypeRef);
    }
    if query.is_null() {
        return Err(invalid_data(
            operation,
            "failed to create one key lookup query",
        ));
    }

    let mut result: CFTypeRef = ptr::null();
    let status = unsafe { SecItemCopyMatching(query, &mut result) };
    unsafe {
        CFRelease(query as CFTypeRef);
    }
    if status == errSecItemNotFound {
        return Err(not_found(
            operation,
            format!("host key {key_label} was not found"),
        ));
    }
    if status != errSecSuccess || result.is_null() {
        return Err(permission_denied(
            operation,
            format!("SecItemCopyMatching failed with status code {status}"),
        ));
    }

    Ok(result as SecKeyRef)
}

/// Export one key external representation payload.
pub(super) fn copy_key_external_representation(
    key: SecKeyRef,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let mut error = std::ptr::null_mut();
    let key_data = unsafe { SecKeyCopyExternalRepresentation(key, &mut error) };
    if key_data.is_null() {
        return Err(security_operation_error(
            operation,
            "export one key external representation",
            error as CFTypeRef,
        ));
    }

    let bytes = copy_cf_data_bytes(key_data, operation);
    unsafe {
        CFRelease(key_data as CFTypeRef);
    }
    let bytes = bytes?;

    Ok(bytes)
}
