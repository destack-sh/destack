#[cfg(any(target_os = "linux", target_os = "macos"))]
use super::HarnessValue;
#[cfg(windows)]
use super::assert_not_supported_result;
#[cfg(windows)]
use super::with_harness_context_with_runtime_options;
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
use super::{
    assert_platform_error_code_with_privileged_policy,
    assert_platform_error_codes_with_privileged_policy, with_harness_context,
};
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
use super::{assert_platform_error_codes_with_privileged_policy, with_harness_context};
use crate::diagnostic::RuntimeResult;
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
use crate::platform::net::RouteKind;
#[cfg(target_os = "linux")]
use crate::platform::net::{
    PACKET_BACKEND_CAP_CAPTURE, PACKET_BACKEND_CAP_FANOUT, PACKET_BACKEND_CAP_FILTER,
    PACKET_BACKEND_CAP_RING, PACKET_BACKEND_CAP_SEND, PACKET_BACKEND_CAP_TIMESTAMP,
};
#[cfg(windows)]
use crate::platform::net::{
    PACKET_BACKEND_CAP_CAPTURE, PACKET_BACKEND_CAP_FILTER, PACKET_BACKEND_CAP_SEND,
};
#[cfg(target_os = "macos")]
use crate::platform::net::{
    PACKET_BACKEND_CAP_CAPTURE, PACKET_BACKEND_CAP_FILTER, PACKET_BACKEND_CAP_SEND,
    PACKET_BACKEND_CAP_TIMESTAMP,
};
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
use crate::platform::net::{
    PacketBackend, PacketBackendSelectionPolicy, PacketCaptureOptions, PacketCaptureOptionsVm,
};
use crate::platform::net::{
    PacketFanoutMode, PacketFanoutOptions, PacketRingOptions, PacketTimestampMode, SocketFamily,
};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::platform::net::{RouteEntry, RouteEntryVm};
use crate::platform::resource::{ResourceId, SocketHandle};
#[cfg(windows)]
use destack_workspace::{PlatformWindowsPacketBackend, RuntimeOptions};
#[cfg(windows)]
use windows_sys::Win32::Networking::WinSock::IPPROTO_RAW;

/// Raw protocol identifier used for raw-socket tests.
#[cfg(unix)]
const RAW_PROTOCOL: i32 = libc::IPPROTO_RAW;

/// Raw protocol identifier used for raw-socket tests.
#[cfg(windows)]
const RAW_PROTOCOL: i32 = IPPROTO_RAW;

/// Assert one packet-control call fails with one explicit invalid-handle or unsupported contract.
fn assert_packet_control_error<T>(result: RuntimeResult<T>) -> RuntimeResult<()> {
    assert_platform_error_codes_with_privileged_policy(
        result,
        &[
            PlatformErrorCode::NotSupported,
            PlatformErrorCode::InvalidArgumentValue,
        ],
    )
}

/// Return the exact capability bitset for the active host packet backend.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
fn expected_packet_backend_capabilities() -> u64 {
    #[cfg(target_os = "linux")]
    {
        return PACKET_BACKEND_CAP_CAPTURE.0
            | PACKET_BACKEND_CAP_SEND.0
            | PACKET_BACKEND_CAP_TIMESTAMP.0
            | PACKET_BACKEND_CAP_FILTER.0
            | PACKET_BACKEND_CAP_FANOUT.0
            | PACKET_BACKEND_CAP_RING.0;
    }

    #[cfg(target_os = "macos")]
    {
        PACKET_BACKEND_CAP_CAPTURE.0
            | PACKET_BACKEND_CAP_SEND.0
            | PACKET_BACKEND_CAP_TIMESTAMP.0
            | PACKET_BACKEND_CAP_FILTER.0
    }

    #[cfg(windows)]
    {
        return PACKET_BACKEND_CAP_CAPTURE.0
            | PACKET_BACKEND_CAP_SEND.0
            | PACKET_BACKEND_CAP_FILTER.0;
    }
}

/// Return the primary backend enum for the active host packet backend.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
fn expected_packet_backend() -> PacketBackend {
    #[cfg(target_os = "linux")]
    {
        return PacketBackend::AfPacket;
    }

    #[cfg(target_os = "macos")]
    {
        PacketBackend::Bpf
    }

    #[cfg(windows)]
    {
        return PacketBackend::WinRawSocket;
    }
}

