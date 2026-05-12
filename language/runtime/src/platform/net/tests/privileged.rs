use super::with_harness_context;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::tests::platform::{error_code_from_runtime_error, is_privileged_test_mode};

const PRIVILEGED_PORTS: [u16; 20] = [
    1, 2, 3, 7, 9, 13, 17, 19, 21, 23, 25, 37, 42, 53, 67, 80, 110, 123, 143, 443,
];

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
            match context.destack_net_listen(
                context.socket_address_value_for_host_port("127.0.local_id.1", port)?,
                32,
            ) {
                Ok(listener) => {
                    context.destack_net_close_listener(listener)?;
                    return Ok(());
                }
                Err(error) => {
                    let code = error_code_from_runtime_error(&error);
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
                        "failed to bind privileged port {port} with unexpected error code {code:?}: {error:?}",
                    );
                }
            }
        }

        panic!("no free privileged port was available in the test port set");
    });
}
