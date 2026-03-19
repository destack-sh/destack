use super::core::*;
use super::metadata::*;

/// Validate one bluetooth scan filter for the BlueZ backend.
pub(super) fn validate_scan_filter(filter: &BluetoothScanFilterValue) -> RuntimeResult<()> {
    if filter.scan_mode.is_some() {
        return Err(core_platform::invalid_argument(
            "filter.scanMode",
            "bluez does not expose passive or active scan selection",
        ));
    }
    if filter.primary_phy.is_some() {
        return Err(core_platform::invalid_argument(
            "filter.primaryPhy",
            "bluez does not expose scan primary PHY selection",
        ));
    }
    if filter.secondary_phy.is_some() {
        return Err(core_platform::invalid_argument(
            "filter.secondaryPhy",
            "bluez does not expose scan secondary PHY selection",
        ));
    }

    Ok(())
}

/// Start one adapter discovery session.
pub(super) fn start_adapter_discovery(
    connection: &Connection,
    adapter_path: &str,
) -> RuntimeResult<()> {
    let proxy = bluez_proxy(
        connection,
        adapter_path,
        BLUEZ_ADAPTER_INTERFACE,
        "destack.device.bluetooth.scan.open",
    )?;

    proxy
        .call::<_, _, ()>("StartDiscovery", &())
        .map_err(|error| {
            bluez_error(
                "destack.device.bluetooth.scan.open",
                "Adapter1.StartDiscovery",
                error,
            )
        })
}

/// Stop one adapter discovery session.
pub(super) fn stop_adapter_discovery(
    connection: &Connection,
    adapter_path: &str,
) -> RuntimeResult<()> {
    let proxy = bluez_proxy(
        connection,
        adapter_path,
        BLUEZ_ADAPTER_INTERFACE,
        "destack.device.bluetooth.scan.close",
    )?;

    proxy
        .call::<_, _, ()>("StopDiscovery", &())
        .map_err(|error| {
            bluez_error(
                "destack.device.bluetooth.scan.close",
                "Adapter1.StopDiscovery",
                error,
            )
        })
}

/// Connect one device session.
pub(super) fn connect_device(connection: &Connection, device_path: &str) -> RuntimeResult<()> {
    let proxy = bluez_proxy(
        connection,
        device_path,
        BLUEZ_DEVICE_INTERFACE,
        "destack.device.bluetooth.session.open",
    )?;

    proxy.call::<_, _, ()>("Connect", &()).map_err(|error| {
        bluez_error(
            "destack.device.bluetooth.session.open",
            "Device1.Connect",
            error,
        )
    })
}

/// Disconnect one device session.
pub(super) fn disconnect_device(connection: &Connection, device_path: &str) -> RuntimeResult<()> {
    let proxy = bluez_proxy(
        connection,
        device_path,
        BLUEZ_DEVICE_INTERFACE,
        "destack.device.bluetooth.session.close",
    )?;

    proxy.call::<_, _, ()>("Disconnect", &()).map_err(|error| {
        bluez_error(
            "destack.device.bluetooth.session.close",
            "Device1.Disconnect",
            error,
        )
    })
}

/// Pair one device session.
pub(super) fn pair_device(connection: &Connection, device_path: &str) -> RuntimeResult<()> {
    let proxy = bluez_proxy(
        connection,
        device_path,
        BLUEZ_DEVICE_INTERFACE,
        "destack.device.bluetooth.session.pair",
    )?;

    proxy.call::<_, _, ()>("Pair", &()).map_err(|error| {
        bluez_error(
            "destack.device.bluetooth.session.pair",
            "Device1.Pair",
            error,
        )
    })
}

/// Remove one bonded device.
pub(super) fn remove_device(
    connection: &Connection,
    adapter_path: &str,
    device_path: &str,
) -> RuntimeResult<()> {
    let proxy = bluez_proxy(
        connection,
        adapter_path,
        BLUEZ_ADAPTER_INTERFACE,
        "destack.device.bluetooth.session.unpair",
    )?;
    let device_path = OwnedObjectPath::try_from(device_path.to_string()).map_err(|error| {
        bluez_error(
            "destack.device.bluetooth.session.unpair",
            "OwnedObjectPath::try_from",
            error,
        )
    })?;

    proxy
        .call::<_, _, ()>("RemoveDevice", &(device_path,))
        .map_err(|error| {
            bluez_error(
                "destack.device.bluetooth.session.unpair",
                "Adapter1.RemoveDevice",
                error,
            )
        })
}

