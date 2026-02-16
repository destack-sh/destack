use super::{assert_platform_error_code, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::{ResolveFlags, SocketFamily};

#[cfg(any(unix, windows))]
#[test]
fn test_net_resolve_localhost() {
    with_harness_context(|mut context| {
        let result = context.resolve("localhost", 0, SocketFamily::Unspecified, ResolveFlags(0));
        match result {
            Ok(addresses) => {
                assert!(!addresses.is_empty(), "resolve should return addresses");
                for (host, _port, _family) in addresses {
                    assert!(!host.is_empty(), "resolved host should not be empty");
                }
            }
            Err(error) => {
                assert_platform_error_code::<()>(Err(error), PlatformErrorCode::NotSupported)?;
            }
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_net_reverse_lookup_localhost() {
    with_harness_context(|mut context| {
        let result = context.reverse_lookup("127.0.0.1", 0, SocketFamily::IPv4);
        match result {
            Ok(names) => {
                assert!(!names.is_empty(), "reverse lookup should return names");
            }
            Err(error) => {
                let code = error.platform_error().map(|error| error.code);
                assert!(
                    matches!(
                        code,
                        Some(PlatformErrorCode::NetDnsFailed)
                            | Some(PlatformErrorCode::Io)
                            | Some(PlatformErrorCode::Net)
                            | Some(PlatformErrorCode::NotSupported)
                    ),
                    "unexpected reverse lookup error code: {code:?}",
                );
            }
        }

        Ok(())
    });
}
