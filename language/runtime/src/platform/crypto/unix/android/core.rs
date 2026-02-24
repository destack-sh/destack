use crate::diagnostic::RuntimeError;
use crate::platform::PlatformError;

/// Return one notSupported runtime error.
pub(super) fn not_supported(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(operation)).boxed()
}
