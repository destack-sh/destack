#[cfg(target_os = "macos")]
use objc2_foundation::{NSData, NSString};

/// Convert one Cocoa string into one owned Rust string.
#[cfg(target_os = "macos")]
pub(crate) fn nsstring_to_string(value: &NSString) -> String {
    value.to_string()
}

/// Convert one Cocoa data payload into owned bytes.
#[cfg(target_os = "macos")]
pub(crate) fn nsdata_to_vec(value: &NSData) -> Vec<u8> {
    value.to_vec()
}
