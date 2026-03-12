use std::ffi::CString;
use std::os::raw::c_void;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

use core_foundation_sys::base::{CFRelease, CFTypeRef, kCFAllocatorDefault};
use core_foundation_sys::data::{CFDataCreate, CFDataGetBytePtr, CFDataGetLength, CFDataRef};
use core_foundation_sys::dictionary::{
    CFDictionaryCreate, kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks,
};
use core_foundation_sys::number::kCFBooleanTrue;
use openssl::x509::X509;
use security_framework_sys::base::{errSecDuplicateItem, errSecItemNotFound, errSecSuccess};
use security_framework_sys::item::{
    kSecAttrAccount, kSecAttrService, kSecClass, kSecClassGenericPassword, kSecReturnData,
    kSecUseAuthenticationUI, kSecUseAuthenticationUISkip, kSecValueData,
};
use security_framework_sys::keychain_item::{SecItemAdd, SecItemCopyMatching, SecItemUpdate};
use security_framework_sys::trust_settings::{
    kSecTrustSettingsDomainAdmin, kSecTrustSettingsDomainUser,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::crypto::CryptoStoreKind;
use crate::platform::crypto::core::CRYPTO_STORE_OPEN_OPERATION;
use crate::platform::crypto::host::unix::core::{self as unix_core, SnapshotConfig};
use crate::runtime::BindingCallContext;

use super::certificate::{
    collect_trust_settings_certificates, collect_trusted_certificates,
    host_store_certificate_lane_is_available,
};
use super::core::{
    configured_keychain_snapshot_account, configured_keychain_snapshot_service,
    configured_store_path, create_cf_string, filesystem_mode_enabled,
    filesystem_store_lane_is_available, invalid_data, permission_denied,
};

/// Snapshot codec configuration for macOS filesystem override lanes.
const MACOS_FILESYSTEM_SNAPSHOT_CONFIG: SnapshotConfig = SnapshotConfig {
    store_label: "macos",
    associated_data: b"destack.crypto.macos.snapshot.v1",
};

/// Return whether one host lane has a writable persistent-key backend.
pub(crate) fn host_store_persistence_backend_is_available(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    // filesystem override enables persistence only when the target path is writable
    if let Some(path) = configured_store_path(binding, kind) {
        if !matches!(kind, CryptoStoreKind::User | CryptoStoreKind::Machine) {
            return false;
        }

        return probe_filesystem_store_writeability(&path);
    }

    // filesystem mode with no lane path disables this lane
    if filesystem_mode_enabled(binding) {
        return false;
    }

    // keychain snapshot persistence is structurally exposed on the user lane
    //
    // the actual write path still fails explicitly when keychain policy denies access
    kind == CryptoStoreKind::User
}

/// Return whether one filesystem key-store path is writable.
fn probe_filesystem_store_writeability(path: &Path) -> bool {
    // walk up to one existing ancestor without mutating the filesystem
    let Some(parent_directory) = path.parent() else {
        return false;
    };
    let mut current = Some(parent_directory);
    while let Some(candidate) = current {
        if candidate.is_dir() {
            let candidate = match CString::new(candidate.as_os_str().as_bytes()) {
                Ok(candidate) => candidate,
                Err(_) => return false,
            };

            return unsafe { libc::access(candidate.as_ptr(), libc::W_OK | libc::X_OK) == 0 };
        }

        current = candidate.parent();
    }

    false
}

/// Return whether one host store lane is currently available.
pub(crate) fn host_store_lane_is_available(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    // filesystem override uses deterministic lane availability
    if filesystem_mode_enabled(binding) {
        return filesystem_store_lane_is_available(binding, kind);
    }

    // ephemeral lane is always available
    if kind == CryptoStoreKind::Ephemeral {
        return true;
    }

    // host-backed lanes are available when the underlying host lane can be queried
    match kind {
        CryptoStoreKind::System | CryptoStoreKind::User | CryptoStoreKind::Machine => {
            host_store_certificate_lane_is_available(kind)
        }
        CryptoStoreKind::Provider => false,
        CryptoStoreKind::Ephemeral => true,
    }
}

/// Open one host store lane and return certificate snapshots.
pub(crate) fn open_host_store_certificates(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> RuntimeResult<Vec<X509>> {
    // filesystem mode only exposes deterministic local lanes
    if filesystem_mode_enabled(binding) {
        return match kind {
            CryptoStoreKind::User | CryptoStoreKind::Machine | CryptoStoreKind::Ephemeral => {
                Ok(Vec::new())
            }
            CryptoStoreKind::System | CryptoStoreKind::Provider => {
                Err(core_platform::not_supported(CRYPTO_STORE_OPEN_OPERATION))
            }
        };
    }

    // keychain-backed lane routing
    match kind {
        CryptoStoreKind::System => collect_trusted_certificates(),
        CryptoStoreKind::User => collect_trust_settings_certificates(kSecTrustSettingsDomainUser),
        CryptoStoreKind::Machine => {
            collect_trust_settings_certificates(kSecTrustSettingsDomainAdmin)
        }
        CryptoStoreKind::Provider => Err(core_platform::not_supported(CRYPTO_STORE_OPEN_OPERATION)),
        CryptoStoreKind::Ephemeral => Ok(Vec::new()),
    }
}

/// Load one backend host-key snapshot payload for one store lane.
pub(crate) fn load_host_key_snapshot_bytes(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    operation: &'static str,
) -> RuntimeResult<Option<Vec<u8>>> {
    // read snapshot bytes from configured filesystem lane when provided
    if let Some(path) = configured_store_path(binding, kind) {
        return unix_core::load_host_key_snapshot_bytes(
            Some(path),
            MACOS_FILESYSTEM_SNAPSHOT_CONFIG,
            operation,
        );
    }

    // keychain snapshot storage currently only supports the user lane
    if kind != CryptoStoreKind::User {
        return Ok(None);
    }
    // build keychain query values
    let service_name = configured_keychain_snapshot_service(binding);
    let account_name = configured_keychain_snapshot_account(binding);
    let service = create_cf_string(&service_name, operation)?;
    let account = create_cf_string(&account_name, operation)?;
    let query_keys = unsafe {
        [
            kSecClass as *const c_void,
            kSecAttrService as *const c_void,
            kSecAttrAccount as *const c_void,
            kSecReturnData as *const c_void,
            kSecUseAuthenticationUI as *const c_void,
        ]
    };
    let query_values = unsafe {
        [
            kSecClassGenericPassword as *const c_void,
            service as *const c_void,
            account as *const c_void,
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
        unsafe {
            CFRelease(service as CFTypeRef);
            CFRelease(account as CFTypeRef);
        }
        return Err(invalid_data(
            operation,
            "failed to create one macOS keychain query dictionary",
        ));
    }

    // query and release temp objects
    let mut result: CFTypeRef = ptr::null();
    let status = unsafe { SecItemCopyMatching(query, &mut result) };
    unsafe {
        CFRelease(query as CFTypeRef);
        CFRelease(service as CFTypeRef);
        CFRelease(account as CFTypeRef);
    }

    if status == errSecItemNotFound {
        return Ok(None);
    }
    if status != errSecSuccess {
        return Err(permission_denied(
            operation,
            format!("SecItemCopyMatching failed with status code {status}"),
        ));
    }
    if result.is_null() {
        return Ok(None);
    }

    // decode data bytes and return them
    let data = result as CFDataRef;
    let pointer = unsafe { CFDataGetBytePtr(data) };
    let length = unsafe { CFDataGetLength(data) };
    let bytes = if pointer.is_null() || length <= 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(pointer, length as usize) }.to_vec()
    };
    unsafe {
        CFRelease(result);
    }

    Ok(Some(bytes))
}

/// Store one backend host-key snapshot payload for one store lane.
pub(crate) fn store_host_key_snapshot_bytes(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    snapshot_bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<()> {
    // write snapshot bytes to configured filesystem lane when provided
    if let Some(path) = configured_store_path(binding, kind) {
        return unix_core::store_host_key_snapshot_bytes(
            Some(path),
            kind,
            snapshot_bytes,
            MACOS_FILESYSTEM_SNAPSHOT_CONFIG,
            operation,
        );
    }

    // keychain snapshot storage currently only supports the user lane
    if kind != CryptoStoreKind::User {
        return Err(core_platform::not_supported(operation));
    }
    // build keychain data payload
    let service_name = configured_keychain_snapshot_service(binding);
    let account_name = configured_keychain_snapshot_account(binding);
    let service = create_cf_string(&service_name, operation)?;
    let account = create_cf_string(&account_name, operation)?;
    let data = unsafe {
        CFDataCreate(
            kCFAllocatorDefault,
            snapshot_bytes.as_ptr(),
            snapshot_bytes.len() as isize,
        )
    };
    if data.is_null() {
        unsafe {
            CFRelease(service as CFTypeRef);
            CFRelease(account as CFTypeRef);
        }
        return Err(invalid_data(
            operation,
            "failed to create one macOS keychain value payload",
        ));
    }

    // try add first
    let add_keys = unsafe {
        [
            kSecClass as *const c_void,
            kSecAttrService as *const c_void,
            kSecAttrAccount as *const c_void,
            kSecValueData as *const c_void,
            kSecUseAuthenticationUI as *const c_void,
        ]
    };
    let add_values = unsafe {
        [
            kSecClassGenericPassword as *const c_void,
            service as *const c_void,
            account as *const c_void,
            data as *const c_void,
            kSecUseAuthenticationUISkip as *const c_void,
        ]
    };
    let add_query = unsafe {
        CFDictionaryCreate(
            kCFAllocatorDefault,
            add_keys.as_ptr(),
            add_values.as_ptr(),
            add_keys.len() as isize,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };
    if add_query.is_null() {
        unsafe {
            CFRelease(service as CFTypeRef);
            CFRelease(account as CFTypeRef);
            CFRelease(data as CFTypeRef);
        }
        return Err(invalid_data(
            operation,
            "failed to create one macOS keychain add dictionary",
        ));
    }

    let add_status = unsafe { SecItemAdd(add_query, ptr::null_mut()) };
    unsafe {
        CFRelease(add_query as CFTypeRef);
    }

    // update on duplicate
    if add_status == errSecDuplicateItem {
        let query_keys = unsafe {
            [
                kSecClass as *const c_void,
                kSecAttrService as *const c_void,
                kSecAttrAccount as *const c_void,
                kSecUseAuthenticationUI as *const c_void,
            ]
        };
        let query_values = unsafe {
            [
                kSecClassGenericPassword as *const c_void,
                service as *const c_void,
                account as *const c_void,
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
                CFRelease(service as CFTypeRef);
                CFRelease(account as CFTypeRef);
                CFRelease(data as CFTypeRef);
            }
            return Err(invalid_data(
                operation,
                "failed to create one macOS keychain update query dictionary",
            ));
        }

        let update_keys = unsafe { [kSecValueData as *const c_void] };
        let update_values = [data as *const c_void];
        let update = unsafe {
            CFDictionaryCreate(
                kCFAllocatorDefault,
                update_keys.as_ptr(),
                update_values.as_ptr(),
                update_keys.len() as isize,
                &kCFTypeDictionaryKeyCallBacks,
                &kCFTypeDictionaryValueCallBacks,
            )
        };
        if update.is_null() {
            unsafe {
                CFRelease(query as CFTypeRef);
                CFRelease(service as CFTypeRef);
                CFRelease(account as CFTypeRef);
                CFRelease(data as CFTypeRef);
            }
            return Err(invalid_data(
                operation,
                "failed to create one macOS keychain update payload dictionary",
            ));
        }

        let update_status = unsafe { SecItemUpdate(query, update) };
        unsafe {
            CFRelease(query as CFTypeRef);
            CFRelease(update as CFTypeRef);
            CFRelease(service as CFTypeRef);
            CFRelease(account as CFTypeRef);
            CFRelease(data as CFTypeRef);
        }
        if update_status != errSecSuccess {
            return Err(permission_denied(
                operation,
                format!("SecItemUpdate failed with status code {update_status}"),
            ));
        }

        return Ok(());
    }

    // close add path
    unsafe {
        CFRelease(service as CFTypeRef);
        CFRelease(account as CFTypeRef);
        CFRelease(data as CFTypeRef);
    }
    if add_status != errSecSuccess {
        return Err(permission_denied(
            operation,
            format!("SecItemAdd failed with status code {add_status}"),
        ));
    }

    Ok(())
}
