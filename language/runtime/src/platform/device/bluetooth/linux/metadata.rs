use super::core::*;

/// Build one BlueZ backend error value.
pub(super) fn bluez_error(
    operation: &'static str,
    action: &'static str,
    error: impl std::fmt::Display,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        None,
        Some(operation.to_string()),
        Some(action.to_string()),
        format!("{action} failed: {error}"),
    ))
    .boxed()
}

/// Open one blocking system bus connection.
pub(super) fn bluez_connection(operation: &'static str) -> RuntimeResult<Connection> {
    Connection::system().map_err(|error| bluez_error(operation, "Connection::system", error))
}

/// Load the managed-object snapshot from BlueZ.
pub(super) fn bluez_managed_objects(
    connection: &Connection,
    operation: &'static str,
) -> RuntimeResult<zbus::fdo::ManagedObjects> {
    let proxy = ObjectManagerProxy::builder(connection)
        .destination(BLUEZ_SERVICE)
        .map_err(|error| bluez_error(operation, "ObjectManagerProxy::destination", error))?
        .path(BLUEZ_ROOT_PATH)
        .map_err(|error| bluez_error(operation, "ObjectManagerProxy::path", error))?
        .build()
        .map_err(|error| bluez_error(operation, "ObjectManagerProxy::build", error))?;

    proxy
        .get_managed_objects()
        .map_err(|error| bluez_error(operation, "ObjectManagerProxy::get_managed_objects", error))
}

/// Return one interface property map when present.
pub(super) fn interface_properties<'a>(
    interfaces: &'a HashMap<zbus::names::OwnedInterfaceName, HashMap<String, OwnedValue>>,
    interface: &str,
) -> Option<&'a HashMap<String, OwnedValue>> {
    interfaces
        .iter()
        .find(|(name, _)| name.as_str() == interface)
        .map(|(_, properties)| properties)
}

/// Decode one required property.
pub(super) fn property<T>(
    properties: &HashMap<String, OwnedValue>,
    name: &'static str,
    operation: &'static str,
) -> RuntimeResult<T>
where
    T: TryFrom<OwnedValue>,
    T::Error: std::fmt::Display,
{
    let value = properties.get(name).ok_or_else(|| {
        RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoInvalidData),
            None,
            None,
            Some(operation.to_string()),
            None,
            format!("bluez object is missing required property {name}"),
        ))
        .boxed()
    })?;

    let value = value
        .try_clone()
        .map_err(|error| bluez_error(operation, "OwnedValue::try_clone", error))?;

    T::try_from(value).map_err(|error| {
        RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoInvalidData),
            None,
            None,
            Some(operation.to_string()),
            None,
            format!("bluez property {name} had one mismatched type: {error}"),
        ))
        .boxed()
    })
}

/// Decode one optional property.
pub(super) fn optional_property<T>(
    properties: &HashMap<String, OwnedValue>,
    name: &'static str,
) -> Option<T>
where
    T: TryFrom<OwnedValue>,
{
    properties
        .get(name)
        .and_then(|value| value.try_clone().ok())
        .and_then(|value| T::try_from(value).ok())
}

/// Decode one optional string property.
pub(super) fn optional_string(
    properties: &HashMap<String, OwnedValue>,
    name: &'static str,
) -> Option<String> {
    optional_property::<String>(properties, name)
}

/// Decode one optional boolean property.
pub(super) fn optional_bool(
    properties: &HashMap<String, OwnedValue>,
    name: &'static str,
) -> Option<bool> {
    optional_property::<bool>(properties, name)
}

/// Decode one optional RSSI property.
pub(super) fn optional_rssi(properties: &HashMap<String, OwnedValue>) -> Option<i32> {
    optional_property::<i16>(properties, "RSSI").map(i32::from)
}

/// Decode one optional tx-power property.
pub(super) fn optional_tx_power(properties: &HashMap<String, OwnedValue>) -> Option<i16> {
    optional_property::<i16>(properties, "TxPower")
}

/// Decode one optional UUID array property.
pub(super) fn optional_uuids(properties: &HashMap<String, OwnedValue>) -> Vec<String> {
    optional_property::<Vec<String>>(properties, "UUIDs").unwrap_or_default()
}

/// Decode one manufacturer-data map.
pub(super) fn manufacturer_data(
    properties: &HashMap<String, OwnedValue>,
) -> Vec<BluetoothAdvertisementManufacturerDataValue> {
    let Some(entries) =
        optional_property::<HashMap<u16, OwnedValue>>(properties, "ManufacturerData")
    else {
        return Vec::new();
    };

    let mut result = Vec::with_capacity(entries.len());

    for (company_id, value) in entries {
        if let Ok(data) = Vec::<u8>::try_from(value) {
            result.push(BluetoothAdvertisementManufacturerDataValue { company_id, data });
        }
    }

    result.sort_by_key(|entry| entry.company_id);
    result
}