/// Start one characteristic notification subscription.
pub(super) fn start_notify(
    connection: &Connection,
    characteristic_path: &str,
) -> RuntimeResult<()> {
    let proxy = bluez_proxy(
        connection,
        characteristic_path,
        BLUEZ_GATT_CHARACTERISTIC_INTERFACE,
        "destack.device.bluetooth.gatt.subscribe",
    )?;

    proxy.call::<_, _, ()>("StartNotify", &()).map_err(|error| {
        bluez_error(
            "destack.device.bluetooth.gatt.subscribe",
            "GattCharacteristic1.StartNotify",
            error,
        )
    })
}

/// Stop one characteristic notification subscription.
pub(super) fn stop_notify(connection: &Connection, characteristic_path: &str) -> RuntimeResult<()> {
    let proxy = bluez_proxy(
        connection,
        characteristic_path,
        BLUEZ_GATT_CHARACTERISTIC_INTERFACE,
        "destack.device.bluetooth.gatt.unsubscribe",
    )?;

    proxy.call::<_, _, ()>("StopNotify", &()).map_err(|error| {
        bluez_error(
            "destack.device.bluetooth.gatt.unsubscribe",
            "GattCharacteristic1.StopNotify",
            error,
        )
    })
}

/// Parse one stable adapter identifier into one object path.
pub(super) fn parse_adapter_path(
    connection: &Connection,
    id: &str,
    operation: &'static str,
) -> RuntimeResult<OwnedObjectPath> {
    let path = OwnedObjectPath::try_from(id.to_string()).map_err(|_| {
        core_platform::invalid_argument("adapterId", "adapter identifier must be one object path")
    })?;
    let objects = bluez_managed_objects(connection, operation)?;
    let interfaces = objects
        .get(&path)
        .ok_or_else(|| core_platform::io_not_found(operation, "unknown bluetooth adapter"))?;
    if interface_properties(interfaces, BLUEZ_ADAPTER_INTERFACE).is_none() {
        return Err(core_platform::io_not_found(
            operation,
            "unknown bluetooth adapter",
        ));
    }

    Ok(path)
}

/// Parse one stable device identifier into one object path.
pub(super) fn parse_device_path(
    connection: &Connection,
    adapter_path: &str,
    id: &str,
    operation: &'static str,
) -> RuntimeResult<OwnedObjectPath> {
    let path = OwnedObjectPath::try_from(id.to_string()).map_err(|_| {
        core_platform::invalid_argument("deviceId", "device identifier must be one object path")
    })?;
    let objects = bluez_managed_objects(connection, operation)?;
    let interfaces = objects
        .get(&path)
        .ok_or_else(|| core_platform::io_not_found(operation, "unknown bluetooth device"))?;
    let properties = interface_properties(interfaces, BLUEZ_DEVICE_INTERFACE)
        .ok_or_else(|| core_platform::io_not_found(operation, "unknown bluetooth device"))?;
    let object_adapter = property::<OwnedObjectPath>(properties, "Adapter", operation)?;
    if object_adapter.as_str() != adapter_path {
        return Err(core_platform::invalid_argument(
            "deviceId",
            "device does not belong to the selected adapter",
        ));
    }

    Ok(path)
}

/// Load the current matching device snapshot for one adapter.
pub(super) fn adapter_device_snapshot(
    connection: &Connection,
    adapter_path: &str,
    filter: Option<&BluetoothScanFilterValue>,
    operation: &'static str,
) -> RuntimeResult<BTreeMap<String, BluetoothDeviceDescriptorValue>> {
    let objects = bluez_managed_objects(connection, operation)?;
    let mut snapshot = BTreeMap::new();

    for (path, interfaces) in &objects {
        if !is_child_path(path.as_str(), adapter_path) {
            continue;
        }

        let Some(properties) = interface_properties(interfaces, BLUEZ_DEVICE_INTERFACE) else {
            continue;
        };

        let descriptor = device_descriptor(path, properties, operation)?;
        if filter.is_some_and(|filter| !matches_scan_filter(&descriptor, filter)) {
            continue;
        }

        snapshot.insert(descriptor.id.clone(), descriptor);
    }

    Ok(snapshot)
}

