use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;

use destack_vm as vm;
use parking_lot::Mutex;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeArray;
use crate::platform::core::{invalid_argument, io_would_block, monotonic_now_ns};
use crate::platform::net::{
    NetInterface, NetInterfaceValue, RouteEntry, RouteEntryValue, RouteKind, SocketFamily,
    native as native_net,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{NativeAbiCodec, resource};
use crate::runtime::BindingCallContext;

use crate::platform::os::network::backend;
use crate::platform::os::{
    NetworkConnectionType, NetworkEvent, NetworkEventVm, NetworkState, NetworkStateVm,
};

/// Maximum wait slice used while one network watch polls host state.
const NETWORK_WATCH_SLICE_NS: u64 = 100_000_000;

/// Runtime-owned network watch state.
#[derive(Debug)]
struct NetworkWatchState {
    /// Last emitted network snapshot.
    last_state: Mutex<NetworkState>,
    /// Whether this watch has been closed.
    is_closed: AtomicBool,
    /// Sequence number for the next emitted event.
    next_sequence: AtomicU64,
}

impl NetworkWatchState {
    /// Create one network watch seeded from the current snapshot.
    fn new(initial_state: NetworkState) -> Self {
        Self {
            last_state: Mutex::new(initial_state),
            is_closed: AtomicBool::new(false),
            next_sequence: AtomicU64::new(0),
        }
    }

    /// Return whether this watch has been closed.
    fn is_closed(&self) -> bool {
        self.is_closed.load(Ordering::Relaxed)
    }

    /// Close this watch.
    fn close(&self) {
        self.is_closed.store(true, Ordering::Relaxed);
    }
}

/// Read one point-in-time host network state snapshot.
pub(crate) fn state(binding: &BindingCallContext) -> RuntimeResult<NetworkState> {
    backend::read_network_state(binding)
}

/// Read one point-in-time host network state snapshot from the audited net substrate.
pub(crate) fn read_network_state_from_host(
    binding: &BindingCallContext,
) -> RuntimeResult<NetworkState> {
    let interfaces = list_interfaces(binding)?;
    let routes_v4 = list_routes(binding, SocketFamily::IPv4)?;
    let routes_v6 = list_routes(binding, SocketFamily::IPv6)?;
    let internet_reachable = has_default_route(&routes_v4) || has_default_route(&routes_v6);
    let active_interface =
        find_preferred_interface(&interfaces, &routes_v4, &routes_v6).or_else(|| {
            interfaces
                .iter()
                .find(|interface| interface_connected(interface))
        });

    // report no route when no active interface is available
    if active_interface.is_none() {
        return Ok(NetworkState {
            connection_type: NetworkConnectionType::None,
            connected: false,
            internet_reachable: false,
            expensive: None,
            constrained: None,
            roaming: None,
            cellular_generation: None,
            downlink_mbps: None,
            uplink_mbps: None,
        });
    }

    let connection_type = active_interface
        .map(|interface| classify_connection_type(interface.name.as_str()))
        .unwrap_or(NetworkConnectionType::Unknown);

    Ok(NetworkState {
        connection_type,
        connected: true,
        internet_reachable,
        expensive: None,
        constrained: None,
        roaming: None,
        cellular_generation: None,
        downlink_mbps: None,
        uplink_mbps: None,
    })
}

/// Open one host network watch stream.
pub(crate) fn watch_open(
    binding: &BindingCallContext,
) -> RuntimeResult<resource::NetworkWatchHandle> {
    let initial_state = state(binding)?;
    let watch_state = Arc::new(NetworkWatchState::new(initial_state));
    let entry = ResourceEntry::new(ResourceKind::NetworkWatch)
        .with_label("os.network.watch")
        .with_payload(watch_state);
    let handle = binding
        .agent()
        .resources
        .insert(binding.world(), entry, Some(binding.engine()));

    Ok(resource::NetworkWatchHandle(handle))
}

/// Close one host network watch stream.
pub(crate) fn watch_close(
    binding: &BindingCallContext,
    handle: resource::NetworkWatchHandle,
) -> RuntimeResult<()> {
    let removed =
        binding
            .agent()
            .resources
            .remove(binding.world(), handle.0, Some(binding.engine()));
    let Some(entry) = removed else {
        return Err(invalid_network_watch_handle());
    };
    let Some(payload) = entry.payload else {
        return Err(invalid_network_watch_handle());
    };
    let Ok(watch_state) = payload.downcast::<Arc<NetworkWatchState>>() else {
        return Err(invalid_network_watch_handle());
    };

    watch_state.close();

    Ok(())
}

/// Wait for one host network event.
pub(crate) fn watch_read(
    binding: &BindingCallContext,
    handle: resource::NetworkWatchHandle,
    timeout_ns: u64,
) -> RuntimeResult<NetworkEvent> {
    let watch_state = resolve_watch_state(binding, handle)?;
    let now = monotonic_now_ns();
    let deadline_ns = now.saturating_add(timeout_ns);

    binding.wait_for_binding_result(
        "destack.os.network.watchRead",
        "timed out waiting for network event",
        deadline_ns,
        NETWORK_WATCH_SLICE_NS,
        || try_next_event(binding, &watch_state),
        thread::sleep,
    )
}

/// Poll one host network event without blocking.
pub(crate) fn watch_try_read(
    binding: &BindingCallContext,
    handle: resource::NetworkWatchHandle,
) -> RuntimeResult<NetworkEvent> {
    let watch_state = resolve_watch_state(binding, handle)?;
    let Some(event) = try_next_event(binding, &watch_state)? else {
        return Err(io_would_block(
            "destack.os.network.watchTryRead",
            "no network event is currently queued",
        ));
    };

    Ok(event)
}

/// Return one owned interface list snapshot.
fn list_interfaces(binding: &BindingCallContext) -> RuntimeResult<Vec<NetInterfaceValue>> {
    let mut out = NativeArray {
        data: std::ptr::null_mut(),
        len: 0,
        capacity: 0,
    };

    // delegate to the audited net substrate
    unsafe {
        native_net::destack_net_list_interfaces(binding, &mut out)?;
    }

    let interfaces = unsafe { out.as_slice()? };
    let mut values = Vec::with_capacity(interfaces.len());

    for interface in interfaces {
        values.push(unsafe { <NetInterface as NativeAbiCodec>::into_value(*interface)? });
    }

    Ok(values)
}

/// Return one owned route-list snapshot for one address family.
fn list_routes(
    binding: &BindingCallContext,
    family: SocketFamily,
) -> RuntimeResult<Vec<RouteEntryValue>> {
    let mut out = NativeArray {
        data: std::ptr::null_mut(),
        len: 0,
        capacity: 0,
    };

    // delegate to the audited net substrate
    unsafe {
        native_net::destack_net_route_list(binding, &mut out, family)?;
    }

    let routes = unsafe { out.as_slice()? };
    let mut values = Vec::with_capacity(routes.len());

    for route in routes {
        values.push(unsafe { <RouteEntry as NativeAbiCodec>::into_value(*route)? });
    }

    Ok(values)
}

/// Return whether one route list contains one default internet route.
fn has_default_route(routes: &[RouteEntryValue]) -> bool {
    routes.iter().any(|route| {
        route.kind == RouteKind::Unicast && route.prefix_length == 0 && route.interface_index != 0
    })
}

/// Return the interface referenced by the preferred default route when one exists.
fn find_preferred_interface<'a>(
    interfaces: &'a [NetInterfaceValue],
    routes_v4: &[RouteEntryValue],
    routes_v6: &[RouteEntryValue],
) -> Option<&'a NetInterfaceValue> {
    let default_route = routes_v4.iter().chain(routes_v6.iter()).find(|route| {
        route.kind == RouteKind::Unicast && route.prefix_length == 0 && route.interface_index != 0
    })?;

    interfaces
        .iter()
        .find(|interface| interface.index == default_route.interface_index)
}