/// Decode one service-data map.
pub(super) fn service_data(
    properties: &HashMap<String, OwnedValue>,
) -> Vec<BluetoothAdvertisementServiceDataValue> {
    let Some(entries) = optional_property::<HashMap<String, OwnedValue>>(properties, "ServiceData")
    else {
        return Vec::new();
    };

    let mut result = Vec::with_capacity(entries.len());

    for (service_uuid, value) in entries {
        if let Ok(data) = Vec::<u8>::try_from(value) {
            result.push(BluetoothAdvertisementServiceDataValue { service_uuid, data });
        }
    }

    result.sort_by(|left, right| left.service_uuid.cmp(&right.service_uuid));
    result
}

/// Return whether one managed-object path is nested under one parent path.
pub(super) fn is_child_path(path: &str, parent: &str) -> bool {
    path.len() > parent.len()
        && path.starts_with(parent)
        && path.as_bytes().get(parent.len()) == Some(&b'/')
}

/// Build one adapter descriptor.
pub(super) fn adapter_descriptor(
    path: &OwnedObjectPath,
    interfaces: &HashMap<zbus::names::OwnedInterfaceName, HashMap<String, OwnedValue>>,
    operation: &'static str,
) -> RuntimeResult<BluetoothAdapterDescriptorValue> {
    let properties =
        interface_properties(interfaces, BLUEZ_ADAPTER_INTERFACE).ok_or_else(|| {
            core_platform::io_operation_error(
                operation,
                Some(PlatformErrorCode::IoInvalidData),
                "bluez adapter object is missing org.bluez.Adapter1",
            )
        })?;

    let name = optional_string(properties, "Alias")
        .or_else(|| optional_string(properties, "Name"))
        .unwrap_or_else(|| path.as_str().to_string());

    Ok(BluetoothAdapterDescriptorValue {
        id: path.as_str().to_string(),
        name,
        powered: optional_bool(properties, "Powered").unwrap_or(true),
        discovering: optional_bool(properties, "Discovering"),
        extended_advertising: None,
    })
}

/// Load the current adapter snapshot from BlueZ.
pub(super) fn adapter_snapshot(
    connection: &Connection,
    operation: &'static str,
) -> RuntimeResult<BTreeMap<String, BluetoothAdapterDescriptorValue>> {
    let objects = bluez_managed_objects(connection, operation)?;
    let mut snapshot = BTreeMap::new();

    for (path, interfaces) in &objects {
        if interface_properties(interfaces, BLUEZ_ADAPTER_INTERFACE).is_none() {
            continue;
        }

        let descriptor = adapter_descriptor(path, interfaces, operation)?;
        snapshot.insert(descriptor.id.clone(), descriptor);
    }

    Ok(snapshot)
}

/// Find one current adapter descriptor by id.
pub(super) fn current_adapter_descriptor(
    connection: &Connection,
    adapter_path: &str,
    operation: &'static str,
) -> RuntimeResult<Option<BluetoothAdapterDescriptorValue>> {
    let objects = bluez_managed_objects(connection, operation)?;
    let path = OwnedObjectPath::try_from(adapter_path.to_string()).map_err(|_| {
        core_platform::invalid_argument("adapterId", "adapter identifier must be one object path")
    })?;
    let Some(interfaces) = objects.get(&path) else {
        return Ok(None);
    };
    if interface_properties(interfaces, BLUEZ_ADAPTER_INTERFACE).is_none() {
        return Ok(None);
    }

    Ok(Some(adapter_descriptor(&path, interfaces, operation)?))
}

/// Build one device descriptor.
pub(super) fn device_descriptor(
    path: &OwnedObjectPath,
    properties: &HashMap<String, OwnedValue>,
    operation: &'static str,
) -> RuntimeResult<BluetoothDeviceDescriptorValue> {
    let address = property::<String>(properties, "Address", operation)?;
    let name = optional_string(properties, "Alias").or_else(|| optional_string(properties, "Name"));
    let is_paired = optional_bool(properties, "Paired").unwrap_or(false);
    let connected = optional_bool(properties, "Connected").unwrap_or(false);
    let transport = optional_string(properties, "AddressType").map(|value| match value.as_str() {
        "public" | "random" | "le" => BluetoothLeTransport::LowEnergy,
        "dual" => BluetoothLeTransport::DualMode,
        _ => BluetoothLeTransport::Unknown,
    });

    Ok(BluetoothDeviceDescriptorValue {
        id: path.as_str().to_string(),
        address: Some(address),
        name,
        rssi: optional_rssi(properties),
        pair_state: Some(if is_paired {
            BluetoothPairState::Paired
        } else {
            BluetoothPairState::Unpaired
        }),
        connected,
        connectable: optional_bool(properties, "Connectable"),
        transport,
        advertisement: BluetoothAdvertisementDataValue {
            local_name: optional_string(properties, "Name")
                .or_else(|| optional_string(properties, "Alias")),
            tx_power: optional_tx_power(properties),
            service_uuids: optional_uuids(properties),
            manufacturer_data: manufacturer_data(properties),
            service_data: service_data(properties),
        },
    })
}

