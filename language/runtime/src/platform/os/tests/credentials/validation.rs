use super::super::with_harness_context;
use super::core::{
    assert_platform_error_code, credential_authentication_options_value, credential_query_value,
    credential_write_options_value, string_value,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::{
    CredentialAccessibility, CredentialAuthenticationPolicy, CredentialAuthenticationRequirement,
};

/// Verify credentials write rejects empty payload bytes.
#[cfg(any(unix, windows))]
#[test]
fn test_credentials_write_rejects_empty_payload() {
    with_harness_context(|mut context| {
        let options = credential_write_options_value(
            &mut context,
            "destack.validation",
            "write-empty",
            "",
            &[],
            CredentialAccessibility::HostDefault,
            CredentialAuthenticationPolicy::None,
            true,
        )?;
        let result = context.destack_os_credentials_write(options);
        let error = match result {
            Ok(_) => panic!("write with empty payload should fail"),
            Err(error) => error,
        };

        assert_platform_error_code(&error, PlatformErrorCode::InvalidArgumentValue);

        Ok(())
    });
}

/// Verify credentials read rejects empty service identifiers.
#[cfg(any(unix, windows))]
#[test]
fn test_credentials_read_rejects_empty_service() {
    with_harness_context(|mut context| {
        let query = credential_query_value(&mut context, "", "destack-account", "", false);
        let result = context.destack_os_credentials_read(query);
        let error = match result {
            Ok(_) => panic!("read with empty service should fail"),
            Err(error) => error,
        };

        assert_platform_error_code(&error, PlatformErrorCode::InvalidArgumentValue);

        Ok(())
    });
}

/// Verify credentials contains rejects empty account identifiers.
#[cfg(any(unix, windows))]
#[test]
fn test_credentials_contains_rejects_empty_account() {
    with_harness_context(|mut context| {
        let service = string_value(&mut context, "destack.validation");
        let account = string_value(&mut context, "");
        let access_group = string_value(&mut context, "");
        let result = context.destack_os_credentials_contains(service, account, access_group);
        let error = match result {
            Ok(_) => panic!("contains with empty account should fail"),
            Err(error) => error,
        };

        assert_platform_error_code(&error, PlatformErrorCode::InvalidArgumentValue);

        Ok(())
    });
}

/// Verify credentials authenticate rejects fully empty prompt payloads.
#[cfg(any(unix, windows))]
#[test]
fn test_credentials_authenticate_rejects_empty_prompt() {
    with_harness_context(|mut context| {
        let options = credential_authentication_options_value(
            &mut context,
            "",
            "",
            "",
            CredentialAuthenticationRequirement::BiometricOrDeviceCredential,
        );
        let result = context.destack_os_credentials_authenticate(options);
        let error = match result {
            Ok(_) => panic!("authenticate with empty prompt should fail"),
            Err(error) => error,
        };

        assert_platform_error_code(&error, PlatformErrorCode::InvalidArgumentValue);

        Ok(())
    });
}
