use crate::diagnostic::RuntimeError;
use crate::host::HostStatus;
use crate::platform::core as core_platform;
use crate::platform::os::CredentialAuthenticationMechanism;
use crate::platform::os::credentials::core::{invalid_data, permission_denied};
use crate::runtime::BindingCallContext;

/// Mechanism code for one unknown host authentication method.
const AUTHENTICATION_MECHANISM_UNKNOWN_CODE: u32 = 1;
/// Mechanism code for one biometric host authentication method.
const AUTHENTICATION_MECHANISM_BIOMETRIC_CODE: u32 = 2;
/// Mechanism code for one device-credential host authentication method.
const AUTHENTICATION_MECHANISM_DEVICE_CREDENTIAL_CODE: u32 = 3;

/// Return one runtime identifier for Android host callback routing.
pub(super) fn host_session_id(
    binding: &BindingCallContext,
    _operation: &'static str,
) -> Result<u64, Box<RuntimeError>> {
    Ok(binding.worker().runtime_id.0)
}

/// Decode one Android host authentication mechanism code.
pub(super) fn decode_authentication_mechanism(
    mechanism_code: u32,
    operation: &'static str,
) -> Result<CredentialAuthenticationMechanism, Box<RuntimeError>> {
    let mechanism = match mechanism_code {
        AUTHENTICATION_MECHANISM_UNKNOWN_CODE => CredentialAuthenticationMechanism::Unknown,
        AUTHENTICATION_MECHANISM_BIOMETRIC_CODE => CredentialAuthenticationMechanism::Biometric,
        AUTHENTICATION_MECHANISM_DEVICE_CREDENTIAL_CODE => {
            CredentialAuthenticationMechanism::DeviceCredential
        }
        _ => {
            return Err(invalid_data(
                operation,
                format!(
                    "android host credentials authenticate returned unsupported mechanism code {mechanism_code}"
                ),
            ));
        }
    };

    Ok(mechanism)
}

/// Map one Android host callback status into one runtime result.
pub(super) fn host_status_result(
    status: u32,
    operation: &'static str,
    action: &'static str,
) -> Result<(), Box<RuntimeError>> {
    let Some(status) = HostStatus::from_code(status) else {
        return Err(invalid_data(
            operation,
            format!("android host credentials {action} failed with status code {status}"),
        ));
    };

    match status {
        HostStatus::Ok => Ok(()),
        HostStatus::NotSupported => Err(core_platform::not_supported(operation)),
        HostStatus::InvalidArgument => Err(invalid_data(
            operation,
            format!("android host credentials {action} reported one invalid argument"),
        )),
        HostStatus::NotFound => Err(core_platform::io_not_found(
            operation,
            format!("android host credentials {action} could not find one credential"),
        )),
        HostStatus::PermissionDenied => Err(permission_denied(
            operation,
            format!("android host credentials {action} was denied"),
        )),
        HostStatus::BufferTooSmall => Err(invalid_data(
            operation,
            format!(
                "android host credentials {action} reported one unexpectedly small output buffer"
            ),
        )),
        HostStatus::Failed => Err(invalid_data(
            operation,
            format!("android host credentials {action} failed"),
        )),
        HostStatus::WouldBlock => Err(invalid_data(
            operation,
            format!("android host credentials {action} would block unexpectedly"),
        )),
    }
}