/// Build one stable service identifier.
pub(super) fn service_id(path: &OwnedObjectPath) -> String {
    path.as_str().to_string()
}

/// Build one stable characteristic identifier.
pub(super) fn characteristic_id(path: &OwnedObjectPath) -> String {
    path.as_str().to_string()
}

/// Build one stable descriptor identifier.
pub(super) fn descriptor_id(path: &OwnedObjectPath) -> String {
    path.as_str().to_string()
}

/// Build one characteristic property set from BlueZ flags.
pub(super) fn characteristic_properties(flags: &[String]) -> BluetoothGattCharacteristicProperties {
    let has_flag = |expected: &str| flags.iter().any(|flag| flag == expected);

    BluetoothGattCharacteristicProperties {
        broadcast: has_flag("broadcast"),
        read: has_flag("read"),
        write_without_response: has_flag("write-without-response")
            || has_flag("write-without-response-acquired"),
        write: has_flag("write") || has_flag("reliable-write"),
        notify: has_flag("notify"),
        indicate: has_flag("indicate"),
        authenticated_signed_writes: has_flag("authenticated-signed-writes"),
        reliable_write: has_flag("reliable-write"),
        writable_auxiliaries: has_flag("writable-auxiliaries"),
    }
}

/// Build one GATT cache from one device subtree snapshot.
pub(super) fn gatt_cache(
    device_path: &str,
    objects: &zbus::fdo::ManagedObjects,
    operation: &'static str,
) -> RuntimeResult<LinuxBluetoothGattCache> {
    let mut services = BTreeMap::new();
    let mut characteristics = BTreeMap::new();
    let mut descriptors = BTreeMap::new();

    for (path, interfaces) in objects {
        if !is_child_path(path.as_str(), device_path) {
            continue;
        }

        if let Some(properties) = interface_properties(interfaces, BLUEZ_GATT_SERVICE_INTERFACE) {
            let id = service_id(path);
            let included_service_paths =
                optional_property::<Vec<OwnedObjectPath>>(properties, "Includes")
                    .unwrap_or_default();
            let included_service_ids = included_service_paths
                .into_iter()
                .map(|path| service_id(&path))
                .collect();

            services.insert(
                id.clone(),
                BluetoothGattServiceValue {
                    id,
                    uuid: property::<String>(properties, "UUID", operation)?,
                    primary: optional_bool(properties, "Primary").unwrap_or(true),
                    included_service_ids,
                },
            );
            continue;
        }

        if let Some(properties) =
            interface_properties(interfaces, BLUEZ_GATT_CHARACTERISTIC_INTERFACE)
        {
            let id = characteristic_id(path);
            let service_path = property::<OwnedObjectPath>(properties, "Service", operation)?;
            let flags = optional_property::<Vec<String>>(properties, "Flags").unwrap_or_default();

            characteristics.insert(
                id.clone(),
                BluetoothGattCharacteristicValue {
                    id,
                    service_id: service_id(&service_path),
                    uuid: property::<String>(properties, "UUID", operation)?,
                    properties: characteristic_properties(&flags),
                },
            );
            continue;
        }

        if let Some(properties) = interface_properties(interfaces, BLUEZ_GATT_DESCRIPTOR_INTERFACE)
        {
            let id = descriptor_id(path);
            let characteristic_path =
                property::<OwnedObjectPath>(properties, "Characteristic", operation)?;
            let characteristic_id = characteristic_id(&characteristic_path);
            let service_id = characteristics
                .get(&characteristic_id)
                .map(|characteristic| characteristic.service_id.clone())
                .unwrap_or_else(|| device_path.to_string());

            descriptors.insert(
                id.clone(),
                BluetoothGattDescriptorValue {
                    id,
                    service_id,
                    characteristic_id,
                    uuid: property::<String>(properties, "UUID", operation)?,
                },
            );
        }
    }

    Ok(LinuxBluetoothGattCache {
        services,
        characteristics,
        descriptors,
    })
}