/// Find one current device descriptor by id.
pub(super) fn current_device_descriptor(
    connection: &Connection,
    device_path: &str,
    operation: &'static str,
) -> RuntimeResult<Option<BluetoothDeviceDescriptorValue>> {
    let objects = bluez_managed_objects(connection, operation)?;
    let path = OwnedObjectPath::try_from(device_path.to_string()).map_err(|_| {
        core_platform::invalid_argument("deviceId", "device identifier must be one object path")
    })?;
    let Some(interfaces) = objects.get(&path) else {
        return Ok(None);
    };
    let Some(properties) = interface_properties(interfaces, BLUEZ_DEVICE_INTERFACE) else {
        return Ok(None);
    };

    Ok(Some(device_descriptor(&path, properties, operation)?))
}

/// Decode one object-manager interfaces-added signal body.
pub(super) fn interfaces_added_body(
    message: &zbus::Message,
    operation: &'static str,
) -> RuntimeResult<(
    OwnedObjectPath,
    HashMap<OwnedInterfaceName, HashMap<String, OwnedValue>>,
)> {
    message
        .body()
        .deserialize()
        .map_err(|error| bluez_error(operation, "Message::body().deserialize", error))
}

/// Decode one object-manager interfaces-removed signal body.
pub(super) fn interfaces_removed_body(
    message: &zbus::Message,
    operation: &'static str,
) -> RuntimeResult<(OwnedObjectPath, Vec<OwnedInterfaceName>)> {
    message
        .body()
        .deserialize()
        .map_err(|error| bluez_error(operation, "Message::body().deserialize", error))
}

/// Decode one properties-changed signal body.
pub(super) fn properties_changed_body(
    message: &zbus::Message,
    operation: &'static str,
) -> RuntimeResult<(String, HashMap<String, OwnedValue>, Vec<String>)> {
    message
        .body()
        .deserialize()
        .map_err(|error| bluez_error(operation, "Message::body().deserialize", error))
}

/// Return the message member string when present.
pub(super) fn message_member(message: &zbus::Message) -> Option<String> {
    message
        .header()
        .member()
        .map(|member| member.as_str().to_string())
}

/// Return the message interface string when present.
pub(super) fn message_interface(message: &zbus::Message) -> Option<String> {
    message
        .header()
        .interface()
        .map(|interface_name| interface_name.as_str().to_string())
}

/// Return the message path string when present.
pub(super) fn message_path(message: &zbus::Message) -> Option<String> {
    message
        .header()
        .path()
        .map(|path| path.as_str().to_string())
}

