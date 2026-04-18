use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::HostStatus;
use crate::platform::PlatformError;
use crate::runtime::BindingCallContext;

use super::{io_not_found, io_operation_error, io_would_block, not_supported};

/// Return one runtime id for Android host callback routing.
pub(crate) fn host_session_id(
    binding: &BindingCallContext,
    _operation: &'static str,
) -> RuntimeResult<u64> {
    Ok(binding.worker().runtime_id.0)
}

/// Convert one Android host buffer length into one checked `u32`.
pub(crate) fn checked_u32_length(
    len: usize,
    operation: &'static str,
    label: &'static str,
) -> RuntimeResult<u32> {
    u32::try_from(len).map_err(|_| {
        invalid_data(
            operation,
            format!("android host {label} length exceeded u32 range"),
        )
    })
}

/// Build one Android-host invalid-data runtime error.
pub(crate) fn invalid_data(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::invalid_data(format!(
        "{operation}: {}",
        message.into()
    )))
    .boxed()
}

/// Map one Android host callback status into one runtime result.
pub(crate) fn host_status_result(
    status: u32,
    operation: &'static str,
    action: &'static str,
) -> RuntimeResult<()> {
    if status == HostStatus::BufferTooSmall.code() {
        return Err(invalid_data(
            operation,
            format!("android host {action} reported one unexpectedly small output buffer"),
        ));
    }

    let Some(status) = HostStatus::from_code(status) else {
        return Err(invalid_data(
            operation,
            format!("android host {action} failed with status code {status}"),
        ));
    };

    match status {
        HostStatus::Ok => Ok(()),
        HostStatus::NotSupported => Err(not_supported(operation)),
        HostStatus::InvalidArgument => Err(invalid_data(
            operation,
            format!("android host {action} reported one invalid argument"),
        )),
        HostStatus::NotFound => Err(io_not_found(
            operation,
            format!("android host {action} could not resolve one object"),
        )),
        HostStatus::PermissionDenied => Err(io_operation_error(
            operation,
            Some(crate::platform::diagnostic::PlatformErrorCode::IoPermissionDenied),
            format!("android host {action} was denied permission"),
        )),
        HostStatus::BufferTooSmall => Err(invalid_data(
            operation,
            format!("android host {action} reported one unexpectedly small output buffer"),
        )),
        HostStatus::Failed => Err(invalid_data(
            operation,
            format!("android host {action} failed"),
        )),
        HostStatus::WouldBlock => Err(io_would_block(
            operation,
            format!("android host {action} would block"),
        )),
    }
}