/// Return route-list snapshots where supported and surface notSupported elsewhere.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
#[test]
fn test_net_route_list_support_matches_platform() {
    with_harness_context(|mut context| {
        // load both route families and decode every returned row
        let routes_v4 = context.destack_net_route_list(SocketFamily::IPv4)?;
        let routes_v4 = context.route_entries_from_value(routes_v4)?;
        let routes_v6 = context.destack_net_route_list(SocketFamily::IPv6)?;
        let routes_v6 = context.route_entries_from_value(routes_v6)?;

        // validate ipv4 route family and prefix invariants for every row
        for route in routes_v4 {
            assert_eq!(route.family, SocketFamily::IPv4);
            assert!(route.prefix_length <= 32);
            assert!(matches!(
                route.kind,
                RouteKind::Unicast
                    | RouteKind::Local
                    | RouteKind::Broadcast
                    | RouteKind::Multicast
                    | RouteKind::Blackhole
            ));
        }

        // validate ipv6 route family and prefix invariants for every row
        for route in routes_v6 {
            assert_eq!(route.family, SocketFamily::IPv6);
            assert!(route.prefix_length <= 128);
            assert!(matches!(
                route.kind,
                RouteKind::Unicast
                    | RouteKind::Local
                    | RouteKind::Broadcast
                    | RouteKind::Multicast
                    | RouteKind::Blackhole
            ));
        }

        Ok(())
    });
}

