use std::os::raw::c_void;
use std::ptr;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform;
use crate::platform::os::credentials::core::{
    CredentialWriteOptionsOwned, already_exists, interrupted, invalid_data, permission_denied,
    would_block,
};
use crate::platform::os::{
    CredentialAccessibility, CredentialAuthenticationPolicy, CredentialAuthenticationRequirement,
};
use core_foundation_sys::base::{CFRelease, CFTypeRef, OSStatus, kCFAllocatorDefault};
use core_foundation_sys::data::CFDataCreate;
use core_foundation_sys::dictionary::{
    CFDictionaryCreate, kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks,
};
use core_foundation_sys::string::{CFStringCreateWithBytes, kCFStringEncodingUTF8};
use security_framework_sys::access_control::{
    SecAccessControlCreateWithFlags, kSecAccessControlBiometryCurrentSet,
    kSecAccessControlDevicePasscode, kSecAccessControlUserPresence,
    kSecAttrAccessibleAfterFirstUnlock, kSecAttrAccessibleWhenUnlocked,
};
use security_framework_sys::base::{
    errSecAuthFailed, errSecBadReq, errSecDuplicateItem, errSecIO, errSecInternalComponent,
    errSecItemNotFound, errSecParam, errSecUnimplemented,
};

/// Security status code for user-cancelled operation.
const ERR_SEC_USER_CANCELED: OSStatus = -128;
/// Security status code for missing host entitlement.
const ERR_SEC_MISSING_ENTITLEMENT: OSStatus = -34018;
/// Security status code for unavailable keychain service.
const ERR_SEC_NOT_AVAILABLE: OSStatus = -25291;
/// Security status code for blocked user interaction.
const ERR_SEC_INTERACTION_NOT_ALLOWED: OSStatus = -25308;

/// Create one optional keychain access-group object.
pub(crate) fn create_optional_access_group(
    access_group: Option<&str>,
    operation: &'static str,
) -> RuntimeResult<Option<OwnedCfReference>> {
    // propagate missing access-group fields
    let Some(access_group) = access_group else {
        return Ok(None);
    };

    // create one cfstring access-group value
    let access_group = create_cf_string(access_group, operation)?;
    Ok(Some(access_group))
}

/// Create one optional keychain access-control object.
pub(crate) fn create_optional_access_control(
    options: &CredentialWriteOptionsOwned,
    operation: &'static str,
) -> RuntimeResult<Option<OwnedCfReference>> {
    // no policy flags and default accessibility means no access-control object
    if options.authentication == CredentialAuthenticationPolicy::None
        && options.accessibility == CredentialAccessibility::HostDefault
    {
        return Ok(None);
    }

    // map authentication policy to one sec access-control flag value
    let access_control_flags = match options.authentication {
        CredentialAuthenticationPolicy::None => 0,
        CredentialAuthenticationPolicy::UserPresence => kSecAccessControlUserPresence,
        CredentialAuthenticationPolicy::Biometric => kSecAccessControlBiometryCurrentSet,
        CredentialAuthenticationPolicy::DeviceCredential => kSecAccessControlDevicePasscode,
    };
    let accessible = accessibility_value(options.accessibility);

    // create one keychain access-control object
    let mut access_control_error = ptr::null_mut();
    let access_control = unsafe {
        SecAccessControlCreateWithFlags(
            kCFAllocatorDefault,
            accessible.cast(),
            access_control_flags,
            &mut access_control_error,
        )
    };
    if access_control.is_null() {
        return Err(invalid_data(
            operation,
            "SecAccessControlCreateWithFlags failed to create one access-control object",
        ));
    }

    let access_control = OwnedCfReference::new(
        access_control.cast(),
        operation,
        "null keychain access-control reference",
    )?;
    Ok(Some(access_control))
}

/// Create one access-control object for an explicit authentication prompt lane.
pub(crate) fn create_authentication_access_control(
    requirement: CredentialAuthenticationRequirement,
    operation: &'static str,
) -> RuntimeResult<OwnedCfReference> {
    // map required policy to one sec access-control flag value
    let access_control_flags = match requirement {
        CredentialAuthenticationRequirement::BiometricOrDeviceCredential => {
            kSecAccessControlUserPresence
        }
        CredentialAuthenticationRequirement::Biometric => kSecAccessControlBiometryCurrentSet,
        CredentialAuthenticationRequirement::DeviceCredential => kSecAccessControlDevicePasscode,
    };

    // create one keychain access-control object
    let mut access_control_error = ptr::null_mut();
    let access_control = unsafe {
        SecAccessControlCreateWithFlags(
            kCFAllocatorDefault,
            kSecAttrAccessibleWhenUnlocked as *const c_void,
            access_control_flags,
            &mut access_control_error,
        )
    };
    if access_control.is_null() {
        return Err(invalid_data(
            operation,
            "SecAccessControlCreateWithFlags failed to create one authentication access-control object",
        ));
    }

    OwnedCfReference::new(
        access_control.cast(),
        operation,
        "null keychain authentication access-control reference",
    )
}

