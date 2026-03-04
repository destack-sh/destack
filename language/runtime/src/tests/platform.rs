use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;

/// Extract one platform error code from one failed runtime result.
pub(crate) fn error_code_from_result<T>(
    result: RuntimeResult<T>,
) -> RuntimeResult<PlatformErrorCode> {
    match result {
        Ok(_) => Err(
            RuntimeError::from(PlatformError::invalid_argument("operation should fail")).boxed(),
        ),
        Err(error) => {
            let platform = error.platform_error().ok_or_else(|| {
                RuntimeError::from(PlatformError::invalid_argument(
                    "missing platform error payload",
                ))
                .boxed()
            })?;

            Ok(platform.code)
        }
    }
}

/// Extract one optional platform error code from one runtime error.
pub(crate) fn error_code_from_runtime_error(error: &RuntimeError) -> Option<PlatformErrorCode> {
    error.platform_error().map(|platform| platform.code)
}

/// Assert one failed runtime result with one exact platform code.
pub(crate) fn assert_platform_error_code<T>(
    result: RuntimeResult<T>,
    expected: PlatformErrorCode,
) -> RuntimeResult<()> {
    let actual = error_code_from_result(result)?;
    assert_eq!(actual, expected);

    Ok(())
}

/// Assert one failed runtime result with one expected platform code set.
pub(crate) fn assert_platform_error_codes<T>(
    result: RuntimeResult<T>,
    expected: &[PlatformErrorCode],
) -> RuntimeResult<()> {
    let actual = error_code_from_result(result)?;
    assert!(
        expected.contains(&actual),
        "unexpected platform error code {actual:?}, expected one of {expected:?}"
    );

    Ok(())
}

/// Assert one failed runtime result with one exact platform code in privileged-mode policy.
pub(crate) fn assert_platform_error_code_with_privileged_policy<T>(
    result: RuntimeResult<T>,
    expected: PlatformErrorCode,
) -> RuntimeResult<()> {
    let actual = error_code_from_result(result)?;
    assert_no_permission_denied_in_privileged_mode(actual, &[expected]);
    assert_eq!(actual, expected);

    Ok(())
}

/// Assert one failed runtime result with one expected platform code set in privileged-mode policy.
pub(crate) fn assert_platform_error_codes_with_privileged_policy<T>(
    result: RuntimeResult<T>,
    expected: &[PlatformErrorCode],
) -> RuntimeResult<()> {
    let actual = error_code_from_result(result)?;
    assert_no_permission_denied_in_privileged_mode(actual, expected);
    assert!(
        expected.contains(&actual),
        "unexpected platform error code {actual:?}, expected one of {expected:?}"
    );

    Ok(())
}

/// Assert one result is ok or fails with one expected platform code set.
pub(crate) fn assert_ok_or_expected_error<T>(
    result: RuntimeResult<T>,
    expected: &[PlatformErrorCode],
) -> RuntimeResult<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) => {
            let Some(code) = error_code_from_runtime_error(&error) else {
                return Err(error);
            };

            if expected.contains(&code) {
                return Ok(None);
            }

            Err(error)
        }
    }
}

/// Return whether one optional platform error code is not-supported.
pub(crate) fn is_not_supported_code(code: Option<PlatformErrorCode>) -> bool {
    code == Some(PlatformErrorCode::NotSupported)
}

/// Return whether one concrete platform error code is not-supported.
pub(crate) fn is_not_supported_platform_code(code: PlatformErrorCode) -> bool {
    is_not_supported_code(Some(code))
}

/// Assert one failed runtime result carries not-supported.
pub(crate) fn assert_not_supported_result<T>(result: RuntimeResult<T>) -> RuntimeResult<()> {
    let code = error_code_from_result(result)?;
    assert!(is_not_supported_platform_code(code));

    Ok(())
}

/// Return one optional value, mapping not-supported errors to none.
pub(crate) fn result_or_skip_not_supported<T>(
    result: RuntimeResult<T>,
) -> RuntimeResult<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) => {
            if is_not_supported_code(error_code_from_runtime_error(&error)) {
                return Ok(None);
            }

            Err(error)
        }
    }
}

/// Assert one runtime error carries not-supported.
pub(crate) fn assert_not_supported_error(error: &RuntimeError) {
    let code = error_code_from_runtime_error(error).expect("expected one platform error payload");
    assert!(is_not_supported_platform_code(code));
}

/// Assert one runtime error does not carry not-supported.
pub(crate) fn assert_not_not_supported_error(error: &RuntimeError) {
    let code = error_code_from_runtime_error(error).expect("expected one platform error payload");
    assert!(!is_not_supported_platform_code(code));
}

/// Assert one concrete platform error code carries not-supported.
pub(crate) fn assert_not_supported_platform_code(code: PlatformErrorCode) {
    assert!(is_not_supported_platform_code(code));
}

/// Assert one concrete platform error code does not carry not-supported.
pub(crate) fn assert_not_not_supported_platform_code(code: PlatformErrorCode) {
    assert!(!is_not_supported_platform_code(code));
}

/// Assert one runtime error carries one exact platform code.
pub(crate) fn assert_runtime_error_code(error: &RuntimeError, expected: PlatformErrorCode) {
    let code = error_code_from_runtime_error(error).expect("expected one platform error payload");
    assert_eq!(code, expected);
}

/// Fail privileged runs when assertions observe permission-denied errors.
fn assert_no_permission_denied_in_privileged_mode(
    observed: PlatformErrorCode,
    expected: &[PlatformErrorCode],
) {
    if !is_privileged_test_mode() {
        return;
    }

    if is_permission_denied_code(observed) {
        panic!(
            "permission-denied error {observed:?} is not allowed when DESTACK_TEST_PRIVILEGED=1 (expected one of {expected:?})",
        );
    }
}

/// Return whether one process environment enables privileged test mode.
pub(crate) fn is_privileged_test_mode() -> bool {
    let value = std::env::var("DESTACK_TEST_PRIVILEGED").unwrap_or_default();
    matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES")
}

/// Return whether one platform code represents a permission denial.
fn is_permission_denied_code(code: PlatformErrorCode) -> bool {
    matches!(
        code,
        PlatformErrorCode::IoPermissionDenied
            | PlatformErrorCode::ProcessPermissionDenied
            | PlatformErrorCode::SecurityDenied
    )
}