/// Reject unspecified route-list family on supported route backends.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
#[test]
fn test_net_route_list_rejects_unspecified_family_when_backend_is_available() {
    with_harness_context(|mut context| {
        let result = context.destack_net_route_list(SocketFamily::Unspecified);
        assert_platform_error_code_with_privileged_policy(
            result,
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

/// Reject packet-open promiscuous mode without one interface index on Linux.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn test_net_packet_open_requires_interface_for_promiscuous_mode() {
    with_harness_context(|mut context| {
        let options = PacketCaptureOptions {
            backend: PacketBackend::Auto,
            backend_policy: PacketBackendSelectionPolicy::AllowFallback,
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
        assert_platform_error_code_with_privileged_policy(
            result,
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

/// Reject one zero snap length for packet capture on every backend.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
#[test]
fn test_net_packet_open_rejects_zero_snap_length() {
    #[cfg(windows)]
    with_harness_context_with_runtime_options(
        |runtime_options: &mut RuntimeOptions| {
            runtime_options.platform.windows.net_packet_backend =
                PlatformWindowsPacketBackend::RawSocket;
        },
        |mut context| {
            let options = PacketCaptureOptions {
                backend: PacketBackend::Auto,
                backend_policy: PacketBackendSelectionPolicy::AllowFallback,
                interface_index: 0,
                snap_length: 0,
                timeout_ms: 0,
                promiscuous: false,
            };
            let options = if context.vm_context.is_some() {
                context.harness_value_vm::<PacketCaptureOptions, PacketCaptureOptionsVm>(options)
            } else {
                context.harness_value(options)
            };

            assert_platform_error_code_with_privileged_policy(
                context.destack_net_packet_open(options),
                PlatformErrorCode::InvalidArgumentValue,
            )?;

            Ok(())
        },
    );

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    with_harness_context(|mut context| {
        let options = PacketCaptureOptions {
            backend: PacketBackend::Auto,
            backend_policy: PacketBackendSelectionPolicy::AllowFallback,
            interface_index: 0,
            snap_length: 0,
            timeout_ms: 0,
            promiscuous: false,
        };
        let options = if context.vm_context.is_some() {
            context.harness_value_vm::<PacketCaptureOptions, PacketCaptureOptionsVm>(options)
        } else {
            context.harness_value(options)
        };

        assert_platform_error_code_with_privileged_policy(
            context.destack_net_packet_open(options),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

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
                backend: PacketBackend::Auto,
                backend_policy: PacketBackendSelectionPolicy::AllowFallback,
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

            assert_not_supported_result(result)?;

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
            runtime_options.platform.windows.net_packet_backend =
                PlatformWindowsPacketBackend::RawSocket;
        },
        |mut context| {
            let options = PacketCaptureOptions {
                backend: PacketBackend::Auto,
                backend_policy: PacketBackendSelectionPolicy::AllowFallback,
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

            assert_platform_error_code_with_privileged_policy(
                result,
                PlatformErrorCode::InvalidArgumentValue,
            )?;

            Ok(())
        },
    );
}

/// Reject one strict packet-backend request when the requested backend is unavailable.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
#[test]
fn test_net_packet_open_strict_backend_selection_rejects_unavailable_backend() {
    #[cfg(target_os = "linux")]
    let strict_backend = PacketBackend::Bpf;
    #[cfg(target_os = "macos")]
    let strict_backend = PacketBackend::AfPacket;
    #[cfg(windows)]
    let strict_backend = PacketBackend::Bpf;

    with_harness_context(|mut context| {
        let options = PacketCaptureOptions {
            backend: strict_backend,
            backend_policy: PacketBackendSelectionPolicy::Strict,
            interface_index: 1,
            snap_length: 4096,
            timeout_ms: 0,
            promiscuous: false,
        };
        let options = if context.vm_context.is_some() {
            context.harness_value_vm::<PacketCaptureOptions, PacketCaptureOptionsVm>(options)
        } else {
            context.harness_value(options)
        };

        assert_platform_error_code_with_privileged_policy(
            context.destack_net_packet_open(options),
            PlatformErrorCode::NotSupported,
        )?;

        Ok(())
    });
}

/// Allow one unavailable packet-backend request to fall back to the active backend.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
#[test]
fn test_net_packet_open_allow_fallback_uses_available_backend() {
    #[cfg(target_os = "linux")]
    let requested_backend = PacketBackend::Bpf;
    #[cfg(target_os = "macos")]
    let requested_backend = PacketBackend::AfPacket;
    #[cfg(windows)]
    let requested_backend = PacketBackend::Bpf;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    with_harness_context(|mut context| {
        let options = PacketCaptureOptions {
            backend: requested_backend,
            backend_policy: PacketBackendSelectionPolicy::AllowFallback,
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

        assert_platform_error_code_with_privileged_policy(
            context.destack_net_packet_open(options),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });

    #[cfg(windows)]
    with_harness_context_with_runtime_options(
        |runtime_options: &mut RuntimeOptions| {
            runtime_options.platform.windows.net_packet_backend =
                PlatformWindowsPacketBackend::RawSocket;
        },
        |mut context| {
            let options = PacketCaptureOptions {
                backend: requested_backend,
                backend_policy: PacketBackendSelectionPolicy::AllowFallback,
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

            assert_platform_error_code_with_privileged_policy(
                context.destack_net_packet_open(options),
                PlatformErrorCode::InvalidArgumentValue,
            )?;

            Ok(())
        },
    );
}

/// Report exact packet backend capability flags for the active host backend.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
#[test]
fn test_net_packet_backend_list_reports_exact_capabilities() {
    #[cfg(windows)]
    with_harness_context_with_runtime_options(
        |runtime_options: &mut RuntimeOptions| {
            runtime_options.platform.windows.net_packet_backend =
                PlatformWindowsPacketBackend::RawSocket;
        },
        |mut context| {
            let descriptors = context.destack_net_packet_backend_list()?;
            let descriptors = context.packet_backend_descriptors_from_value(descriptors)?;
            let (_name, descriptor) = descriptors
                .into_iter()
                .find(|(_name, descriptor)| descriptor.backend == expected_packet_backend())
                .expect("active packet backend should be listed");

            assert_eq!(
                descriptor.capability_flags.0,
                expected_packet_backend_capabilities()
            );

            Ok(())
        },
    );

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    with_harness_context(|mut context| {
        let descriptors = context.destack_net_packet_backend_list()?;
        let descriptors = context.packet_backend_descriptors_from_value(descriptors)?;
        let (_name, descriptor) = descriptors
            .into_iter()
            .find(|(_name, descriptor)| descriptor.backend == expected_packet_backend())
            .expect("active packet backend should be listed");

        assert_eq!(
            descriptor.capability_flags.0,
            expected_packet_backend_capabilities()
        );

        Ok(())
    });
}

/// Reflect Windows packet-backend availability from runtime configuration.
#[cfg(windows)]
#[test]
fn test_net_packet_backend_list_windows_tracks_runtime_backend_availability() {
    with_harness_context_with_runtime_options(
        |_| {},
        |mut context| {
            let descriptors = context.destack_net_packet_backend_list()?;
            let descriptors = context.packet_backend_descriptors_from_value(descriptors)?;
            let (_name, backend) = descriptors
                .into_iter()
                .find(|(_name, descriptor)| descriptor.backend == PacketBackend::WinRawSocket)
                .expect("win_raw_socket backend should be listed");

            assert!(!backend.available);

            Ok(())
        },
    );

    with_harness_context_with_runtime_options(
        |runtime_options: &mut RuntimeOptions| {
            runtime_options.platform.windows.net_packet_backend =
                PlatformWindowsPacketBackend::RawSocket;
        },
        |mut context| {
            let descriptors = context.destack_net_packet_backend_list()?;
            let descriptors = context.packet_backend_descriptors_from_value(descriptors)?;
            let (_name, backend) = descriptors
                .into_iter()
                .find(|(_name, descriptor)| descriptor.backend == PacketBackend::WinRawSocket)
                .expect("win_raw_socket backend should be listed");

            assert!(backend.available);

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
            (HarnessValue::Native(destination), HarnessValue::Native(gateway)) => context
                .harness_value(RouteEntry {
                    family: SocketFamily::IPv6,
                    destination,
                    prefix_length: 129,
                    gateway,
                    interface_index: 0,
                    metric: 0,
                    kind: RouteKind::Unicast,
                }),
            (HarnessValue::Vm(destination), HarnessValue::Vm(gateway)) => {
                context.harness_value_vm(RouteEntryVm {
                    family: SocketFamily::IPv6,
                    destination,
                    prefix_length: 129,
                    gateway,
                    interface_index: 0,
                    metric: 0,
                    kind: RouteKind::Unicast,
                })
            }
            _ => unreachable!("mixed harness values are not possible"),
        };
        assert_platform_error_code_with_privileged_policy(
            context.destack_net_route_add(route),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        // construct one invalid IPv6 route entry again for delete
        let destination = context.socket_address_value("2001:db8::", 0, SocketFamily::IPv6)?;
        let gateway = context.socket_address_value("::", 0, SocketFamily::IPv6)?;
        let route = match (destination, gateway) {
            (HarnessValue::Native(destination), HarnessValue::Native(gateway)) => context
                .harness_value(RouteEntry {
                    family: SocketFamily::IPv6,
                    destination,
                    prefix_length: 129,
                    gateway,
                    interface_index: 0,
                    metric: 0,
                    kind: RouteKind::Unicast,
                }),
            (HarnessValue::Vm(destination), HarnessValue::Vm(gateway)) => {
                context.harness_value_vm(RouteEntryVm {
                    family: SocketFamily::IPv6,
                    destination,
                    prefix_length: 129,
                    gateway,
                    interface_index: 0,
                    metric: 0,
                    kind: RouteKind::Unicast,
                })
            }
            _ => unreachable!("mixed harness values are not possible"),
        };
        assert_platform_error_code_with_privileged_policy(
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
        let socket = context.destack_net_raw_socket(SocketFamily::IPv4, RAW_PROTOCOL);
        let socket = match socket {
            Ok(socket) => socket,
            Err(error) => {
                assert_platform_error_codes_with_privileged_policy::<SocketHandle>(
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
            assert_platform_error_codes_with_privileged_policy::<()>(
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
        let invalid_socket = SocketHandle(ResourceId::local(0));

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
        assert_packet_control_error(
            context.destack_net_packet_set_timestamp_mode(
                invalid_socket,
                PacketTimestampMode::Software,
            ),
        )?;

        // packet filter lanes
        assert_packet_control_error(
            context.destack_net_packet_set_filter(invalid_socket, filter_payload),
        )?;
        assert_packet_control_error(context.destack_net_packet_clear_filter(invalid_socket))?;

        // packet fanout lanes
        assert_packet_control_error(
            context.destack_net_packet_set_fanout(invalid_socket, fanout_options),
        )?;
        assert_packet_control_error(context.destack_net_packet_clear_fanout(invalid_socket))?;

        // packet ring lanes
        assert_packet_control_error(
            context.destack_net_packet_set_rx_ring(invalid_socket, ring_options_rx),
        )?;
        assert_packet_control_error(
            context.destack_net_packet_set_tx_ring(invalid_socket, ring_options_tx),
        )?;
        assert_packet_control_error(context.destack_net_packet_clear_ring(invalid_socket))?;

        // packet data and stats lanes
        assert_packet_control_error(
            context.destack_net_packet_receive(invalid_socket, packet_payload),
        )?;
        let packet_payload = context.bytes_slice_value(b"destack-packet")?;
        assert_packet_control_error(
            context.destack_net_packet_send(invalid_socket, packet_payload),
        )?;
        assert_packet_control_error(context.destack_net_packet_stats(invalid_socket))?;

        Ok(())
    });
}
