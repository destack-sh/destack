use crate::diagnostic::RuntimeError;
use crate::host::{
    HOST_STATUS_FAILED, HOST_STATUS_INVALID_ARGUMENT, HOST_STATUS_NOT_FOUND,
    HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK, HOST_STATUS_PERMISSION_DENIED,
};
use crate::platform::core as core_platform;
use crate::platform::os::CredentialAuthenticationMechanism;
use crate::runtime::BindingCallContext;

use super::super::super::core::{invalid_data, permission_denied};

/// Mechanism code for one unknown host authentication method.
const AUTHENTICATION_MECHANISM_UNKNOWN_CODE: u32 = 1;
/// Mechanism code for one biometric host authentication method.
const AUTHENTICATION_MECHANISM_BIOMETRIC_CODE: u32 = 2;
/// Mechanism code for one device-credential host authentication method.
const AUTHENTICATION_MECHANISM_DEVICE_CREDENTIAL_CODE: u32 = 3;

/// Return one callback runtime identifier for Android host callback routing.
pub(super) fn callback_runtime_id(
    context: &BindingCallContext,
    operation: &'static str,
) -> Result<u64, Box<RuntimeError>> {
    let Some(runtime_id) = context.host().callback_runtime_id() else {
        return Err(core_platform::not_supported(operation));
    };

    Ok(runtime_id)
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
    if status == HOST_STATUS_OK {
        return Ok(());
    }

    if status == HOST_STATUS_NOT_SUPPORTED {
        return Err(core_platform::not_supported(operation));
    }

    if status == HOST_STATUS_INVALID_ARGUMENT {
        return Err(invalid_data(
            operation,
            format!("android host credentials {action} reported one invalid argument"),
        ));
    }

    if status == HOST_STATUS_NOT_FOUND {
        return Err(core_platform::io_not_found(
            operation,
            format!("android host credentials {action} could not find one credential"),
        ));
    }

    if status == HOST_STATUS_PERMISSION_DENIED {
        return Err(permission_denied(
            operation,
            format!("android host credentials {action} was denied"),
        ));
    }

    if status == HOST_STATUS_FAILED {
        return Err(invalid_data(
            operation,
            format!("android host credentials {action} failed"),
        ));
    }

    Err(invalid_data(
        operation,
        format!("android host credentials {action} failed with status code {status}"),
    ))
}
