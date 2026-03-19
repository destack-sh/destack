use super::*;

/// Validate one Android adapter selection against the single-adapter host bridge.
pub(crate) fn ensure_android_primary_adapter(
    binding: &BindingCallContext,
    adapter_id: &str,
    operation: &'static str,
) -> RuntimeResult<()> {
    let adapters = super::descriptors::read_adapter_descriptors(binding, operation)?;
    let selected_adapter = adapters
        .iter()
        .find(|adapter| adapter.id == adapter_id)
        .ok_or_else(|| {
            core_platform::io_not_found(
                operation,
                format!("bluetooth adapter {adapter_id} not found"),
            )
        })?;
    let primary_adapter = adapters.first().ok_or_else(|| {
        core_platform::io_not_found(operation, "no bluetooth adapter is available")
    })?;

    if selected_adapter.id != primary_adapter.id {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(())
}

/// Decode one Bluetooth transport code.
fn decode_transport(code: u32, operation: &'static str) -> RuntimeResult<BluetoothLeTransport> {
    match code {
        1 => Ok(BluetoothLeTransport::LowEnergy),
        3 => Ok(BluetoothLeTransport::DualMode),
        2 => Ok(BluetoothLeTransport::Unknown),
        _ => Err(invalid_data(
            operation,
            format!("android host returned one unknown bluetooth transport code {code}"),
        )),
    }
}

/// Decode one optional Bluetooth transport code.
fn decode_optional_transport(
    code: u32,
    operation: &'static str,
) -> RuntimeResult<Option<BluetoothLeTransport>> {
    if code == 0 {
        return Ok(None);
    }

    Ok(Some(decode_transport(code, operation)?))
}

/// Decode one optional Bluetooth pair-state code.
pub(crate) fn decode_optional_pair_state(
    code: u32,
    operation: &'static str,
) -> RuntimeResult<Option<BluetoothPairState>> {
    match code {
        0 => Ok(None),
        1 => Ok(Some(BluetoothPairState::Unknown)),
        2 => Ok(Some(BluetoothPairState::Unpaired)),
        3 => Ok(Some(BluetoothPairState::Pairing)),
        4 => Ok(Some(BluetoothPairState::Paired)),
        _ => Err(invalid_data(
            operation,
            format!("android host returned one unknown bluetooth pair-state code {code}"),
        )),
    }
}

/// Decode one Bluetooth characteristic property bitset.
fn decode_characteristic_properties(flags: u32) -> BluetoothGattCharacteristicProperties {
    BluetoothGattCharacteristicProperties {
        broadcast: flags & BLUETOOTH_CHARACTERISTIC_PROPERTY_BROADCAST != 0,
        read: flags & BLUETOOTH_CHARACTERISTIC_PROPERTY_READ != 0,
        write_without_response: flags & BLUETOOTH_CHARACTERISTIC_PROPERTY_WRITE_WITHOUT_RESPONSE
            != 0,
        write: flags & BLUETOOTH_CHARACTERISTIC_PROPERTY_WRITE != 0,
        notify: flags & BLUETOOTH_CHARACTERISTIC_PROPERTY_NOTIFY != 0,
        indicate: flags & BLUETOOTH_CHARACTERISTIC_PROPERTY_INDICATE != 0,
        authenticated_signed_writes: flags & BLUETOOTH_CHARACTERISTIC_PROPERTY_AUTH_SIGNED_WRITE
            != 0,
        reliable_write: flags & BLUETOOTH_CHARACTERISTIC_PROPERTY_RELIABLE_WRITE != 0,
        writable_auxiliaries: flags & BLUETOOTH_CHARACTERISTIC_PROPERTY_WRITABLE_AUXILIARIES != 0,
    }
}

/// Decode one Android Bluetooth adapter row.
pub(crate) fn decode_adapter_descriptor(
    header: &AndroidHostBluetoothAdapterDescriptorHeader,
    string_bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<BluetoothAdapterDescriptorValue> {
    Ok(BluetoothAdapterDescriptorValue {
        id: decode_required_string(
            string_bytes,
            header.id_offset,
            header.id_len,
            "id",
            operation,
        )?,
        name: decode_required_string(
            string_bytes,
            header.name_offset,
            header.name_len,
            "name",
            operation,
        )?,
        powered: header.is_powered != 0,
        discovering: Some(header.is_discovering != 0),
        extended_advertising: Some(
            header.capability_flags & BLUETOOTH_ADAPTER_CAPABILITY_EXTENDED_ADVERTISING != 0,
        ),
    })
}

/// Decode one Android Bluetooth device row.
pub(crate) fn decode_device_descriptor(
    header: &AndroidHostBluetoothDeviceDescriptorHeader,
    string_bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<BluetoothDeviceDescriptorValue> {
    let pair_state = decode_optional_pair_state(header.pair_state, operation)?;
    Ok(BluetoothDeviceDescriptorValue {
        id: decode_required_string(
            string_bytes,
            header.id_offset,
            header.id_len,
            "id",
            operation,
        )?,
        address: decode_optional_string(
            string_bytes,
            header.address_offset,
            header.address_len,
            "address",
            operation,
        )?,
        name: decode_optional_string(
            string_bytes,
            header.name_offset,
            header.name_len,
            "name",
            operation,
        )?,
        rssi: Some(header.rssi_dbm),
        pair_state,
        connected: header.is_connected != 0,
        connectable: None,
        transport: decode_optional_transport(header.transport, operation)?,
        advertisement: BluetoothAdvertisementDataValue {
            local_name: decode_optional_string(
                string_bytes,
                header.local_name_offset,
                header.local_name_len,
                "localName",
                operation,
            )?,
            tx_power: None,
            service_uuids: Vec::new(),
            manufacturer_data: Vec::new(),
            service_data: Vec::new(),
        },
    })
}

/// Decode one Android Bluetooth service row.
pub(crate) fn decode_service_descriptor(
    header: &AndroidHostBluetoothGattServiceHeader,
    string_bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<BluetoothGattServiceValue> {
    Ok(BluetoothGattServiceValue {
        id: decode_required_string(
            string_bytes,
            header.id_offset,
            header.id_len,
            "id",
            operation,
        )?,
        uuid: decode_required_string(
            string_bytes,
            header.uuid_offset,
            header.uuid_len,
            "uuid",
            operation,
        )?,
        primary: header.is_primary != 0,
        included_service_ids: Vec::new(),
    })
}

/// Decode one Android Bluetooth characteristic row.
pub(crate) fn decode_characteristic_descriptor(
    header: &AndroidHostBluetoothGattCharacteristicHeader,
    string_bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<BluetoothGattCharacteristicValue> {
    Ok(BluetoothGattCharacteristicValue {
        id: decode_required_string(
            string_bytes,
            header.id_offset,
            header.id_len,
            "id",
            operation,
        )?,
        service_id: decode_required_string(
            string_bytes,
            header.service_id_offset,
            header.service_id_len,
            "serviceId",
            operation,
        )?,
        uuid: decode_required_string(
            string_bytes,
            header.uuid_offset,
            header.uuid_len,
            "uuid",
            operation,
        )?,
        properties: decode_characteristic_properties(header.property_flags),
    })
}

/// Decode one Android Bluetooth descriptor row.
pub(crate) fn decode_descriptor_descriptor(
    header: &AndroidHostBluetoothGattDescriptorHeader,
    string_bytes: &[u8],
    cache: &AndroidBluetoothGattCache,
    operation: &'static str,
) -> RuntimeResult<BluetoothGattDescriptorValue> {
    let characteristic_id = decode_required_string(
        string_bytes,
        header.characteristic_id_offset,
        header.characteristic_id_len,
        "characteristicId",
        operation,
    )?;
    let characteristic = cache
        .characteristics
        .get(&characteristic_id)
        .ok_or_else(|| {
            invalid_data(
                operation,
                "android host returned one descriptor before its parent characteristic",
            )
        })?;

    Ok(BluetoothGattDescriptorValue {
        id: decode_required_string(
            string_bytes,
            header.id_offset,
            header.id_len,
            "id",
            operation,
        )?,
        service_id: characteristic.service_id.clone(),
        characteristic_id,
        uuid: decode_required_string(
            string_bytes,
            header.uuid_offset,
            header.uuid_len,
            "uuid",
            operation,
        )?,
    })
}

/// Return whether one Android scan filter is bridgeable.
pub(crate) fn ensure_android_scan_filter_supported(
    filter: &Option<BluetoothScanFilterValue>,
    operation: &'static str,
) -> RuntimeResult<()> {
    let Some(filter) = filter.as_ref() else {
        return Ok(());
    };

    if !filter.service_uuids.is_empty()
        || !filter.manufacturer_data.is_empty()
        || !filter.service_data.is_empty()
    {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(())
}

/// Build one host filter flag payload.
pub(crate) fn android_filter_flags(filter: &Option<BluetoothScanFilterValue>) -> u32 {
    let Some(filter) = filter.as_ref() else {
        return 0;
    };

    let mut flags = 0u32;

    if let Some(scan_mode) = filter.scan_mode {
        flags |= (scan_mode as u32) << 8;
    }
    if let Some(primary_phy) = filter.primary_phy {
        flags |= (primary_phy as u32) << 12;
    }
    if let Some(secondary_phy) = filter.secondary_phy {
        flags |= (secondary_phy as u32) << 16;
    }
    if filter.keep_repeated_devices {
        flags |= 1 << 20;
    }

    flags
}

/// Grow one Bluetooth row and string buffer after one `bufferTooSmall` result.
pub(crate) fn grow_row_and_string_buffers<T: Default + Clone>(
    headers: &mut Vec<T>,
    required_headers: usize,
    string_bytes: &mut Vec<u8>,
    required_string_bytes: usize,
    operation: &'static str,
) -> RuntimeResult<()> {
    let next_header_capacity = headers.len().max(required_headers).saturating_mul(2);
    let next_string_capacity = string_bytes
        .len()
        .max(required_string_bytes)
        .saturating_mul(2);

    if next_header_capacity > MAX_BLUETOOTH_ROW_CAPACITY {
        return Err(invalid_data(
            operation,
            "android host bluetooth result exceeded the maximum row count",
        ));
    }

    if next_string_capacity > MAX_BLUETOOTH_STRING_CAPACITY {
        return Err(invalid_data(
            operation,
            "android host bluetooth result exceeded the maximum string payload size",
        ));
    }

    headers.resize(next_header_capacity, T::default());
    string_bytes.resize(next_string_capacity, 0);

    Ok(())
}
