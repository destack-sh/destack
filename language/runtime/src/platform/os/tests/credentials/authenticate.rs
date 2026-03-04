use super::super::with_harness_context;
#[cfg(any(target_vendor = "apple", target_os = "android", windows))]
use super::core::assert_not_not_supported_error;
#[cfg(target_os = "linux")]
use super::core::assert_not_supported_error;
use super::core::credential_authentication_options_value;
use crate::platform::os::CredentialAuthenticationRequirement;

/// Return notSupported on platforms where authenticate lane is intentionally unavailable.
#[cfg(target_os = "linux")]
#[test]
fn test_credentials_authenticate_reports_not_supported() {
    with_harness_context(|mut context| {
        let options = credential_authentication_options_value(
            &mut context,
            "Destack Test",
            "Credentials",
            "Authenticate to continue",
            CredentialAuthenticationRequirement::BiometricOrDeviceCredential,
        );
        let result = context.destack_os_credentials_authenticate(options);
        let error = match result {
            Ok(_) => panic!("authenticate should report notSupported"),
            Err(error) => error,
        };
        assert_not_supported_error(&error);

        Ok(())
    });
}

/// Avoid notSupported on platforms that implement host authentication lanes.
#[cfg(any(target_vendor = "apple", target_os = "android", windows))]
#[test]
fn test_credentials_authenticate_does_not_report_not_supported() {
    with_harness_context(|mut context| {
        let options = credential_authentication_options_value(
            &mut context,
            "Destack Test",
            "Credentials",
            "Authenticate to continue",
            CredentialAuthenticationRequirement::BiometricOrDeviceCredential,
        );
        let result = context.destack_os_credentials_authenticate(options);

        // verify host failure paths are not surfaced as notSupported on implemented lanes
        if let Err(error) = result {
            assert_not_not_supported_error(&error);
        }

        Ok(())
    });
}