/// Dispatch one BlueZ signal to one adapter-watch registration.
pub(super) fn handle_adapter_message(
    resource: &LinuxBluetoothAdapterWatchResource,
    connection: &Connection,
    message: &zbus::Message,
    known_adapters: &mut BTreeMap<String, BluetoothAdapterDescriptorValue>,
) -> bool {
    // object-manager topology
    if message_interface(message).as_deref() == Some("org.freedesktop.DBus.ObjectManager") {
        match message_member(message).as_deref() {
            // inserted interfaces
            Some("InterfacesAdded") => {
                let Ok((path, interfaces)) =
                    interfaces_added_body(message, "destack.device.bluetooth.adapter.watch")
                else {
                    return true;
                };
                if interface_properties(&interfaces, BLUEZ_ADAPTER_INTERFACE).is_none() {
                    return true;
                }
                let Ok(descriptor) = adapter_descriptor(
                    &path,
                    &interfaces,
                    "destack.device.bluetooth.adapter.watch",
                ) else {
                    return true;
                };
                let is_new = !known_adapters.contains_key(&descriptor.id);
                let is_changed = known_adapters.get(&descriptor.id) != Some(&descriptor);
                known_adapters.insert(descriptor.id.clone(), descriptor.clone());

                // first topology publication
                if is_new {
                    resource
                        .event_queue
                        .push_drop_oldest(adapter_attached_event(
                            &resource.event_state,
                            descriptor,
                        ));
                }
                // repeated update
                else if is_changed {
                    resource
                        .event_queue
                        .push_drop_oldest(adapter_changed_event(&resource.event_state, descriptor));
                }
            }

            // removed interfaces
            Some("InterfacesRemoved") => {
                let Ok((path, interfaces)) =
                    interfaces_removed_body(message, "destack.device.bluetooth.adapter.watch")
                else {
                    return true;
                };
                if !interfaces
                    .iter()
                    .any(|interface_name| interface_name.as_str() == BLUEZ_ADAPTER_INTERFACE)
                {
                    return true;
                }
                let Some(descriptor) = known_adapters.remove(path.as_str()) else {
                    return true;
                };

                resource
                    .event_queue
                    .push_drop_oldest(adapter_detached_event(&resource.event_state, descriptor));
            }
            _ => {}
        }

        return true;
    }

    // properties-changed updates
    if message_interface(message).as_deref() != Some("org.freedesktop.DBus.Properties") {
        return true;
    }
    let Some(path) = message_path(message) else {
        return true;
    };
    let Ok((interface_name, _changed_properties, _invalidated_properties)) =
        properties_changed_body(message, "destack.device.bluetooth.adapter.watch")
    else {
        return true;
    };
    if interface_name != BLUEZ_ADAPTER_INTERFACE {
        return true;
    }

    let current_descriptor = match current_adapter_descriptor(
        connection,
        path.as_str(),
        "destack.device.bluetooth.adapter.watch",
    ) {
        Ok(descriptor) => descriptor,
        Err(_) => return true,
    };
    let Some(descriptor) = current_descriptor else {
        let Some(previous) = known_adapters.remove(path.as_str()) else {
            return true;
        };

        resource
            .event_queue
            .push_drop_oldest(adapter_detached_event(&resource.event_state, previous));
        return true;
    };

    let is_new = !known_adapters.contains_key(&descriptor.id);
    let is_changed = known_adapters.get(&descriptor.id) != Some(&descriptor);
    known_adapters.insert(descriptor.id.clone(), descriptor.clone());

    if is_new {
        resource
            .event_queue
            .push_drop_oldest(adapter_attached_event(&resource.event_state, descriptor));
    } else if is_changed {
        resource
            .event_queue
            .push_drop_oldest(adapter_changed_event(&resource.event_state, descriptor));
    }

    true
}