/// Build one BlueZ proxy.
pub(super) fn bluez_proxy<'a>(
    connection: &'a Connection,
    path: &'a str,
    interface: &'a str,
    operation: &'static str,
) -> RuntimeResult<zbus::blocking::Proxy<'a>> {
    zbus::blocking::Proxy::new(connection, BLUEZ_SERVICE, path, interface)
        .map_err(|error| bluez_error(operation, "Proxy::new", error))
}

/// Register one set of BlueZ signal match rules on a dedicated connection.
pub(super) fn bluez_signal_connection(
    operation: &'static str,
    rules: &[MatchRule<'static>],
) -> RuntimeResult<Connection> {
    let connection = bluez_connection(operation)?;
    let proxy = DBusProxy::new(&connection)
        .map_err(|error| bluez_error(operation, "DBusProxy::new", error))?;

    for rule in rules {
        proxy
            .add_match_rule(rule.clone())
            .map_err(|error| bluez_error(operation, "DBusProxy::add_match_rule", error))?;
    }

    Ok(connection)
}

/// Build one root object-manager signal match rule.
pub(super) fn bluez_object_manager_rule() -> RuntimeResult<MatchRule<'static>> {
    MatchRule::builder()
        .msg_type(MessageType::Signal)
        .sender(BLUEZ_SERVICE)
        .map_err(|error| {
            bluez_error(
                "destack.device.bluetooth.signal",
                "MatchRule::sender",
                error,
            )
        })?
        .interface("org.freedesktop.DBus.ObjectManager")
        .map_err(|error| {
            bluez_error(
                "destack.device.bluetooth.signal",
                "MatchRule::interface",
                error,
            )
        })?
        .path(BLUEZ_ROOT_PATH)
        .map_err(|error| bluez_error("destack.device.bluetooth.signal", "MatchRule::path", error))
        .map(|builder| builder.build())
}

/// Build one properties-changed signal match rule for one exact path.
pub(super) fn bluez_properties_changed_rule(
    path: &str,
    interface_name: Option<&'static str>,
) -> RuntimeResult<MatchRule<'static>> {
    let builder = MatchRule::builder()
        .msg_type(MessageType::Signal)
        .sender(BLUEZ_SERVICE)
        .map_err(|error| {
            bluez_error(
                "destack.device.bluetooth.signal",
                "MatchRule::sender",
                error,
            )
        })?
        .interface("org.freedesktop.DBus.Properties")
        .map_err(|error| {
            bluez_error(
                "destack.device.bluetooth.signal",
                "MatchRule::interface",
                error,
            )
        })?
        .member("PropertiesChanged")
        .map_err(|error| {
            bluez_error(
                "destack.device.bluetooth.signal",
                "MatchRule::member",
                error,
            )
        })?
        .path(path.to_string())
        .map_err(|error| {
            bluez_error("destack.device.bluetooth.signal", "MatchRule::path", error)
        })?;
    let builder = match interface_name {
        Some(interface_name) => builder.add_arg(interface_name).map_err(|error| {
            bluez_error(
                "destack.device.bluetooth.signal",
                "MatchRule::add_arg",
                error,
            )
        })?,
        None => builder,
    };

    Ok(builder.build())
}

/// Build one properties-changed signal match rule for one path namespace.
pub(super) fn bluez_properties_changed_rule_namespace(
    path_namespace: &str,
    interface_name: Option<&'static str>,
) -> RuntimeResult<MatchRule<'static>> {
    let builder = MatchRule::builder()
        .msg_type(MessageType::Signal)
        .sender(BLUEZ_SERVICE)
        .map_err(|error| {
            bluez_error(
                "destack.device.bluetooth.signal",
                "MatchRule::sender",
                error,
            )
        })?
        .interface("org.freedesktop.DBus.Properties")
        .map_err(|error| {
            bluez_error(
                "destack.device.bluetooth.signal",
                "MatchRule::interface",
                error,
            )
        })?
        .member("PropertiesChanged")
        .map_err(|error| {
            bluez_error(
                "destack.device.bluetooth.signal",
                "MatchRule::member",
                error,
            )
        })?
        .path_namespace(path_namespace.to_string())
        .map_err(|error| {
            bluez_error(
                "destack.device.bluetooth.signal",
                "MatchRule::path_namespace",
                error,
            )
        })?;
    let builder = match interface_name {
        Some(interface_name) => builder.add_arg(interface_name).map_err(|error| {
            bluez_error(
                "destack.device.bluetooth.signal",
                "MatchRule::add_arg",
                error,
            )
        })?,
        None => builder,
    };

    Ok(builder.build())
}
