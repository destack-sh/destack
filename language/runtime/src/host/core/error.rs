use crate::diagnostic::RuntimeError;
use crate::host::Platform;
use crate::host::core::request::HostRequestCompletion;
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;

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
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::IoNotFound),
        format!("missing host queue registration for {platform} runtime {runtime_id}"),
    ))
    .boxed()
}

/// Return one missing runtime-host queue error without a platform tag.
pub(crate) fn missing_host_queue_registration(runtime_id: u64) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::IoNotFound),
        format!("missing host queue registration for runtime {runtime_id}"),
    ))
    .boxed()
}

/// Return one unsupported host completion error for synchronous operation decoding.
pub(crate) fn unsupported_request_completion(
    operation: &'static str,
    completion: HostRequestCompletion,
) -> Box<RuntimeError> {
    let completion = match completion {
        HostRequestCompletion::Immediate => "immediate",
        HostRequestCompletion::Deferred => "deferred",
        HostRequestCompletion::EventCompleting => "event-completing",
    };

    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        format!(
            "{operation} returned one {completion} host completion, but sync host decoding is still in use: move this request to one interactive host transaction path"
        ),
    ))
    .boxed()
}