/// Dispatch one BlueZ signal to one scan registration.
pub(super) fn handle_scan_message(
    resource: &LinuxBluetoothScanResource,
    connection: &Connection,
    message: &zbus::Message,
    known_devices: &mut BTreeMap<String, BluetoothDeviceDescriptorValue>,
) -> bool {
    // object-manager topology
    if message_interface(message).as_deref() == Some("org.freedesktop.DBus.ObjectManager") {
        match message_member(message).as_deref() {
            // inserted interfaces
            Some("InterfacesAdded") => {
                let Ok((path, interfaces)) =
                    interfaces_added_body(message, "destack.device.bluetooth.scan.watch")
                else {
                    return true;
                };
                if !is_child_path(path.as_str(), &resource.adapter_path) {
                    return true;
                }
                let Some(properties) = interface_properties(&interfaces, BLUEZ_DEVICE_INTERFACE)
                else {
                    return true;
                };
                let Ok(descriptor) =
                    device_descriptor(&path, properties, "destack.device.bluetooth.scan.watch")
                else {
                    return true;
                };

                if resource
                    .filter
                    .as_ref()
                    .is_some_and(|filter| !matches_scan_filter(&descriptor, filter))
                {
                    return true;
                }

                let is_new = !known_devices.contains_key(&descriptor.id);
                let is_changed = known_devices.get(&descriptor.id) != Some(&descriptor);
                known_devices.insert(descriptor.id.clone(), descriptor.clone());

                if is_new {
                    resource.device_queue.push_drop_oldest(descriptor.clone());
                    resource
                        .event_queue
                        .push_drop_oldest(scan_discovered_event(&resource.event_state, descriptor));
                }
                // repeated update
                else if is_changed
                    || resource
                        .filter
                        .as_ref()
                        .is_some_and(|filter| filter.keep_repeated_devices)
                {
                    resource.device_queue.push_drop_oldest(descriptor.clone());
                    resource
                        .event_queue
                        .push_drop_oldest(scan_updated_event(&resource.event_state, descriptor));
                }
            }
            // removed interfaces
            Some("InterfacesRemoved") => {
                let Ok((path, interfaces)) =
                    interfaces_removed_body(message, "destack.device.bluetooth.scan.watch")
                else {
                    return true;
                };
                if !is_child_path(path.as_str(), &resource.adapter_path) {
                    return true;
                }
                if !interfaces
                    .iter()
                    .any(|interface_name| interface_name.as_str() == BLUEZ_DEVICE_INTERFACE)
                {
                    return true;
                }
                let Some(descriptor) = known_devices.remove(path.as_str()) else {
                    return true;
                };

                resource
                    .event_queue
                    .push_drop_oldest(scan_lost_event(&resource.event_state, descriptor));
            }
            _ => {}
        }

        return true;
    }

    // properties-changed updates
    if message_interface(message).as_deref() != Some("org.freedesktop.DBus.Properties") {
        return true;
    }
    let Some(path) = message_path(message) else {
        return true;
    };
    let Ok((interface_name, _changed_properties, _invalidated_properties)) =
        properties_changed_body(message, "destack.device.bluetooth.scan.watch")
    else {
        return true;
    };

    if path == resource.adapter_path {
        return true;
    }
    if interface_name != BLUEZ_DEVICE_INTERFACE
        || !is_child_path(path.as_str(), &resource.adapter_path)
    {
        return true;
    }

    let current_descriptor = match current_device_descriptor(
        connection,
        path.as_str(),
        "destack.device.bluetooth.scan.watch",
    ) {
        Ok(descriptor) => descriptor,
        Err(_) => return true,
    };
    let Some(descriptor) = current_descriptor else {
        let Some(previous) = known_devices.remove(path.as_str()) else {
            return true;
        };

        resource
            .event_queue
            .push_drop_oldest(scan_lost_event(&resource.event_state, previous));
        return true;
    };

    if resource
        .filter
        .as_ref()
        .is_some_and(|filter| !matches_scan_filter(&descriptor, filter))
    {
        let Some(previous) = known_devices.remove(&descriptor.id) else {
            return true;
        };

        resource
            .event_queue
            .push_drop_oldest(scan_lost_event(&resource.event_state, previous));
        return true;
    }

    let is_new = !known_devices.contains_key(&descriptor.id);
    let is_changed = known_devices.get(&descriptor.id) != Some(&descriptor);
    known_devices.insert(descriptor.id.clone(), descriptor.clone());

    if is_new {
        resource.device_queue.push_drop_oldest(descriptor.clone());
        resource
            .event_queue
            .push_drop_oldest(scan_discovered_event(&resource.event_state, descriptor));
    }
    // repeated update
    else if is_changed
        || resource
            .filter
            .as_ref()
            .is_some_and(|filter| filter.keep_repeated_devices)
    {
        resource.device_queue.push_drop_oldest(descriptor.clone());
        resource
            .event_queue
            .push_drop_oldest(scan_updated_event(&resource.event_state, descriptor));
    }

    true
}

