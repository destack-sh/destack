use crate::diagnostic::RuntimeError;
use crate::host::Platform;
use crate::platform::PlatformError;

/// Return one runtime-host not-supported error.
pub(crate) fn not_supported(operation: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(operation)).boxed()
}

/// Return one runtime-host invalid-argument error.
pub(crate) fn invalid_argument_value(
    argument: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::invalid_argument_value(argument, message)).boxed()
}

/// Return one missing runtime-host queue error.
pub(crate) fn missing_host_queue(runtime_id: u64, platform: Platform) -> Box<RuntimeError> {
    let platform = platform.canonical_tag();
    not_supported(format!("runtime.host.queue.{platform}.{runtime_id}"))
}
