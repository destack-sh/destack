use std::env;
use std::path::PathBuf;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::crypto::CryptoStoreKind;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::runtime::BindingCallContext;

use super::constants::{
    DEFAULT_UNIX_SYSTEM_CERTIFICATE_DIRECTORIES, DEFAULT_UNIX_SYSTEM_CERTIFICATE_FILES,
    UNIX_MACHINE_KEYSTORE_ABSOLUTE_PATH, UNIX_USER_KEYSTORE_RELATIVE_PATH,
};

/// Return one ioPermissionDenied runtime error.
fn permission_denied(operation: &'static str, message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoPermissionDenied),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Return one notSupported runtime error.
pub(super) fn not_supported(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(operation)).boxed()
}
