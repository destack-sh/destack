#[cfg(windows)]
use super::with_harness_context_with_runtime_options;
use super::{assert_platform_error_code, assert_platform_error_codes, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::{
    PacketCaptureOptions, PacketCaptureOptionsVm, PacketFanoutMode, PacketFanoutOptions,
    PacketRingOptions, PacketTimestampMode, SocketFamily,
};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::platform::net::{RouteEntry, RouteEntryVm, RouteKind};
use crate::platform::resource::{ResourceId, SocketHandle};
#[cfg(windows)]
use destack_workspace::RuntimeOptions;

/// Return route-list snapshots where supported and surface notSupported elsewhere.
#[test]
fn test_net_route_list_support_matches_platform() {
    with_harness_context(|mut context| {
        let result = context.destack_net_route_list(SocketFamily::IPv4);

        #[cfg(any(target_os = "linux", target_os = "macos", windows))]
        {
            let _routes = result?;
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
        {
            assert_platform_error_code(result, PlatformErrorCode::NotSupported)?;
        }

        Ok(())
    });
}

/// Reject unspecified route-list family on supported route backends.
#[test]
fn test_net_route_list_rejects_unspecified_family_when_backend_is_available() {
    with_harness_context(|mut context| {
        let result = context.destack_net_route_list(SocketFamily::Unspecified);

        #[cfg(any(target_os = "linux", target_os = "macos", windows))]
        {
            assert_platform_error_code(result, PlatformErrorCode::InvalidArgumentValue)?;
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
        {
            assert_platform_error_code(result, PlatformErrorCode::NotSupported)?;
        }

        Ok(())
    });
}

/// Reject packet-open promiscuous mode without one interface index on Linux.
#[test]
fn test_net_packet_open_requires_interface_for_promiscuous_mode() {
    with_harness_context(|mut context| {
        let options = PacketCaptureOptions {
            interface_index: 0,
            snap_length: 4096,
            timeout_ms: 0,
            promiscuous: true,
        };
        let options = if context.vm_context.is_some() {
            context.harness_value_vm::<PacketCaptureOptions, PacketCaptureOptionsVm>(options)
        } else {
            context.harness_value(options)
        };
        let result = context.destack_net_packet_open(options);

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            assert_platform_error_code(result, PlatformErrorCode::InvalidArgumentValue)?;
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            assert_platform_error_code(result, PlatformErrorCode::NotSupported)?;
        }

        Ok(())
    });
}

/// Return notSupported for Windows packet lanes when no host backend is configured.
#[cfg(windows)]
#[test]
fn test_net_packet_open_windows_reports_not_supported_when_backend_disabled() {
    with_harness_context_with_runtime_options(
        |_| {},
        |mut context| {
            let options = PacketCaptureOptions {
                interface_index: 0,
                snap_length: 4096,
                timeout_ms: 0,
                promiscuous: false,
            };
            let options = if context.vm_context.is_some() {
                context.harness_value_vm::<PacketCaptureOptions, PacketCaptureOptionsVm>(options)
            } else {
                context.harness_value(options)
            };
            let result = context.destack_net_packet_open(options);

            assert_platform_error_code(result, PlatformErrorCode::NotSupported)?;

            Ok(())
        },
    );
}

/// Validate Windows packet backend option routes into backend validation when enabled.
#[cfg(windows)]
#[test]
fn test_net_packet_open_windows_enabled_runs_backend_validation() {
    with_harness_context_with_runtime_options(
        |runtime_options: &mut RuntimeOptions| {
            runtime_options
                .platform
                .windows
                .net
                .windows_packet_backend_enabled = Some(true);
        },
        |mut context| {
            let options = PacketCaptureOptions {
                interface_index: 0,
                snap_length: 4096,
                timeout_ms: 0,
                promiscuous: true,
            };
            let options = if context.vm_context.is_some() {
                context.harness_value_vm::<PacketCaptureOptions, PacketCaptureOptionsVm>(options)
            } else {
                context.harness_value(options)
            };
            let result = context.destack_net_packet_open(options);

            assert_platform_error_code(result, PlatformErrorCode::InvalidArgumentValue)?;

            Ok(())
        },
    );
}

/// Reject IPv6 route mutations with invalid prefix lengths on linux.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn test_net_route_add_delete_ipv6_rejects_invalid_prefix_length() {
    with_harness_context(|mut context| {
        // construct one invalid IPv6 route entry for add and delete
        let destination = context.socket_address_value("2001:db8::", 0, SocketFamily::IPv6)?;
        let gateway = context.socket_address_value("::", 0, SocketFamily::IPv6)?;
        let route = match (destination, gateway) {
            (super::HarnessValue::Native(destination), super::HarnessValue::Native(gateway)) => {
                context.harness_value(RouteEntry {
                    family: SocketFamily::IPv6,
                    destination,
                    prefix_length: 129,
                    gateway,
                    interface_index: 0,
                    metric: 0,
                    kind: RouteKind::Unicast,
                })
            }
            (super::HarnessValue::Vm(destination), super::HarnessValue::Vm(gateway)) => context
                .harness_value_vm(RouteEntryVm {
                    family: SocketFamily::IPv6,
                    destination,
                    prefix_length: 129,
                    gateway,
                    interface_index: 0,
                    metric: 0,
                    kind: RouteKind::Unicast,
                }),
            _ => unreachable!("mixed harness values are not possible"),
        };
        assert_platform_error_code(
            context.destack_net_route_add(route),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        // construct one invalid IPv6 route entry again for delete
        let destination = context.socket_address_value("2001:db8::", 0, SocketFamily::IPv6)?;
        let gateway = context.socket_address_value("::", 0, SocketFamily::IPv6)?;
        let route = match (destination, gateway) {
            (super::HarnessValue::Native(destination), super::HarnessValue::Native(gateway)) => {
                context.harness_value(RouteEntry {
                    family: SocketFamily::IPv6,
                    destination,
                    prefix_length: 129,
                    gateway,
                    interface_index: 0,
                    metric: 0,
                    kind: RouteKind::Unicast,
                })
            }
            (super::HarnessValue::Vm(destination), super::HarnessValue::Vm(gateway)) => context
                .harness_value_vm(RouteEntryVm {
                    family: SocketFamily::IPv6,
                    destination,
                    prefix_length: 129,
                    gateway,
                    interface_index: 0,
                    metric: 0,
                    kind: RouteKind::Unicast,
                }),
            _ => unreachable!("mixed harness values are not possible"),
        };
        assert_platform_error_code(
            context.destack_net_route_delete(route),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

/// Probe raw-socket lanes and accept explicit unsupported or permission-denied outcomes.
#[cfg(any(unix, windows))]
#[test]
fn test_net_raw_socket_and_header_included_lanes() {
    with_harness_context(|mut context| {
        // open one raw socket and handle host capability outcomes explicitly
        let socket = context.destack_net_raw_socket(SocketFamily::IPv4, libc::IPPROTO_RAW);
        let socket = match socket {
            Ok(socket) => socket,
            Err(error) => {
                assert_platform_error_codes::<SocketHandle>(
                    Err(error),
                    &[
                        PlatformErrorCode::IoPermissionDenied,
                        PlatformErrorCode::NotSupported,
                        PlatformErrorCode::IoInvalidData,
                        PlatformErrorCode::NetUnsupportedProtocol,
                        PlatformErrorCode::NetAddressNotAvailable,
                        PlatformErrorCode::InvalidArgumentValue,
                        PlatformErrorCode::Net,
                        PlatformErrorCode::Io,
                    ],
                )?;
                return Ok(());
            }
        };

        // toggle header-included mode or report explicit unsupported outcomes
        let header_result = context.destack_net_raw_set_header_included(socket, true);
        if let Err(error) = header_result {
            assert_platform_error_codes::<()>(
                Err(error),
                &[
                    PlatformErrorCode::NotSupported,
                    PlatformErrorCode::IoInvalidData,
                    PlatformErrorCode::NetUnsupportedProtocol,
                    PlatformErrorCode::Net,
                    PlatformErrorCode::Io,
                ],
            )?;
        }

        // close the socket
        context.destack_net_close(socket)?;

        Ok(())
    });
}

/// Probe packet-control lanes with one invalid handle and require explicit error contracts.
#[cfg(any(unix, windows))]
#[test]
fn test_net_packet_control_invalid_handle_contracts() {
    with_harness_context(|mut context| {
        // prepare one invalid packet socket handle
        let invalid_socket = SocketHandle(ResourceId(0));

        // build one packet fanout options payload
        let fanout_options = PacketFanoutOptions {
            group_id: 1,
            mode: PacketFanoutMode::Hash,
            flags: 0,
        };
        let fanout_options = if context.vm_context.is_some() {
            context.harness_value_vm::<PacketFanoutOptions, PacketFanoutOptions>(fanout_options)
        } else {
            context.harness_value(fanout_options)
        };

        // build one packet ring options payload
        let ring_options = PacketRingOptions {
            block_size: 4096,
            block_count: 1,
            frame_size: 2048,
            frame_count: 1,
            retire_timeout_ms: 0,
        };
        let ring_options_rx = if context.vm_context.is_some() {
            context.harness_value_vm::<PacketRingOptions, PacketRingOptions>(ring_options)
        } else {
            context.harness_value(ring_options)
        };
        let ring_options_tx = if context.vm_context.is_some() {
            context.harness_value_vm::<PacketRingOptions, PacketRingOptions>(ring_options)
        } else {
            context.harness_value(ring_options)
        };

        // build one reusable packet payload
        let packet_payload = context.bytes_slice_value(b"destack-packet")?;
        let filter_payload = context.bytes_slice_value(&[0x06, 0x00, 0x00, 0x00])?;

        // packet timestamping mode
        assert_platform_error_codes(
            context.destack_net_packet_set_timestamp_mode(
                invalid_socket,
                PacketTimestampMode::Software,
            ),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoInvalidData,
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::NetAddressNotAvailable,
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::Net,
                PlatformErrorCode::Io,
            ],
        )?;

        // packet filter lanes
        assert_platform_error_codes(
            context.destack_net_packet_set_filter(invalid_socket, filter_payload),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoInvalidData,
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::NetAddressNotAvailable,
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::Net,
                PlatformErrorCode::Io,
            ],
        )?;
        assert_platform_error_codes(
            context.destack_net_packet_clear_filter(invalid_socket),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoInvalidData,
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::NetAddressNotAvailable,
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::Net,
                PlatformErrorCode::Io,
            ],
        )?;

        // packet fanout lanes
        assert_platform_error_codes(
            context.destack_net_packet_set_fanout(invalid_socket, fanout_options),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoInvalidData,
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::NetAddressNotAvailable,
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::Net,
                PlatformErrorCode::Io,
            ],
        )?;
        assert_platform_error_codes(
            context.destack_net_packet_clear_fanout(invalid_socket),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoInvalidData,
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::NetAddressNotAvailable,
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::Net,
                PlatformErrorCode::Io,
            ],
        )?;

        // packet ring lanes
        assert_platform_error_codes(
            context.destack_net_packet_set_rx_ring(invalid_socket, ring_options_rx),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoInvalidData,
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::NetAddressNotAvailable,
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::Net,
                PlatformErrorCode::Io,
            ],
        )?;
        assert_platform_error_codes(
            context.destack_net_packet_set_tx_ring(invalid_socket, ring_options_tx),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoInvalidData,
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::NetAddressNotAvailable,
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::Net,
                PlatformErrorCode::Io,
            ],
        )?;
        assert_platform_error_codes(
            context.destack_net_packet_clear_ring(invalid_socket),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoInvalidData,
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::NetAddressNotAvailable,
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::Net,
                PlatformErrorCode::Io,
            ],
        )?;

        // packet data and stats lanes
        assert_platform_error_codes(
            context.destack_net_packet_receive(invalid_socket, packet_payload),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoInvalidData,
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::NetAddressNotAvailable,
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::Net,
                PlatformErrorCode::Io,
                PlatformErrorCode::IoWouldBlock,
            ],
        )?;
        let packet_payload = context.bytes_slice_value(b"destack-packet")?;
        assert_platform_error_codes(
            context.destack_net_packet_send(invalid_socket, packet_payload),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoInvalidData,
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::NetAddressNotAvailable,
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::Net,
                PlatformErrorCode::Io,
                PlatformErrorCode::IoWouldBlock,
            ],
        )?;
        assert_platform_error_codes(
            context.destack_net_packet_stats(invalid_socket),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoInvalidData,
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::NetAddressNotAvailable,
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::Net,
                PlatformErrorCode::Io,
            ],
        )?;

        Ok(())
    });
}
