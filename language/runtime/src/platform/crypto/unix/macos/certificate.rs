use std::collections::HashSet;
use std::os::raw::c_void;
use std::ptr;

use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef};
use core_foundation_sys::base::{CFEqual, CFRelease, CFTypeRef, kCFAllocatorDefault};
use core_foundation_sys::data::{CFDataCreate, CFDataGetBytePtr, CFDataGetLength};
use core_foundation_sys::dictionary::{
    CFDictionaryCreate, CFDictionaryGetValue, CFDictionaryRef, kCFTypeDictionaryKeyCallBacks,
    kCFTypeDictionaryValueCallBacks,
};
use core_foundation_sys::number::{
    CFNumberGetValue, CFNumberRef, kCFBooleanTrue, kCFNumberIntType,
};
use openssl::x509::X509;
use security_framework_sys::base::{
    SecCertificateRef, errSecDuplicateItem, errSecItemNotFound, errSecNoTrustSettings,
    errSecSuccess,
};
use security_framework_sys::certificate::SecCertificateCopyData;
use security_framework_sys::item::{
    kSecClass, kSecClassCertificate, kSecMatchLimit, kSecMatchLimitAll, kSecMatchTrustedOnly,
    kSecReturnRef, kSecUseAuthenticationUI, kSecUseAuthenticationUISkip, kSecValueData,
};
use security_framework_sys::keychain_item::{SecItemAdd, SecItemCopyMatching, SecItemDelete};
use security_framework_sys::trust_settings::{
    SecTrustSettingsCopyCertificates, SecTrustSettingsCopyTrustSettings,
    kSecTrustSettingsDomainAdmin, kSecTrustSettingsDomainUser, kSecTrustSettingsResultDeny,
    kSecTrustSettingsResultTrustAsRoot, kSecTrustSettingsResultTrustRoot,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::crypto::CryptoStoreKind;
use crate::platform::crypto::core::{CRYPTO_STORE_OPEN_OPERATION, push_der_certificate_if_unique};
use crate::runtime::BindingCallContext;

use super::core::{create_cf_string, filesystem_mode_enabled, invalid_data, permission_denied};

/// Trust-settings decision for one certificate lane.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TrustSettingsDecision {
    /// Certificate is trusted for TLS server usage.
    Trusted,
    /// Certificate is explicitly denied.
    Denied,
    /// Certificate has no explicit trust decision.
    Unspecified,
}

/// Evaluate one trust-settings array for TLS server trust semantics.
fn evaluate_tls_trust_settings(
    trust_settings: CFArrayRef,
    operation: &'static str,
) -> RuntimeResult<TrustSettingsDecision> {
    let result_key = create_cf_string("kSecTrustSettingsResult", operation)?;
    let policy_name_key = create_cf_string("kSecTrustSettingsPolicyName", operation)?;
    let tls_policy_name = create_cf_string("sslServer", operation)?;

    let decision = (|| {
        // empty trust-settings arrays mean trust-root by Apple contract
        let trust_settings_count = unsafe { CFArrayGetCount(trust_settings) };
        if trust_settings_count == 0 {
            return Ok(TrustSettingsDecision::Trusted);
        }

        // walk trust dictionaries and resolve first explicit trusted or denied outcome
        for index in 0..trust_settings_count {
            let trust_dictionary =
                unsafe { CFArrayGetValueAtIndex(trust_settings, index) as CFDictionaryRef };
            if trust_dictionary.is_null() {
                continue;
            }

            // skip non-ssl policy scoped trust entries
            let policy_name =
                unsafe { CFDictionaryGetValue(trust_dictionary, policy_name_key as *const c_void) };
            if !policy_name.is_null() {
                let is_tls_policy =
                    unsafe { CFEqual(policy_name as CFTypeRef, tls_policy_name as CFTypeRef) != 0 };
                if !is_tls_policy {
                    continue;
                }
            }

            // default missing trust result to trust-root as per trust-settings contract
            let result_value =
                unsafe { CFDictionaryGetValue(trust_dictionary, result_key as *const c_void) };
            if result_value.is_null() {
                return Ok(TrustSettingsDecision::Trusted);
            }

            let mut trust_result: i32 = 0;
            let is_number = unsafe {
                CFNumberGetValue(
                    result_value as CFNumberRef,
                    kCFNumberIntType,
                    (&mut trust_result as *mut i32).cast(),
                )
            };
            if !is_number {
                continue;
            }

            if trust_result == kSecTrustSettingsResultDeny as i32 {
                return Ok(TrustSettingsDecision::Denied);
            }
            if trust_result == kSecTrustSettingsResultTrustRoot as i32
                || trust_result == kSecTrustSettingsResultTrustAsRoot as i32
            {
                return Ok(TrustSettingsDecision::Trusted);
            }
        }

        Ok(TrustSettingsDecision::Unspecified)
    })();

    unsafe {
        CFRelease(result_key as CFTypeRef);
        CFRelease(policy_name_key as CFTypeRef);
        CFRelease(tls_policy_name as CFTypeRef);
    }

    decision
}

