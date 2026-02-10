use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::OsPathVm;
use crate::platform::net::{
    NetInterfaceVm, PacketCaptureOptionsVm, PacketCaptureRecordVm, PacketTimestampMode,
    RouteEntryVm, SocketFamily, SocketTimestampingMode,
};
use crate::platform::{PlatformError, VmArray, VmSlice, resource};
use crate::runtime::RuntimeCallContext;

/// Build an unsupported error for VM net bindings that are not implemented yet.
fn not_supported_binding(binding_name: &str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(binding_name)).boxed()
}

/// Read the packet mark for a socket handle.
pub(super) fn destack_net_get_packet_mark(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<u32> {
    Err(not_supported_binding("destack.net.getPacketMark"))
}

/// Read packet timestamping mode for a socket handle.
pub(super) fn destack_net_get_timestamping(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<SocketTimestampingMode> {
    Err(not_supported_binding("destack.net.getTimestamping"))
}

/// Resolve a network interface name to its index.
pub(super) fn destack_net_interface_index(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _name: vm::StringHandle,
) -> RuntimeResult<u32> {
    Err(not_supported_binding("destack.net.interfaceIndex"))
}

/// Resolve a network interface index to its name.
pub(super) fn destack_net_interface_name(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _index: u32,
) -> RuntimeResult<vm::StringHandle> {
    Err(not_supported_binding("destack.net.interfaceName"))
}

/// Enumerate network interfaces.
pub(super) fn destack_net_list_interfaces(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<VmArray<NetInterfaceVm>> {
    Err(not_supported_binding("destack.net.listInterfaces"))
}

/// Open a packet capture socket.
pub(super) fn destack_net_packet_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _options: PacketCaptureOptionsVm,
) -> RuntimeResult<resource::SocketHandle> {
    Err(not_supported_binding("destack.net.packetOpen"))
}

/// Receive one packet capture record.
pub(super) fn destack_net_packet_receive(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _handle: resource::SocketHandle,
    _payload: VmSlice<u8>,
) -> RuntimeResult<PacketCaptureRecordVm> {
    Err(not_supported_binding("destack.net.packetReceive"))
}

/// Send a packet through a packet capture socket.
pub(super) fn destack_net_packet_send(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _handle: resource::SocketHandle,
    _payload: VmSlice<u8>,
) -> RuntimeResult<u64> {
    Err(not_supported_binding("destack.net.packetSend"))
}

/// Set packet timestamp mode on a packet capture socket.
pub(super) fn destack_net_packet_set_timestamp_mode(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _handle: resource::SocketHandle,
    _mode: PacketTimestampMode,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.packetSetTimestampMode"))
}

/// Toggle IPv4 raw socket header include mode.
pub(super) fn destack_net_raw_set_header_included(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _handle: resource::SocketHandle,
    _enabled: bool,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.rawSetHeaderIncluded"))
}

/// Open a raw socket with the given family and protocol.
pub(super) fn destack_net_raw_socket(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _family: SocketFamily,
    _protocol: i32,
) -> RuntimeResult<resource::SocketHandle> {
    Err(not_supported_binding("destack.net.rawSocket"))
}

/// Add a route entry.
pub(super) fn destack_net_route_add(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _route: RouteEntryVm,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.routeAdd"))
}

/// Delete a route entry.
pub(super) fn destack_net_route_delete(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _route: RouteEntryVm,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.routeDelete"))
}

/// List route entries.
pub(super) fn destack_net_route_list(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _family: SocketFamily,
) -> RuntimeResult<VmArray<RouteEntryVm>> {
    Err(not_supported_binding("destack.net.routeList"))
}

/// Set the active route namespace.
pub(super) fn destack_net_route_set_namespace(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.routeSetNamespace"))
}

/// Set packet mark on a socket.
pub(super) fn destack_net_set_packet_mark(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _handle: resource::SocketHandle,
    _mark: u32,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.setPacketMark"))
}

/// Set packet timestamping mode on a socket.
pub(super) fn destack_net_set_timestamping(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _handle: resource::SocketHandle,
    _mode: SocketTimestampingMode,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.setTimestamping"))
}
