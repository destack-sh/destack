use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::net::{
    PacketCaptureOptions, PacketCaptureRecord, PacketTimestampMode, RouteEntry, SocketFamily,
    SocketTimestampingMode,
};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError, fs, resource};
use crate::runtime::RuntimeCallContext;

/// Return a standard not supported error for one net binding.
fn missing_binding(binding_name: &'static str) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(binding_name)).boxed())
}

/// Validate one required output pointer.
unsafe fn check_out_pointer<T>(out: *mut T, name: &'static str) -> RuntimeResult<()> {
    // reject null pointers explicitly
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer(name)).boxed());
    }

    Ok(())
}

/// Read the packet mark for one socket.
pub(crate) unsafe fn destack_net_get_packet_mark(
    _context: &RuntimeCallContext,
    out: *mut u32,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = (out, handle);

    missing_binding("destack.net.getPacketMark")
}

/// Read the timestamping mode for one socket.
pub(crate) unsafe fn destack_net_get_timestamping(
    _context: &RuntimeCallContext,
    out: *mut SocketTimestampingMode,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = (out, handle);

    missing_binding("destack.net.getTimestamping")
}

/// Resolve one network interface name to an index.
pub(crate) unsafe fn destack_net_interface_index(
    _context: &RuntimeCallContext,
    out: *mut u32,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = (out, name);

    missing_binding("destack.net.interfaceIndex")
}

/// Resolve one interface index to a name.
pub(crate) unsafe fn destack_net_interface_name(
    _context: &RuntimeCallContext,
    out: *mut NativeStringRef,
    index: u32,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = (out, index);

    missing_binding("destack.net.interfaceName")
}

/// List network interfaces.
pub(crate) unsafe fn destack_net_list_interfaces(
    _context: &RuntimeCallContext,
    out: *mut NativeArray<crate::platform::net::NetInterface>,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = out;

    missing_binding("destack.net.listInterfaces")
}

/// Open one packet socket.
pub(crate) unsafe fn destack_net_packet_open(
    _context: &RuntimeCallContext,
    out: *mut resource::SocketHandle,
    options: PacketCaptureOptions,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = (out, options);

    missing_binding("destack.net.packetOpen")
}

/// Receive one packet capture record.
pub(crate) unsafe fn destack_net_packet_receive(
    _context: &RuntimeCallContext,
    out: *mut PacketCaptureRecord,
    handle: resource::SocketHandle,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = (out, handle, payload);

    missing_binding("destack.net.packetReceive")
}

/// Send one packet through a packet socket.
pub(crate) unsafe fn destack_net_packet_send(
    _context: &RuntimeCallContext,
    out: *mut u64,
    handle: resource::SocketHandle,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = (out, handle, payload);

    missing_binding("destack.net.packetSend")
}

/// Configure packet timestamping mode.
pub(crate) unsafe fn destack_net_packet_set_timestamp_mode(
    _context: &RuntimeCallContext,
    handle: resource::SocketHandle,
    mode: PacketTimestampMode,
) -> RuntimeResult<()> {
    // keep arguments used
    let _ = (handle, mode);

    missing_binding("destack.net.packetSetTimestampMode")
}

/// Configure raw socket header included mode.
pub(crate) unsafe fn destack_net_raw_set_header_included(
    _context: &RuntimeCallContext,
    handle: resource::SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // keep arguments used
    let _ = (handle, enabled);

    missing_binding("destack.net.rawSetHeaderIncluded")
}

/// Open one raw socket.
pub(crate) unsafe fn destack_net_raw_socket(
    _context: &RuntimeCallContext,
    out: *mut resource::SocketHandle,
    family: SocketFamily,
    protocol: i32,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = (out, family, protocol);

    missing_binding("destack.net.rawSocket")
}

/// Add one route entry.
pub(crate) unsafe fn destack_net_route_add(
    _context: &RuntimeCallContext,
    route: RouteEntry,
) -> RuntimeResult<()> {
    // keep arguments used
    let _ = route;

    missing_binding("destack.net.routeAdd")
}

/// Delete one route entry.
pub(crate) unsafe fn destack_net_route_delete(
    _context: &RuntimeCallContext,
    route: RouteEntry,
) -> RuntimeResult<()> {
    // keep arguments used
    let _ = route;

    missing_binding("destack.net.routeDelete")
}

/// List route entries for one family.
pub(crate) unsafe fn destack_net_route_list(
    _context: &RuntimeCallContext,
    out: *mut NativeArray<RouteEntry>,
    family: SocketFamily,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = (out, family);

    missing_binding("destack.net.routeList")
}

/// Set the route namespace context.
pub(crate) unsafe fn destack_net_route_set_namespace(
    _context: &RuntimeCallContext,
    path: fs::OsPath,
) -> RuntimeResult<()> {
    // keep arguments used
    let _ = path;

    missing_binding("destack.net.routeSetNamespace")
}

/// Set the packet mark for one socket.
pub(crate) unsafe fn destack_net_set_packet_mark(
    _context: &RuntimeCallContext,
    handle: resource::SocketHandle,
    mark: u32,
) -> RuntimeResult<()> {
    // keep arguments used
    let _ = (handle, mark);

    missing_binding("destack.net.setPacketMark")
}

/// Set the timestamping mode for one socket.
pub(crate) unsafe fn destack_net_set_timestamping(
    _context: &RuntimeCallContext,
    handle: resource::SocketHandle,
    mode: SocketTimestampingMode,
) -> RuntimeResult<()> {
    // keep arguments used
    let _ = (handle, mode);

    missing_binding("destack.net.setTimestamping")
}
