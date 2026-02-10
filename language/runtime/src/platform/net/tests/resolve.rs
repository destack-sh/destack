use super::{allow_not_supported, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::{ResolveFlags, SocketFamily};

#[cfg(any(unix, windows))]
#[test]
fn test_net_resolve_localhost() {
    with_harness_context(|mut context| {
        allow_not_supported((|| {
            let addresses =
                context.resolve("localhost", 0, SocketFamily::Unspecified, ResolveFlags(0))?;

            assert!(!addresses.is_empty(), "resolve should return addresses");
            for (host, _port, _family) in addresses {
                assert!(!host.is_empty(), "resolved host should not be empty");
            }

            Ok(())
        })())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_net_reverse_lookup_localhost() {
    with_harness_context(|mut context| {
        allow_not_supported((|| {
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
        })())
    });
}
