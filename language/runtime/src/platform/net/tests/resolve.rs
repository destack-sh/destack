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

        // either return at least one address or report one real lookup failure
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
                    &[PlatformErrorCode::NetDnsFailed],
                )?;
            }
        }

        Ok(())
    });
}

/// Resolve one named service into concrete socket addresses.
#[cfg(any(unix, windows))]
#[test]
fn test_net_resolve_named_service() {
    with_harness_context(|mut context| {
        let addresses = context
            .destack_net_resolve(context.resolve_query_parts_value(
                Some("localhost"),
                Some("http"),
                SocketFamily::IPv4,
                ResolveFlags(0),
            ))
            .and_then(|value| context.socket_addresses_from_value(value))?;

        // assert exact named-service semantics
        assert!(
            !addresses.is_empty(),
            "named-service resolve should return at least one address"
        );
        for (_host, port, family) in addresses {
            assert_eq!(port, 80, "http service should resolve to port 80");
            assert_eq!(family, SocketFamily::IPv4);
        }

        Ok(())
    });
}

/// Resolve one service-only query without requiring a host component.
#[cfg(any(unix, windows))]
#[test]
fn test_net_resolve_service_only_query() {
    with_harness_context(|mut context| {
        let addresses = context
            .destack_net_resolve(context.resolve_query_parts_value(
                None,
                Some("0"),
                SocketFamily::Unspecified,
                ResolveFlags(0),
            ))
            .and_then(|value| context.socket_addresses_from_value(value))?;

        // assert one valid service-only resolution
        assert!(
            !addresses.is_empty(),
            "service-only resolve should return at least one address"
        );
        for (host, port, _family) in addresses {
            assert!(
                !host.is_empty(),
                "service-only resolve should decode one host"
            );
            assert_eq!(port, 0, "service-only resolve should preserve port 0");
        }

        Ok(())
    });
}

/// Reject one resolve query that omits both host and service.
#[cfg(any(unix, windows))]
#[test]
fn test_net_resolve_rejects_missing_host_and_service() {
    with_harness_context(|mut context| {
        assert_platform_error_codes_with_privileged_policy::<()>(
            context
                .destack_net_resolve(context.resolve_query_parts_value(
                    None,
                    None,
                    SocketFamily::Unspecified,
                    ResolveFlags(0),
                ))
                .and_then(|value| context.socket_addresses_from_value(value))
                .map(|_| ()),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

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
                "127.0.local_id.1",
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
                    Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "addresses",
                        "resolve returned no loopback addresses",
                    ))
                    .boxed())
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
                    &[PlatformErrorCode::NetDnsFailed],
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
        let address = context.socket_address_value_for_host_port("127.0.local_id.1", 80)?;

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
                assert_eq!(host, "127.0.local_id.1");
                assert_eq!(service, "80");
            }
            Err(error) => {
                assert_platform_error_codes_with_privileged_policy::<()>(
                    Err(error),
                    &[PlatformErrorCode::NetDnsFailed],
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
        let address = context.socket_address_value_for_host_port("127.0.local_id.1", 80)?;

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