/// Map one accessibility enum to one keychain constant value.
pub(crate) fn accessibility_value(accessibility: CredentialAccessibility) -> *const c_void {
    match accessibility {
        CredentialAccessibility::WhenUnlocked => unsafe {
            kSecAttrAccessibleWhenUnlocked as *const c_void
        },
        CredentialAccessibility::AfterFirstUnlock => unsafe {
            kSecAttrAccessibleAfterFirstUnlock as *const c_void
        },
        CredentialAccessibility::HostDefault => unsafe {
            kSecAttrAccessibleWhenUnlocked as *const c_void
        },
    }
}

/// Create one cfstring from utf8 text.
pub(crate) fn create_cf_string(
    value: &str,
    operation: &'static str,
) -> RuntimeResult<OwnedCfReference> {
    // encode one utf8 string object for keychain queries
    let string = unsafe {
        CFStringCreateWithBytes(
            kCFAllocatorDefault,
            value.as_ptr(),
            value.len() as isize,
            kCFStringEncodingUTF8,
            0,
        )
    };
    OwnedCfReference::new(
        string as CFTypeRef,
        operation,
        "failed to create one cfstring for keychain operation",
    )
}

/// Create one cfdata payload from bytes.
pub(crate) fn create_cf_data(
    bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<OwnedCfReference> {
    // encode one data object for keychain payload writes
    let data = unsafe { CFDataCreate(kCFAllocatorDefault, bytes.as_ptr(), bytes.len() as isize) };
    OwnedCfReference::new(
        data as CFTypeRef,
        operation,
        "failed to create one cfdata payload for keychain operation",
    )
}

/// Create one cfdictionary query object.
pub(crate) fn create_dictionary(
    keys: &[*const c_void],
    values: &[*const c_void],
    operation: &'static str,
    error_message: &str,
) -> RuntimeResult<OwnedCfReference> {
    // validate dictionary input lengths before calling cfdictionarycreate
    if keys.len() != values.len() {
        return Err(invalid_data(
            operation,
            "keychain dictionary keys and values length mismatch",
        ));
    }

    // create one cfdictionary object from key and value vectors
    let dictionary = unsafe {
        CFDictionaryCreate(
            kCFAllocatorDefault,
            keys.as_ptr(),
            values.as_ptr(),
            keys.len() as isize,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };
    OwnedCfReference::new(dictionary as CFTypeRef, operation, error_message)
}

/// Map one keychain status code into one runtime error.
pub(crate) fn map_keychain_status(operation: &'static str, status: OSStatus) -> Box<RuntimeError> {
    // map missing keychain records
    if status == errSecItemNotFound {
        return platform::core::io_not_found(operation, "credential record not found");
    }

    // map duplicate records for create-only writes
    if status == errSecDuplicateItem {
        return already_exists(operation, "credential record already exists");
    }

    // map malformed request payloads
    if status == errSecParam {
        return platform::core::invalid_argument(
            "credential",
            format!("keychain rejected credential request with status {status}"),
        );
    }

    // map backend argument and request-shape errors
    if status == errSecBadReq {
        return platform::core::invalid_argument(
            "credential",
            format!("keychain rejected one malformed credential request with status {status}"),
        );
    }

    // map access denied and auth failures
    if status == errSecAuthFailed {
        return permission_denied(
            operation,
            "keychain authentication failed for credential operation",
        );
    }

    // map missing entitlements to permission-denied
    if status == ERR_SEC_MISSING_ENTITLEMENT {
        return permission_denied(
            operation,
            "keychain credential operation is missing required entitlement",
        );
    }

    // map explicit user cancellation to ioInterrupted
    if status == ERR_SEC_USER_CANCELED {
        return interrupted(
            operation,
            "keychain credential operation was canceled by the user",
        );
    }

    // map blocked-interaction and unavailable-service states to ioWouldBlock
    if status == ERR_SEC_INTERACTION_NOT_ALLOWED
        || status == ERR_SEC_NOT_AVAILABLE
        || status == errSecIO
    {
        return would_block(
            operation,
            format!(
                "keychain credential operation is temporarily unavailable with status {status}"
            ),
        );
    }

    // map backend unavailability to notSupported
    if status == errSecUnimplemented {
        return platform::core::not_supported(operation);
    }

    // map internal security-framework component failures into ioInvalidData
    if status == errSecInternalComponent {
        return invalid_data(
            operation,
            "keychain credential operation failed in one internal security component",
        );
    }

    // map unknown status codes to invalid data for clearer caller diagnostics
    invalid_data(
        operation,
        format!("keychain credential operation failed with status {status}"),
    )
}
/// Owned cf reference wrapper that releases in drop.
pub(crate) struct OwnedCfReference {
    /// Raw cf type pointer.
    value: CFTypeRef,
}

impl OwnedCfReference {
    /// Create one owned reference wrapper from one raw pointer.
    pub(crate) fn new(
        value: CFTypeRef,
        operation: &'static str,
        error_message: &str,
    ) -> RuntimeResult<Self> {
        // reject null references from fallible cf allocators
        if value.is_null() {
            return Err(invalid_data(operation, error_message));
        }

        Ok(Self { value })
    }

    /// Return one raw pointer for dictionary composition.
    pub(crate) fn as_ptr(&self) -> *const c_void {
        self.value as *const c_void
    }
}

impl Drop for OwnedCfReference {
    fn drop(&mut self) {
        // release one owned cf object
        unsafe {
            CFRelease(self.value);
        }
    }
}