/// Return whether one certificate is trusted for TLS in one trust-settings domain.
fn certificate_is_trusted_for_tls_in_domain(
    certificate: SecCertificateRef,
    domain: u32,
    operation: &'static str,
) -> RuntimeResult<bool> {
    let mut trust_settings: CFArrayRef = ptr::null();
    let status =
        unsafe { SecTrustSettingsCopyTrustSettings(certificate, domain, &mut trust_settings) };
    if status == errSecItemNotFound || status == errSecNoTrustSettings {
        return Ok(true);
    }
    if status != errSecSuccess {
        return Err(permission_denied(
            operation,
            format!("SecTrustSettingsCopyTrustSettings failed with status code {status}"),
        ));
    }
    if trust_settings.is_null() {
        return Ok(true);
    }

    let decision = evaluate_tls_trust_settings(trust_settings, operation);
    unsafe {
        CFRelease(trust_settings as CFTypeRef);
    }
    let decision = decision?;

    Ok(decision == TrustSettingsDecision::Trusted)
}

/// Return whether one host certificate lane can be queried.
pub(crate) fn host_store_certificate_lane_is_available(kind: CryptoStoreKind) -> bool {
    // system lane availability is based on keychain certificate query capability
    if kind == CryptoStoreKind::System {
        let query_keys = unsafe {
            [
                kSecClass as *const c_void,
                kSecMatchLimit as *const c_void,
                kSecReturnRef as *const c_void,
                kSecUseAuthenticationUI as *const c_void,
            ]
        };
        let query_values = unsafe {
            [
                kSecClassCertificate as *const c_void,
                kSecMatchLimitAll as *const c_void,
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
        if query.is_null() {
            return false;
        }

        let mut result: CFTypeRef = ptr::null();
        let status = unsafe { SecItemCopyMatching(query, &mut result) };
        unsafe {
            CFRelease(query as CFTypeRef);
            if !result.is_null() {
                CFRelease(result);
            }
        }

        return status == errSecSuccess || status == errSecItemNotFound;
    }

    // user and machine lanes are available when trust-settings queries succeed
    let domain = match kind {
        CryptoStoreKind::User => kSecTrustSettingsDomainUser,
        CryptoStoreKind::Machine => kSecTrustSettingsDomainAdmin,
        _ => return false,
    };
    let mut certificate_array: CFArrayRef = std::ptr::null();
    let status = unsafe { SecTrustSettingsCopyCertificates(domain, &mut certificate_array) };
    if !certificate_array.is_null() {
        unsafe {
            CFRelease(certificate_array as CFTypeRef);
        }
    }

    status == errSecSuccess || status == errSecNoTrustSettings || status == errSecItemNotFound
}

/// Return whether one host store lane supports certificate write operations.
pub(crate) fn host_store_supports_certificate_write(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    if filesystem_mode_enabled(binding) {
        return false;
    }

    kind == CryptoStoreKind::User
}

/// Import one certificate into one host store lane.
pub(crate) fn host_store_import_certificate(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    // filesystem mode does not currently expose certificate persistence
    if filesystem_mode_enabled(binding) {
        return Err(core_platform::not_supported(operation));
    }

    // writable keychain certificate lane is currently user-only
    if kind != CryptoStoreKind::User {
        return Err(core_platform::not_supported(operation));
    }

    // serialize certificate as der payload for keychain import
    let certificate_bytes = certificate
        .to_der()
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let certificate_data = unsafe {
        CFDataCreate(
            kCFAllocatorDefault,
            certificate_bytes.as_ptr(),
            certificate_bytes.len() as isize,
        )
    };
    if certificate_data.is_null() {
        return Err(invalid_data(
            operation,
            "failed to create one certificate data payload",
        ));
    }

    // build keychain import query
    let query_keys = unsafe {
        [
            kSecClass as *const c_void,
            kSecValueData as *const c_void,
            kSecUseAuthenticationUI as *const c_void,
        ]
    };
    let query_values = unsafe {
        [
            kSecClassCertificate as *const c_void,
            certificate_data as *const c_void,
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
    if query.is_null() {
        unsafe {
            CFRelease(certificate_data as CFTypeRef);
        }
        return Err(invalid_data(
            operation,
            "failed to create one certificate import query",
        ));
    }

    // import one keychain certificate item
    let status = unsafe { SecItemAdd(query, ptr::null_mut()) };
    unsafe {
        CFRelease(query as CFTypeRef);
        CFRelease(certificate_data as CFTypeRef);
    }
    if status == errSecSuccess || status == errSecDuplicateItem {
        return Ok(());
    }

    Err(permission_denied(
        operation,
        format!("SecItemAdd failed with status code {status}"),
    ))
}

/// Delete one certificate from one host store lane.
pub(crate) fn host_store_delete_certificate(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    // filesystem mode does not currently expose certificate persistence
    if filesystem_mode_enabled(binding) {
        return Err(core_platform::not_supported(operation));
    }

    // writable keychain certificate lane is currently user-only
    if kind != CryptoStoreKind::User {
        return Err(core_platform::not_supported(operation));
    }

    // serialize certificate as der payload for keychain deletion
    let certificate_bytes = certificate
        .to_der()
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let certificate_data = unsafe {
        CFDataCreate(
            kCFAllocatorDefault,
            certificate_bytes.as_ptr(),
            certificate_bytes.len() as isize,
        )
    };
    if certificate_data.is_null() {
        return Err(invalid_data(
            operation,
            "failed to create one certificate data payload",
        ));
    }

    // build keychain delete query
    let query_keys = unsafe {
        [
            kSecClass as *const c_void,
            kSecValueData as *const c_void,
            kSecUseAuthenticationUI as *const c_void,
        ]
    };
    let query_values = unsafe {
        [
            kSecClassCertificate as *const c_void,
            certificate_data as *const c_void,
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
    if query.is_null() {
        unsafe {
            CFRelease(certificate_data as CFTypeRef);
        }
        return Err(invalid_data(
            operation,
            "failed to create one certificate delete query",
        ));
    }

    // delete one keychain certificate item
    let status = unsafe { SecItemDelete(query) };
    unsafe {
        CFRelease(query as CFTypeRef);
        CFRelease(certificate_data as CFTypeRef);
    }
    if status == errSecSuccess || status == errSecItemNotFound {
        return Ok(());
    }

    Err(permission_denied(
        operation,
        format!("SecItemDelete failed with status code {status}"),
    ))
}

/// Collect trusted certificates from default keychain lanes.
pub(super) fn collect_trusted_certificates() -> RuntimeResult<Vec<X509>> {
    let query_keys = unsafe {
        [
            kSecClass as *const c_void,
            kSecMatchLimit as *const c_void,
            kSecReturnRef as *const c_void,
            kSecMatchTrustedOnly as *const c_void,
            kSecUseAuthenticationUI as *const c_void,
        ]
    };
    let query_values = unsafe {
        [
            kSecClassCertificate as *const c_void,
            kSecMatchLimitAll as *const c_void,
            kCFBooleanTrue as *const c_void,
            kCFBooleanTrue as *const c_void,
            kSecUseAuthenticationUISkip as *const c_void,
        ]
    };
    let query_dictionary = unsafe {
        CFDictionaryCreate(
            kCFAllocatorDefault,
            query_keys.as_ptr(),
            query_values.as_ptr(),
            query_keys.len() as isize,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };
    if query_dictionary.is_null() {
        return Err(invalid_data(
            CRYPTO_STORE_OPEN_OPERATION,
            "failed to create macOS keychain query dictionary",
        ));
    }

    let mut query_result: CFTypeRef = ptr::null();
    let status = unsafe { SecItemCopyMatching(query_dictionary, &mut query_result) };
    unsafe {
        CFRelease(query_dictionary as CFTypeRef);
    }

    if status == errSecItemNotFound {
        return Ok(Vec::new());
    }
    if status != errSecSuccess {
        return Err(permission_denied(
            CRYPTO_STORE_OPEN_OPERATION,
            format!("SecItemCopyMatching failed with status code {status}"),
        ));
    }
    if query_result.is_null() {
        return Ok(Vec::new());
    }

    let certificates = unsafe { collect_certificates_from_cfarray(query_result as CFArrayRef) };
    unsafe {
        CFRelease(query_result);
    }

    Ok(certificates)
}

/// Collect certificates from one trust-settings domain.
pub(super) fn collect_trust_settings_certificates(domain: u32) -> RuntimeResult<Vec<X509>> {
    let mut certificate_array: CFArrayRef = ptr::null();
    let status = unsafe { SecTrustSettingsCopyCertificates(domain, &mut certificate_array) };

    if status == errSecNoTrustSettings || status == errSecItemNotFound {
        return Ok(Vec::new());
    }
    if status != errSecSuccess {
        return Err(permission_denied(
            CRYPTO_STORE_OPEN_OPERATION,
            format!("SecTrustSettingsCopyCertificates failed with status code {status}"),
        ));
    }
    if certificate_array.is_null() {
        return Ok(Vec::new());
    }

    // keep only certificates effectively trusted for TLS and deduplicate by DER payload
    let mut certificates = Vec::new();
    let mut seen_der_certificates = HashSet::new();
    let certificate_count = unsafe { CFArrayGetCount(certificate_array) };
    for index in 0..certificate_count {
        let certificate_ref =
            unsafe { CFArrayGetValueAtIndex(certificate_array, index) as SecCertificateRef };
        if certificate_ref.is_null() {
            continue;
        }

        let is_trusted = certificate_is_trusted_for_tls_in_domain(
            certificate_ref,
            domain,
            CRYPTO_STORE_OPEN_OPERATION,
        )?;
        if !is_trusted {
            continue;
        }

        let certificate_data = unsafe { SecCertificateCopyData(certificate_ref) };
        if certificate_data.is_null() {
            continue;
        }

        let byte_pointer = unsafe { CFDataGetBytePtr(certificate_data) };
        let byte_length = unsafe { CFDataGetLength(certificate_data) };
        if !byte_pointer.is_null() && byte_length > 0 {
            let der_bytes =
                unsafe { std::slice::from_raw_parts(byte_pointer, byte_length as usize) };
            push_der_certificate_if_unique(
                der_bytes,
                &mut certificates,
                &mut seen_der_certificates,
            );
        }

        unsafe {
            CFRelease(certificate_data as CFTypeRef);
        }
    }
    unsafe {
        CFRelease(certificate_array as CFTypeRef);
    }

    Ok(certificates)
}

/// Collect parsed certificates from one CFArray.
unsafe fn collect_certificates_from_cfarray(certificate_array: CFArrayRef) -> Vec<X509> {
    let mut certificates = Vec::new();
    let mut seen_der_certificates = HashSet::new();

    let certificate_count = unsafe { CFArrayGetCount(certificate_array) };
    for index in 0..certificate_count {
        let certificate_ref =
            unsafe { CFArrayGetValueAtIndex(certificate_array, index) as SecCertificateRef };
        if certificate_ref.is_null() {
            continue;
        }

        let certificate_data = unsafe { SecCertificateCopyData(certificate_ref) };
        if certificate_data.is_null() {
            continue;
        }

        let byte_pointer = unsafe { CFDataGetBytePtr(certificate_data) };
        let byte_length = unsafe { CFDataGetLength(certificate_data) };
        if !byte_pointer.is_null() && byte_length > 0 {
            let der_bytes =
                unsafe { std::slice::from_raw_parts(byte_pointer, byte_length as usize) };
            push_der_certificate_if_unique(
                der_bytes,
                &mut certificates,
                &mut seen_der_certificates,
            );
        }

        unsafe {
            CFRelease(certificate_data as CFTypeRef);
        }
    }

    certificates
}