/// Return whether one interface has enough evidence to count as connected.
fn interface_connected(interface: &NetInterfaceValue) -> bool {
    if interface.addresses.is_empty() {
        return false;
    }

    !interface_loopback(interface.name.as_str())
}

/// Return whether one interface name clearly identifies one loopback adapter.
fn interface_loopback(name: &str) -> bool {
    let name = name.trim().to_ascii_lowercase();

    name == "lo" || name == "lo0" || name.contains("loopback") || name.contains("pseudo-interface")
}

/// Classify one host interface into one public connection class.
fn classify_connection_type(name: &str) -> NetworkConnectionType {
    let name = name.trim().to_ascii_lowercase();

    // explicit vpn and tunnel names
    if name.contains("vpn")
        || name.starts_with("utun")
        || name.starts_with("tun")
        || name.starts_with("tap")
        || name.starts_with("ppp")
    {
        return NetworkConnectionType::Vpn;
    }

    // explicit wifi names
    if name.contains("wi-fi")
        || name.contains("wifi")
        || name.starts_with("wlan")
        || name.starts_with("wl")
        || name.starts_with("ath")
    {
        return NetworkConnectionType::Wifi;
    }

    // explicit cellular names
    if name.contains("cellular")
        || name.starts_with("wwan")
        || name.starts_with("rmnet")
        || name.starts_with("pdp_ip")
        || name.starts_with("ccmni")
    {
        return NetworkConnectionType::Cellular;
    }

    // explicit bluetooth pan names
    if name.contains("bluetooth") || name.starts_with("bnep") || name.starts_with("bt-pan") {
        return NetworkConnectionType::Bluetooth;
    }

    // explicit ethernet names
    if name.contains("ethernet") || name.starts_with("eth") || name.starts_with("enx") {
        return NetworkConnectionType::Ethernet;
    }

    NetworkConnectionType::Unknown
}

