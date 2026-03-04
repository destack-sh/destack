use super::{assert_platform_error_codes_with_privileged_policy, with_harness_context};
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
                assert_platform_error_codes_with_privileged_policy::<()>(
                    Err(error),
                    &[
                        PlatformErrorCode::NotSupported,
                        PlatformErrorCode::InvalidArgumentValue,
                        PlatformErrorCode::NetDnsFailed,
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
                assert!(
                    !addresses.is_empty(),
                    "resolve should return at least one loopback address"
                );
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
                assert_platform_error_codes_with_privileged_policy::<()>(
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

/// Reverse-lookup numeric host and service fields when numeric flags are enabled.
#[cfg(any(unix, windows))]
#[test]
fn test_net_reverse_lookup_numeric_host_and_service() {
    with_harness_context(|mut context| {
        // construct one numeric loopback address with a fixed service port
        let address = context.socket_address_value_for_host_port("127.0.0.1", 80)?;

        // request numeric host and numeric service rendering
        let flags = ReverseLookupFlags(0x1 | 0x2);
        let records = context
            .destack_net_reverse_lookup(address, flags)
            .and_then(|value| context.reverse_lookup_records_from_value(value));

        // assert numeric host and service output or accepted capability failures
        match records {
            Ok(records) => {
                assert!(!records.is_empty(), "reverse lookup should return records");
                let (host, service) = &records[0];
                assert_eq!(host, "127.0.0.1");
                assert_eq!(service, "80");
            }
            Err(error) => {
                assert_platform_error_codes_with_privileged_policy::<()>(
                    Err(error),
                    &[
                        PlatformErrorCode::NotSupported,
                        PlatformErrorCode::NetDnsFailed,
                    ],
                )?;
            }
        }

        Ok(())
    });
}

/// Reject unknown reverse-lookup flag bits.
#[cfg(any(unix, windows))]
#[test]
fn test_net_reverse_lookup_rejects_unknown_flag_bits() {
    with_harness_context(|mut context| {
        // construct one numeric loopback address for reverse lookup
        let address = context.socket_address_value_for_host_port("127.0.0.1", 80)?;

        // pass one undefined flag bit
        assert_platform_error_codes_with_privileged_policy::<()>(
            context
                .destack_net_reverse_lookup(address, ReverseLookupFlags(1 << 31))
                .and_then(|value| context.reverse_lookup_names_from_value(value))
                .map(|_| ()),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        Ok(())
    });
}
