use std::os::raw::c_void;
use std::ptr;

use core_foundation_sys::base::{CFRelease, CFTypeRef, kCFAllocatorDefault};
use core_foundation_sys::data::CFDataCreate;
use core_foundation_sys::dictionary::{
    CFDictionaryCreate, kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks,
};
use openssl::x509::X509;
use security_framework_sys::base::{errSecDuplicateItem, errSecItemNotFound, errSecSuccess};
use security_framework_sys::item::{
    kSecClass, kSecClassCertificate, kSecUseAuthenticationUI, kSecUseAuthenticationUISkip,
    kSecValueData,
};
use security_framework_sys::keychain_item::{SecItemAdd, SecItemDelete};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::crypto::CryptoStoreKind;
use crate::platform::crypto::host::unix::core as unix_core;
use crate::runtime::BindingCallContext;

use super::core::{
    configured_system_certificate_directories, configured_system_certificate_files, invalid_data,
    permission_denied,
};

/// Return whether one host store lane supports certificate write operations.
pub(crate) fn host_store_supports_certificate_write(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    let _ = context;

    kind == CryptoStoreKind::User
}

/// Import one certificate into one host store lane.
pub(crate) fn host_store_import_certificate(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    let _ = context;

    // certificate writes are user-lane only
    if kind != CryptoStoreKind::User {
        return Err(core_platform::not_supported(operation));
    }

    // serialize one der payload for keychain insert
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

    // build one keychain insert query
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
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    let _ = context;

    // certificate writes are user-lane only
    if kind != CryptoStoreKind::User {
        return Err(core_platform::not_supported(operation));
    }

    // serialize one der payload for keychain lookup and delete
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

    // build one keychain delete query
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

/// Collect certificates from configured iOS system trust-bundle locations.
pub(super) fn collect_system_certificates(context: &BindingCallContext) -> Vec<X509> {
    let system_certificate_files = configured_system_certificate_files(context);
    let system_certificate_directories = configured_system_certificate_directories(context);

    unix_core::collect_system_certificates(
        &system_certificate_files,
        &system_certificate_directories,
    )
}

/// Return whether one iOS host certificate source path exists.
pub(super) fn has_system_certificate_source(context: &BindingCallContext) -> bool {
    let system_certificate_files = configured_system_certificate_files(context);
    let system_certificate_directories = configured_system_certificate_directories(context);

    unix_core::has_system_certificate_source(
        &system_certificate_files,
        &system_certificate_directories,
    )
}