/// Resolve one watch handle into its payload.
fn resolve_watch_state(
    binding: &BindingCallContext,
    handle: resource::NetworkWatchHandle,
) -> RuntimeResult<Arc<NetworkWatchState>> {
    let resolved = binding.agent().resources.with_entry(handle.0, |entry| {
        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<Arc<NetworkWatchState>>())
            .map(Arc::clone)
    });

    resolved.flatten().ok_or_else(invalid_network_watch_handle)
}

/// Try to produce the next network event when the host snapshot changed.
fn try_next_event(
    binding: &BindingCallContext,
    watch_state: &Arc<NetworkWatchState>,
) -> RuntimeResult<Option<NetworkEvent>> {
    if watch_state.is_closed() {
        return Err(invalid_network_watch_handle());
    }

    let next_state = state(binding)?;
    let mut last_state = watch_state.last_state.lock();
    if *last_state == next_state {
        return Ok(None);
    }

    let timestamp_ns = monotonic_now_ns();
    let sequence = watch_state.next_sequence.fetch_add(1, Ordering::Relaxed);
    *last_state = next_state;

    Ok(Some(NetworkEvent {
        timestamp_ns,
        sequence,
        state: next_state,
    }))
}

/// Build one invalid network-watch handle error.
fn invalid_network_watch_handle() -> Box<RuntimeError> {
    invalid_argument("handle", "unknown network watch handle")
}

/// Expose one VM-facing host network snapshot.
pub(crate) fn state_vm(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<NetworkStateVm> {
    let _ = _context;
    state(binding)
}

/// Expose one VM-facing host network watch open.
pub(crate) fn watch_open_vm(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<resource::NetworkWatchHandle> {
    let _ = _context;
    watch_open(binding)
}

/// Expose one VM-facing host network watch close.
pub(crate) fn watch_close_vm(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::NetworkWatchHandle,
) -> RuntimeResult<()> {
    let _ = _context;
    watch_close(binding, handle)
}

/// Expose one VM-facing host network watch read.
pub(crate) fn watch_read_vm(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::NetworkWatchHandle,
    timeout_ns: u64,
) -> RuntimeResult<NetworkEventVm> {
    let _ = _context;
    watch_read(binding, handle, timeout_ns)
}

/// Expose one VM-facing host network watch try-read.
pub(crate) fn watch_try_read_vm(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::NetworkWatchHandle,
) -> RuntimeResult<NetworkEventVm> {
    let _ = _context;
    watch_try_read(binding, handle)
}
