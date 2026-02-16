use super::with_harness_context;
use crate::platform::diagnostic::PlatformErrorCode;

const PRIVILEGED_PORTS: [u16; 20] = [
    1, 2, 3, 7, 9, 13, 17, 19, 21, 23, 25, 37, 42, 53, 67, 80, 110, 123, 143, 443,
];

fn is_privileged_test_mode() -> bool {
    let value = std::env::var("DESTACK_TEST_PRIVILEGED").unwrap_or_default();
    matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES")
}

/// Bind one privileged tcp port when privileged mode is enabled.
#[cfg(unix)]
#[test]
fn test_net_bind_privileged_port_succeeds_in_privileged_mode() {
    if !is_privileged_test_mode() {
        return;
    }

    with_harness_context(|mut context| {
        // iterate common privileged ports until one bind succeeds
        for port in PRIVILEGED_PORTS {
            match context.listen("127.0.0.1", port, 32) {
                Ok(listener) => {
                    context.close_listener(listener)?;
                    return Ok(());
                }
                Err(error) => {
                    let code = error.platform_error().map(|value| value.code);
                    if matches!(
                        code,
                        Some(PlatformErrorCode::IoPermissionDenied)
                            | Some(PlatformErrorCode::ProcessPermissionDenied)
                            | Some(PlatformErrorCode::SecurityDenied)
                    ) {
                        panic!(
                            "failed to bind privileged port {port}: privileged mode is enabled but operation was denied ({error:?})",
                        );
                    }
                    if code == Some(PlatformErrorCode::NetAddressInUse) {
                        continue;
                    }

                    panic!(
                        "failed to bind privileged port {port} with unexpected error code {:?}: {error:?}",
                        code
                    );
                }
            }
        }

        panic!("no free privileged port was available in the test port set");
    });
}