/// Dispatch one BlueZ signal to one device-session registration.
pub(super) fn handle_device_message(
    resource: &LinuxBluetoothDeviceResource,
    connection: &Connection,
    message: &zbus::Message,
    previous_pair_state: &mut Option<BluetoothPairState>,
    previous_cache: &mut LinuxBluetoothGattCache,
) -> bool {
    // object-manager topology
    if message_interface(message).as_deref() == Some("org.freedesktop.DBus.ObjectManager") {
        match message_member(message).as_deref() {
            // inserted interfaces
            Some("InterfacesAdded") => {
                let Ok((path, interfaces)) =
                    interfaces_added_body(message, "destack.device.bluetooth.session.watch")
                else {
                    return true;
                };
                if !is_child_path(path.as_str(), &resource.device_path) {
                    return true;
                }
                if interface_properties(&interfaces, BLUEZ_GATT_SERVICE_INTERFACE).is_none()
                    && interface_properties(&interfaces, BLUEZ_GATT_CHARACTERISTIC_INTERFACE)
                        .is_none()
                    && interface_properties(&interfaces, BLUEZ_GATT_DESCRIPTOR_INTERFACE).is_none()
                {
                    return true;
                }
            }
            // removed interfaces
            Some("InterfacesRemoved") => {
                let Ok((path, interfaces)) =
                    interfaces_removed_body(message, "destack.device.bluetooth.session.watch")
                else {
                    return true;
                };

                if path.as_str() == resource.device_path
                    && interfaces
                        .iter()
                        .any(|interface_name| interface_name.as_str() == BLUEZ_DEVICE_INTERFACE)
                {
                    resource
                        .event_queue
                        .push_drop_oldest(disconnected_event(&resource.event_state));
                    return false;
                }

                if !is_child_path(path.as_str(), &resource.device_path) {
                    return true;
                }
                if !interfaces.iter().any(|interface_name| {
                    matches!(
                        interface_name.as_str(),
                        BLUEZ_GATT_SERVICE_INTERFACE
                            | BLUEZ_GATT_CHARACTERISTIC_INTERFACE
                            | BLUEZ_GATT_DESCRIPTOR_INTERFACE
                    )
                }) {
                    return true;
                }
            }
            _ => return true,
        }

        let Ok(objects) =
            bluez_managed_objects(connection, "destack.device.bluetooth.session.watch")
        else {
            return true;
        };
        let Ok(current_cache) = gatt_cache(
            &resource.device_path,
            &objects,
            "destack.device.bluetooth.session.watch",
        ) else {
            return true;
        };
        if current_cache != *previous_cache {
            *resource.cache.lock() = current_cache.clone();
            *previous_cache = current_cache;
            resource
                .event_queue
                .push_drop_oldest(gatt_database_changed_event(&resource.event_state));
        }

        return true;
    }

    // device properties
    if message_interface(message).as_deref() != Some("org.freedesktop.DBus.Properties")
        || message_path(message).as_deref() != Some(resource.device_path.as_str())
    {
        return;
    }

    let Ok((interface_name, changed_properties, _invalidated_properties)) =
        properties_changed_body(message, "destack.device.bluetooth.session.watch")
    else {
        return true;
    };
    if interface_name != BLUEZ_DEVICE_INTERFACE {
        return true;
    }

    let current_descriptor = match current_device_descriptor(
        connection,
        &resource.device_path,
        "destack.device.bluetooth.session.watch",
    ) {
        Ok(descriptor) => descriptor,
        Err(_) => return true,
    };
    let Some(current_descriptor) = current_descriptor else {
        resource
            .event_queue
            .push_drop_oldest(disconnected_event(&resource.event_state));
        return false;
    };

    if current_descriptor.pair_state != *previous_pair_state {
        *previous_pair_state = current_descriptor.pair_state;

        if let Some(pair_state) = current_descriptor.pair_state {
            resource
                .event_queue
                .push_drop_oldest(pair_state_changed_event(&resource.event_state, pair_state));
        }
    }

    if changed_properties.contains_key("ServicesResolved") {
        let Ok(objects) =
            bluez_managed_objects(connection, "destack.device.bluetooth.session.watch")
        else {
            return true;
        };
        let Ok(current_cache) = gatt_cache(
            &resource.device_path,
            &objects,
            "destack.device.bluetooth.session.watch",
        ) else {
            return true;
        };
        if current_cache != *previous_cache {
            *resource.cache.lock() = current_cache.clone();
            *previous_cache = current_cache;
            resource
                .event_queue
                .push_drop_oldest(gatt_database_changed_event(&resource.event_state));
        }
    }

    if !current_descriptor.connected {
        resource
            .event_queue
            .push_drop_oldest(disconnected_event(&resource.event_state));

        return false;
    }

    true
}

/// Dispatch one BlueZ signal to one notification registration.
pub(super) fn handle_subscription_message(
    resource: &LinuxBluetoothSubscriptionResource,
    message: &zbus::Message,
) -> bool {
    // characteristic removal
    if message_interface(message).as_deref() == Some("org.freedesktop.DBus.ObjectManager")
        && message_member(message).as_deref() == Some("InterfacesRemoved")
    {
        let Ok((path, interfaces)) =
            interfaces_removed_body(message, "destack.device.bluetooth.gatt.watch")
        else {
            return true;
        };
        if path.as_str() == resource.characteristic_path
            && interfaces.iter().any(|interface_name| {
                interface_name.as_str() == BLUEZ_GATT_CHARACTERISTIC_INTERFACE
            })
        {
            resource.queue.close();

            return false;
        }

        return true;
    }

    // characteristic properties
    if message_interface(message).as_deref() != Some("org.freedesktop.DBus.Properties")
        || message_path(message).as_deref() != Some(resource.characteristic_path.as_str())
    {
        return true;
    }

    let Ok((interface_name, changed_properties, _invalidated_properties)) =
        properties_changed_body(message, "destack.device.bluetooth.gatt.watch")
    else {
        return true;
    };
    if interface_name != BLUEZ_GATT_CHARACTERISTIC_INTERFACE {
        return true;
    }

    if let Some(is_notifying) = optional_bool(&changed_properties, "Notifying")
        && !is_notifying
    {
        resource.queue.close();

        return false;
    }

    let Some(value) = optional_property::<Vec<u8>>(&changed_properties, "Value") else {
        return true;
    };

    resource.queue.push_drop_oldest(gatt_value_event(
        core_platform::monotonic_now_ns(),
        resource.service_id.clone(),
        resource.characteristic_id.clone(),
        resource.service_uuid.clone(),
        resource.characteristic_uuid.clone(),
        value,
    ));

    true
}

