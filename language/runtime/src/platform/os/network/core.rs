use std::sync::Arc;

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeArray;
use crate::platform::core::{io_would_block, monotonic_now_ns};
use crate::platform::net::{
    NetInterface, NetInterfaceValue, RouteEntry, RouteEntryValue, RouteKind, SocketFamily,
    native as native_net,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{NativeAbiCodec, resource};
use crate::runtime::BindingCallContext;
use crate::runtime::process::RuntimeScheduledCallbackControl;

use crate::platform::os::state::{NetworkWatchStream, PlatformOsState, invalid_handle, os_state};
use crate::platform::os::{
    NetworkConnectionType, NetworkEvent, NetworkEventVm, NetworkState, NetworkStateVm,
};

/// Shared callback interval for network watch state refresh.
const NETWORK_WATCH_CALLBACK_INTERVAL_NS: u64 = 100_000_000;

/// Read one point-in-time host network state snapshot.
pub(crate) fn state(binding: &BindingCallContext) -> RuntimeResult<NetworkState> {
    super::target::read_network_state(binding)
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
    let runtime_state = os_state(binding)?;
    let watch_state = Arc::new(NetworkWatchStream::new(initial_state));
    let watch_id = runtime_state.insert_network_watch(watch_state);
    ensure_network_watch_callback(binding)?;
    let entry = ResourceEntry::new(ResourceKind::NetworkWatch)
        .with_label("os.network.watch")
        .with_payload(watch_id);
    let handle = binding
        .worker()
        .resources
        .insert(&binding.world(), entry, Some(binding.engine()));

    Ok(resource::NetworkWatchHandle(handle))
}

/// Close one host network watch stream.
pub(crate) fn watch_close(
    binding: &BindingCallContext,
    handle: resource::NetworkWatchHandle,
) -> RuntimeResult<()> {
    let runtime_state = os_state(binding)?;
    let removed =
        binding
            .worker()
            .resources
            .remove(&binding.world(), handle.0, Some(binding.engine()));
    let Some(entry) = removed else {
        return Err(invalid_handle("unknown network watch handle"));
    };
    let Some(payload) = entry.payload else {
        return Err(invalid_handle("unknown network watch handle"));
    };
    let Ok(watch_id) = payload.downcast::<u64>() else {
        return Err(invalid_handle("unknown network watch handle"));
    };
    let Some(watch_state) = runtime_state.remove_network_watch(*watch_id) else {
        return Err(invalid_handle("unknown network watch handle"));
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
        || {
            if watch_state.is_closed() {
                return Err(invalid_handle("unknown network watch handle"));
            }

            Ok(watch_state.try_take())
        },
        |duration| watch_state.wait_once(duration),
    )
}

/// Poll one host network event without blocking.
pub(crate) fn watch_try_read(
    binding: &BindingCallContext,
    handle: resource::NetworkWatchHandle,
) -> RuntimeResult<NetworkEvent> {
    let watch_state = resolve_watch_state(binding, handle)?;
    binding.advance_wait_progress()?;

    let Some(event) = watch_state.try_take() else {
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
) -> RuntimeResult<Arc<NetworkWatchStream>> {
    let resolved = binding.worker().resources.with_entry(handle.0, |entry| {
        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<u64>())
            .copied()
    });
    let Some(watch_id) = resolved.flatten() else {
        return Err(invalid_handle("unknown network watch handle"));
    };
    let runtime_state = os_state(binding)?;

    runtime_state
        .network_watch(watch_id)
        .ok_or_else(|| invalid_handle("unknown network watch handle"))
}

/// Ensure one shared network watch callback is registered for this worker.
fn ensure_network_watch_callback(binding: &BindingCallContext) -> RuntimeResult<()> {
    let runtime_state = os_state(binding)?;
    if runtime_state.network_watch_callback().is_some() {
        return Ok(());
    }

    let callback_runtime_state = runtime_state.clone();
    let callback_handle = binding.worker().schedule_runtime_callback(
        binding,
        NETWORK_WATCH_CALLBACK_INTERVAL_NS,
        Some(NETWORK_WATCH_CALLBACK_INTERVAL_NS),
        move |binding| service_network_watch_callback(binding, &callback_runtime_state),
    )?;
    runtime_state.set_network_watch_callback(callback_handle);

    Ok(())
}

/// Service one shared network watch callback tick.
fn service_network_watch_callback(
    binding: &BindingCallContext,
    runtime_state: &PlatformOsState,
) -> RuntimeResult<RuntimeScheduledCallbackControl> {
    if !runtime_state.has_network_watches() {
        if let Some(handle) = runtime_state.network_watch_callback() {
            runtime_state.clear_network_watch_callback(handle);
        }

        return Ok(RuntimeScheduledCallbackControl::Cancel);
    }

    let streams = runtime_state.network_watch_streams();
    let next_state = state(binding)?;

    for stream in streams {
        stream.publish_state(next_state);
    }

    Ok(RuntimeScheduledCallbackControl::Keep)
}

/// Expose one VM-facing host network snapshot.
pub(crate) fn state_vm(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<NetworkStateVm> {
    let _ = _context;
    state(binding)
}

/// Expose one VM-facing host network watch open.
pub(crate) fn watch_open_vm(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::NetworkWatchHandle> {
    let _ = _context;
    watch_open(binding)
}

/// Expose one VM-facing host network watch close.
pub(crate) fn watch_close_vm(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::NetworkWatchHandle,
) -> RuntimeResult<()> {
    let _ = _context;
    watch_close(binding, handle)
}

/// Expose one VM-facing host network watch read.
pub(crate) fn watch_read_vm(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::NetworkWatchHandle,
    timeout_ns: u64,
) -> RuntimeResult<NetworkEventVm> {
    let _ = _context;
    watch_read(binding, handle, timeout_ns)
}

/// Expose one VM-facing host network watch try-read.
pub(crate) fn watch_try_read_vm(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::NetworkWatchHandle,
) -> RuntimeResult<NetworkEventVm> {
    let _ = _context;
    watch_try_read(binding, handle)
}
