#[cfg(target_os = "macos")]
use std::ffi::{CStr, c_void};
#[cfg(target_os = "macos")]
use std::mem::{MaybeUninit, size_of};

#[cfg(target_os = "macos")]
use crate::diagnostic::RuntimeError;
#[cfg(target_os = "macos")]
use crate::platform::PlatformError;
#[cfg(target_os = "macos")]
use crate::platform::diagnostic::PlatformErrorCode;

#[cfg(target_os = "macos")]
use super::super::abi::{
    AudioObjectGetPropertyData, AudioObjectGetPropertyDataSize, AudioObjectHasProperty,
    AudioObjectID, AudioObjectPropertyScope, AudioObjectPropertySelector,
    AudioObjectSetPropertyData, CFRelease, CFStringGetCString, CFStringGetLength,
    CFStringGetMaximumSizeForEncoding, CFStringRef, CFTypeRef, OSStatus,
};
#[cfg(target_os = "macos")]
use super::super::constants::{K_CF_STRING_ENCODING_UTF8, K_NO_ERR};
#[cfg(target_os = "macos")]
use super::super::format::property_address;

/// Build one CoreAudio runtime error from one osstatus failure.
#[cfg(target_os = "macos")]
pub(crate) fn error(
    operation: &'static str,
    status: OSStatus,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("{} (osstatus {status})", message.into()),
    ))
    .boxed()
}

/// Return whether one CoreAudio object exposes one property at one scope.
#[cfg(target_os = "macos")]
pub(crate) fn has_property(
    object_id: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
) -> bool {
    let address = property_address(selector, scope);
    unsafe { AudioObjectHasProperty(object_id, &address) != 0 }
}

/// Read one CoreAudio property payload size.
#[cfg(target_os = "macos")]
pub(crate) fn get_data_size(
    object_id: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
) -> Result<u32, OSStatus> {
    let address = property_address(selector, scope);
    let mut size = 0u32;
    let status = unsafe {
        AudioObjectGetPropertyDataSize(object_id, &address, 0, std::ptr::null(), &mut size)
    };
    if status != K_NO_ERR {
        return Err(status);
    }

    Ok(size)
}

/// Read one CoreAudio property payload into one byte buffer.
#[cfg(target_os = "macos")]
pub(crate) fn get_data(
    object_id: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
    bytes: &mut [u8],
) -> Result<(), OSStatus> {
    let address = property_address(selector, scope);
    let mut size = bytes.len() as u32;
    let status = unsafe {
        AudioObjectGetPropertyData(
            object_id,
            &address,
            0,
            std::ptr::null(),
            &mut size,
            bytes.as_mut_ptr() as *mut c_void,
        )
    };
    if status != K_NO_ERR {
        return Err(status);
    }

    if size != bytes.len() as u32 {
        return Err(-1);
    }

    Ok(())
}

/// Write one CoreAudio property payload from one byte buffer.
#[cfg(target_os = "macos")]
pub(crate) fn set_data(
    object_id: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
    bytes: &[u8],
) -> Result<(), OSStatus> {
    let address = property_address(selector, scope);
    let status = unsafe {
        AudioObjectSetPropertyData(
            object_id,
            &address,
            0,
            std::ptr::null(),
            bytes.len() as u32,
            bytes.as_ptr() as *const c_void,
        )
    };
    if status != K_NO_ERR {
        return Err(status);
    }

    Ok(())
}

/// Read one typed scalar CoreAudio property when available.
#[cfg(target_os = "macos")]
pub(crate) fn get_scalar_optional<T: Copy>(
    object_id: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
) -> Option<T> {
    if !has_property(object_id, selector, scope) {
        return None;
    }

    let address = property_address(selector, scope);
    let mut value = MaybeUninit::<T>::uninit();
    let mut size = size_of::<T>() as u32;
    let status = unsafe {
        AudioObjectGetPropertyData(
            object_id,
            &address,
            0,
            std::ptr::null(),
            &mut size,
            value.as_mut_ptr() as *mut c_void,
        )
    };
    if status != K_NO_ERR || size != size_of::<T>() as u32 {
        return None;
    }

    Some(unsafe { value.assume_init() })
}

/// Read one CoreAudio CFString property into one UTF-8 string.
#[cfg(target_os = "macos")]
pub(crate) fn get_cfstring_optional(
    object_id: AudioObjectID,
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
) -> Option<String> {
    let value = get_scalar_optional::<CFStringRef>(object_id, selector, scope)?;
    if value.is_null() {
        return None;
    }

    let length = unsafe { CFStringGetLength(value) };
    if length <= 0 {
        unsafe {
            CFRelease(value as CFTypeRef);
        }
        return None;
    }

    let max_utf8 = unsafe { CFStringGetMaximumSizeForEncoding(length, K_CF_STRING_ENCODING_UTF8) };
    if max_utf8 <= 0 {
        unsafe {
            CFRelease(value as CFTypeRef);
        }
        return None;
    }

    let mut buffer = vec![0i8; max_utf8 as usize + 1];
    let converted = unsafe {
        CFStringGetCString(
            value,
            buffer.as_mut_ptr(),
            buffer.len() as isize,
            K_CF_STRING_ENCODING_UTF8,
        ) != 0
    };

    unsafe {
        CFRelease(value as CFTypeRef);
    }

    if !converted {
        return None;
    }

    let text = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    Some(text.to_string_lossy().into_owned())
}