/// Resolve one scan resource.
pub(super) fn scan_resource(
    binding: &BindingCallContext,
    handle: resource::BluetoothScanHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<LinuxBluetoothScanResource>> {
    bluetooth_payload(
        binding,
        handle.0,
        ResourceKind::BluetoothScan,
        operation,
        "scan",
    )
}

/// Resolve one device resource.
pub(super) fn device_resource(
    binding: &BindingCallContext,
    handle: resource::BluetoothDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<LinuxBluetoothDeviceResource>> {
    bluetooth_payload(
        binding,
        handle.0,
        ResourceKind::BluetoothDevice,
        operation,
        "device",
    )
}

/// Read one blocking scan device snapshot.
pub(super) fn pop_scan_device(
    resource: &LinuxBluetoothScanResource,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<BluetoothDeviceDescriptorValue> {
    resource
        .device_queue
        .pop_with_timeout_or_else(Duration::from_nanos(timeout_ns), || {
            Err(core_platform::io_would_block(
                operation,
                "bluetooth scan device queue is empty",
            ))
        })
}

/// Read one blocking scan event.
pub(super) fn pop_scan_event(
    binding: &BindingCallContext,
    resource: &LinuxBluetoothScanResource,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<BluetoothScanEvent> {
    let value =
        resource
            .event_queue
            .pop_with_timeout_or_else(Duration::from_nanos(timeout_ns), || {
                Err(core_platform::io_would_block(
                    operation,
                    "bluetooth scan event queue is empty",
                ))
            })?;

    Ok(stored_scan_event(binding, value))
}

/// Read one blocking session event.
pub(super) fn pop_session_event(
    binding: &BindingCallContext,
    resource: &LinuxBluetoothDeviceResource,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<BluetoothSessionEvent> {
    let value =
        resource
            .event_queue
            .pop_with_timeout_or_else(Duration::from_nanos(timeout_ns), || {
                Err(core_platform::io_would_block(
                    operation,
                    "bluetooth session event queue is empty",
                ))
            })?;

    Ok(stored_session_event(binding, value))
}

/// Read one blocking notification event.
pub(super) fn pop_gatt_value_event(
    binding: &BindingCallContext,
    resource: &LinuxBluetoothSubscriptionResource,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<BluetoothGattValueEvent> {
    let value =
        resource
            .queue
            .pop_with_timeout_or_else(Duration::from_nanos(timeout_ns), || {
                Err(core_platform::io_would_block(
                    operation,
                    "bluetooth value event queue is empty",
                ))
            })?;

    Ok(stored_gatt_value_event(binding, value))
}

/// Read one characteristic or descriptor value.
pub(super) fn read_gatt_value(
    connection: &Connection,
    path: &str,
    interface: &str,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let proxy = bluez_proxy(&connection, path, interface, operation)?;
    let options: HashMap<String, OwnedValue> = HashMap::new();

    proxy
        .call::<_, _, Vec<u8>>("ReadValue", &(options,))
        .map_err(|error| bluez_error(operation, "ReadValue", error))
}

/// Write one characteristic or descriptor value.
pub(super) fn write_gatt_value(
    connection: &Connection,
    path: &str,
    interface: &str,
    value: &[u8],
    options: HashMap<String, OwnedValue>,
    operation: &'static str,
) -> RuntimeResult<()> {
    let proxy = bluez_proxy(&connection, path, interface, operation)?;

    proxy
        .call::<_, _, ()>("WriteValue", &(value.to_vec(), options))
        .map_err(|error| bluez_error(operation, "WriteValue", error))
}

/// Resolve one ATT MTU snapshot.
pub(super) fn current_mtu(_resource: &LinuxBluetoothDeviceResource) -> u16 {
    23
}
