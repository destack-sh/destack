use super::{assert_platform_error_codes, with_harness_context};
use crate::diagnostic::RuntimeError;
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::{ResolveFlags, ReverseLookupFlags, SocketFamily};

/// Resolve localhost into concrete socket addresses.
#[cfg(any(unix, windows))]
#[test]
fn test_net_resolve_localhost() {
    with_harness_context(|mut context| {
        let result = context
            .destack_net_resolve(context.resolve_query_value(
                "localhost",
                0,
                SocketFamily::Unspecified,
                ResolveFlags(0),
            ))
            .and_then(|value| context.socket_addresses_from_value(value));

        // either return at least one address or report not-supported
        match result {
            Ok(addresses) => {
                assert!(!addresses.is_empty(), "resolve should return addresses");
                for (host, _port, _family) in addresses {
                    assert!(!host.is_empty(), "resolved host should not be empty");
                }
            }
            Err(error) => {
                assert_platform_error_codes::<()>(
                    Err(error),
                    &[
                        PlatformErrorCode::NotSupported,
                        PlatformErrorCode::InvalidArgumentValue,
                    ],
                )?;
            }
        }

        Ok(())
    });
}

/// Reverse-resolve localhost loopback addresses.
#[cfg(any(unix, windows))]
#[test]
fn test_net_reverse_lookup_localhost() {
    with_harness_context(|mut context| {
        let resolved = context
            .destack_net_resolve(context.resolve_query_value(
                "127.0.0.1",
                0,
                SocketFamily::IPv4,
                ResolveFlags(0),
            ))
            .and_then(|value| context.socket_addresses_from_value(value));

        let result = match resolved {
            Ok(addresses) => {
                if let Some((host, port, family)) = addresses.first().cloned() {
                    context
                        .destack_net_reverse_lookup(
                            context.socket_address_value(&host, port, family)?,
                            ReverseLookupFlags(0),
                        )
                        .and_then(|value| context.reverse_lookup_names_from_value(value))
                } else {
                    Err(RuntimeError::from(PlatformError::not_supported("reverse lookup")).boxed())
                }
            }
            Err(error) => Err(error),
        };

        // either return names or one of the expected network lookup failures
        match result {
            Ok(names) => {
                assert!(!names.is_empty(), "reverse lookup should return names");
            }
            Err(error) => {
                assert_platform_error_codes::<()>(
                    Err(error),
                    &[
                        PlatformErrorCode::NetDnsFailed,
                        PlatformErrorCode::NotSupported,
                        PlatformErrorCode::InvalidArgumentValue,
                    ],
                )?;
            }
        }

        Ok(())
    });
}
