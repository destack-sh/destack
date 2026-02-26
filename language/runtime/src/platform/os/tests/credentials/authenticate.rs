use super::super::with_harness_context;
#[cfg(any(target_os = "linux", windows))]
use super::core::assert_platform_error_code;
use super::core::credential_authentication_options_value;
use crate::platform::diagnostic::PlatformErrorCode;

/// Return notSupported on platforms where authenticate lane is intentionally unavailable.
#[cfg(any(target_os = "linux", windows))]
#[test]
fn test_credentials_authenticate_reports_not_supported() {
    with_harness_context(|mut context| {
        let options = credential_authentication_options_value(
            &mut context,
            "Destack Test",
            "Credentials",
            "Authenticate to continue",
            true,
        );
        let result = context.destack_os_credentials_authenticate(options);
        let error = match result {
            Ok(_) => panic!("authenticate should report notSupported"),
            Err(error) => error,
        };
        assert_platform_error_code(&error, PlatformErrorCode::NotSupported);

        Ok(())
    });
}

/// Avoid notSupported on platforms that implement host authentication lanes.
#[cfg(any(target_vendor = "apple", target_os = "android"))]
#[test]
fn test_credentials_authenticate_does_not_report_not_supported() {
    with_harness_context(|mut context| {
        let options = credential_authentication_options_value(
            &mut context,
            "Destack Test",
            "Credentials",
            "Authenticate to continue",
            true,
        );
        let result = context.destack_os_credentials_authenticate(options);

        // verify host failure paths are not surfaced as notSupported on implemented lanes
        if let Err(error) = result {
            let platform_error = error
                .platform_error()
                .expect("expected one platform error payload");
            assert_ne!(platform_error.code, PlatformErrorCode::NotSupported);
        }

        Ok(())
    });
}
