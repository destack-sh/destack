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
        1 => Ok(BluetoothLeTransport::Unknown),
        2 => Ok(BluetoothLeTransport::LowEnergy),
        3 => Ok(BluetoothLeTransport::DualMode),
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

/// Read one serialized payload slice from the shared Android byte buffer.
fn decode_payload_slice<'a>(
    bytes: &'a [u8],
    offset: u32,
    len: u32,
    field: &'static str,
    operation: &'static str,
) -> RuntimeResult<&'a [u8]> {
    let start = offset as usize;
    let end = start.saturating_add(len as usize);

    if end > bytes.len() {
        return Err(invalid_data(
            operation,
            format!("android host returned one out-of-range bluetooth {field} payload"),
        ));
    }

    Ok(&bytes[start..end])
}

/// Read one little-endian `u32` from one payload cursor.
fn decode_u32(bytes: &[u8], cursor: &mut usize, operation: &'static str) -> RuntimeResult<u32> {
    let end = cursor.saturating_add(4);

    if end > bytes.len() {
        return Err(invalid_data(
            operation,
            "android host returned one truncated bluetooth payload",
        ));
    }

    let value = u32::from_le_bytes(bytes[*cursor..end].try_into().map_err(|_| {
        invalid_data(
            operation,
            "android host returned one malformed bluetooth integer payload",
        )
    })?);
    *cursor = end;

    Ok(value)
}

/// Read one little-endian `u16` from one payload cursor.
fn decode_u16(bytes: &[u8], cursor: &mut usize, operation: &'static str) -> RuntimeResult<u16> {
    let end = cursor.saturating_add(2);

    if end > bytes.len() {
        return Err(invalid_data(
            operation,
            "android host returned one truncated bluetooth payload",
        ));
    }

    let value = u16::from_le_bytes(bytes[*cursor..end].try_into().map_err(|_| {
        invalid_data(
            operation,
            "android host returned one malformed bluetooth integer payload",
        )
    })?);
    *cursor = end;

    Ok(value)
}

/// Read one exact byte slice from one payload cursor.
fn decode_bytes<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    len: usize,
    operation: &'static str,
) -> RuntimeResult<&'a [u8]> {
    let end = cursor.saturating_add(len);

    if end > bytes.len() {
        return Err(invalid_data(
            operation,
            "android host returned one truncated bluetooth payload",
        ));
    }

    let value = &bytes[*cursor..end];
    *cursor = end;

    Ok(value)
}

/// Append one little-endian `u32` to one payload buffer.
fn encode_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

/// Append one little-endian `u16` to one payload buffer.
fn encode_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

/// Append one byte slice to one payload buffer and return its offset and length.
fn append_payload(bytes: &mut Vec<u8>, payload: &[u8]) -> (u32, u32) {
    let offset = bytes.len() as u32;
    bytes.extend_from_slice(payload);

    (offset, payload.len() as u32)
}

/// Append one UTF-8 string to one payload buffer and return its offset and length.
fn append_payload_string(bytes: &mut Vec<u8>, value: &str) -> (u32, u32) {
    append_payload(bytes, value.as_bytes())
}

/// Encode one string list into one Android bluetooth payload.
fn encode_string_list(values: &[String]) -> Vec<u8> {
    let mut bytes = Vec::new();

    encode_u32(&mut bytes, values.len() as u32);

    for value in values {
        encode_u32(&mut bytes, value.len() as u32);
        bytes.extend_from_slice(value.as_bytes());
    }

    bytes
}

/// Decode one string list from one Android bluetooth payload.
fn decode_string_list(bytes: &[u8], operation: &'static str) -> RuntimeResult<Vec<String>> {
    let mut cursor = 0usize;
    let count = decode_u32(bytes, &mut cursor, operation)? as usize;
    let mut values = Vec::with_capacity(count);

    for _ in 0..count {
        let len = decode_u32(bytes, &mut cursor, operation)? as usize;
        let value = decode_bytes(bytes, &mut cursor, len, operation)?;
        values.push(String::from_utf8(value.to_vec()).map_err(|_| {
            invalid_data(
                operation,
                "android host returned one non-utf8 bluetooth string payload",
            )
        })?);
    }

    Ok(values)
}

/// Encode one manufacturer-data filter list into one Android bluetooth payload.
fn encode_manufacturer_filters(values: &[BluetoothManufacturerDataFilterValue]) -> Vec<u8> {
    let mut bytes = Vec::new();

    encode_u32(&mut bytes, values.len() as u32);

    for value in values {
        let data_prefix = value
            .data
            .as_ref()
            .map_or(&[][..], |data| data.data_prefix.as_slice());
        let mask = value
            .data
            .as_ref()
            .and_then(|data| data.mask.as_deref())
            .unwrap_or(&[]);

        encode_u16(&mut bytes, value.company_id);
        encode_u16(&mut bytes, 0);
        encode_u32(&mut bytes, data_prefix.len() as u32);
        encode_u32(&mut bytes, mask.len() as u32);
        bytes.extend_from_slice(data_prefix);
        bytes.extend_from_slice(mask);
    }

    bytes
}

/// Decode one manufacturer-data advertisement list from one Android bluetooth payload.
fn decode_manufacturer_data(
    bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<BluetoothAdvertisementManufacturerDataValue>> {
    let mut cursor = 0usize;
    let count = decode_u32(bytes, &mut cursor, operation)? as usize;
    let mut values = Vec::with_capacity(count);

    for _ in 0..count {
        let company_id = decode_u16(bytes, &mut cursor, operation)?;
        let _reserved = decode_u16(bytes, &mut cursor, operation)?;
        let data_len = decode_u32(bytes, &mut cursor, operation)? as usize;
        let data = decode_bytes(bytes, &mut cursor, data_len, operation)?.to_vec();

        values.push(BluetoothAdvertisementManufacturerDataValue { company_id, data });
    }

    Ok(values)
}

/// Encode one manufacturer-data advertisement list into one Android bluetooth payload.
fn encode_manufacturer_data(values: &[(u16, &[u8])]) -> Vec<u8> {
    let mut bytes = Vec::new();

    encode_u32(&mut bytes, values.len() as u32);

    for (company_id, data) in values {
        encode_u16(&mut bytes, *company_id);
        encode_u16(&mut bytes, 0);
        encode_u32(&mut bytes, data.len() as u32);
        bytes.extend_from_slice(data);
    }

    bytes
}

/// Encode one service-data filter list into one Android bluetooth payload.
fn encode_service_data_filters(values: &[BluetoothServiceDataFilterValue]) -> Vec<u8> {
    let mut bytes = Vec::new();

    encode_u32(&mut bytes, values.len() as u32);

    for value in values {
        let data_prefix = value
            .data
            .as_ref()
            .map_or(&[][..], |data| data.data_prefix.as_slice());
        let mask = value
            .data
            .as_ref()
            .and_then(|data| data.mask.as_deref())
            .unwrap_or(&[]);

        encode_u32(&mut bytes, value.service_uuid.len() as u32);
        encode_u32(&mut bytes, data_prefix.len() as u32);
        encode_u32(&mut bytes, mask.len() as u32);
        bytes.extend_from_slice(value.service_uuid.as_bytes());
        bytes.extend_from_slice(data_prefix);
        bytes.extend_from_slice(mask);
    }

    bytes
}

/// Decode one service-data advertisement list from one Android bluetooth payload.
fn decode_service_data(
    bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<BluetoothAdvertisementServiceDataValue>> {
    let mut cursor = 0usize;
    let count = decode_u32(bytes, &mut cursor, operation)? as usize;
    let mut values = Vec::with_capacity(count);

    for _ in 0..count {
        let service_uuid_len = decode_u32(bytes, &mut cursor, operation)? as usize;
        let data_len = decode_u32(bytes, &mut cursor, operation)? as usize;
        let service_uuid = String::from_utf8(
            decode_bytes(bytes, &mut cursor, service_uuid_len, operation)?.to_vec(),
        )
        .map_err(|_| {
            invalid_data(
                operation,
                "android host returned one non-utf8 bluetooth service uuid payload",
            )
        })?;
        let data = decode_bytes(bytes, &mut cursor, data_len, operation)?.to_vec();

        values.push(BluetoothAdvertisementServiceDataValue { service_uuid, data });
    }

    Ok(values)
}

/// Encode one service-data advertisement list into one Android bluetooth payload.
fn encode_service_data(values: &[(&str, &[u8])]) -> Vec<u8> {
    let mut bytes = Vec::new();

    encode_u32(&mut bytes, values.len() as u32);

    for (service_uuid, data) in values {
        encode_u32(&mut bytes, service_uuid.len() as u32);
        encode_u32(&mut bytes, data.len() as u32);
        bytes.extend_from_slice(service_uuid.as_bytes());
        bytes.extend_from_slice(data);
    }

    bytes
}

/// Encode one Android scan filter into the host callback payload.
pub(crate) fn encode_android_scan_filter(
    filter: &Option<BluetoothScanFilterValue>,
) -> (AndroidHostBluetoothScanFilterHeader, Vec<u8>) {
    let Some(filter) = filter.as_ref() else {
        return (AndroidHostBluetoothScanFilterHeader::default(), Vec::new());
    };

    let mut bytes = Vec::new();
    let (name_offset, name_len) = filter
        .name
        .as_deref()
        .map_or((0, 0), |value| append_payload_string(&mut bytes, value));
    let (name_prefix_offset, name_prefix_len) = filter
        .name_prefix
        .as_deref()
        .map_or((0, 0), |value| append_payload_string(&mut bytes, value));
    let service_uuid_payload = encode_string_list(&filter.service_uuids);
    let (service_uuids_offset, service_uuids_len) = if filter.service_uuids.is_empty() {
        (0, 0)
    } else {
        append_payload(&mut bytes, &service_uuid_payload)
    };
    let manufacturer_data_payload = encode_manufacturer_filters(&filter.manufacturer_data);
    let (manufacturer_data_offset, manufacturer_data_len) = if filter.manufacturer_data.is_empty() {
        (0, 0)
    } else {
        append_payload(&mut bytes, &manufacturer_data_payload)
    };
    let service_data_payload = encode_service_data_filters(&filter.service_data);
    let (service_data_offset, service_data_len) = if filter.service_data.is_empty() {
        (0, 0)
    } else {
        append_payload(&mut bytes, &service_data_payload)
    };

    (
        AndroidHostBluetoothScanFilterHeader {
            is_present: 1,
            name_offset,
            name_len,
            name_prefix_offset,
            name_prefix_len,
            has_minimum_rssi: u32::from(filter.minimum_rssi.is_some()),
            minimum_rssi_dbm: filter.minimum_rssi.unwrap_or_default(),
            scan_mode: filter.scan_mode.map_or(0, |value| value as u32),
            primary_phy: filter.primary_phy.map_or(0, |value| value as u32),
            secondary_phy: filter.secondary_phy.map_or(0, |value| value as u32),
            keep_repeated_devices: u32::from(filter.keep_repeated_devices),
            service_uuids_offset,
            service_uuids_len,
            manufacturer_data_offset,
            manufacturer_data_len,
            service_data_offset,
            service_data_len,
        },
        bytes,
    )
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
    let service_uuids = if header.service_uuids_len == 0 {
        Vec::new()
    } else {
        decode_string_list(
            decode_payload_slice(
                string_bytes,
                header.service_uuids_offset,
                header.service_uuids_len,
                "serviceUuids",
                operation,
            )?,
            operation,
        )?
    };
    let manufacturer_data = if header.manufacturer_data_len == 0 {
        Vec::new()
    } else {
        decode_manufacturer_data(
            decode_payload_slice(
                string_bytes,
                header.manufacturer_data_offset,
                header.manufacturer_data_len,
                "manufacturerData",
                operation,
            )?,
            operation,
        )?
    };
    let service_data = if header.service_data_len == 0 {
        Vec::new()
    } else {
        decode_service_data(
            decode_payload_slice(
                string_bytes,
                header.service_data_offset,
                header.service_data_len,
                "serviceData",
                operation,
            )?,
            operation,
        )?
    };

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
            service_uuids,
            manufacturer_data,
            service_data,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::device::{
        BluetoothDataFilterValue, BluetoothManufacturerDataFilterValue, BluetoothPhy,
        BluetoothScanFilterValue, BluetoothScanMode, BluetoothServiceDataFilterValue,
    };

    /// Build one Android bluetooth scan filter with the full public payload shape.
    fn scan_filter() -> BluetoothScanFilterValue {
        BluetoothScanFilterValue {
            service_uuids: vec![String::from("12345678-1234-1234-1234-1234567890ab")],
            name: Some(String::from("Destack Peripheral")),
            name_prefix: Some(String::from("Destack")),
            manufacturer_data: vec![BluetoothManufacturerDataFilterValue {
                company_id: 0x1337,
                data: Some(BluetoothDataFilterValue {
                    data_prefix: vec![0xaa, 0xbb],
                    mask: Some(vec![0xff, 0x0f]),
                }),
            }],
            service_data: vec![BluetoothServiceDataFilterValue {
                service_uuid: String::from("180d"),
                data: Some(BluetoothDataFilterValue {
                    data_prefix: vec![0xcc, 0xdd],
                    mask: Some(vec![0xf0, 0x0f]),
                }),
            }],
            keep_repeated_devices: true,
            minimum_rssi: Some(-70),
            scan_mode: Some(BluetoothScanMode::Passive),
            primary_phy: Some(BluetoothPhy::Le2M),
            secondary_phy: Some(BluetoothPhy::LeCoded),
        }
    }

    /// Decode Android Bluetooth device-type codes without swapping classic and LE.
    #[test]
    fn test_decode_optional_transport_matches_android_device_type_codes() {
        assert_eq!(decode_optional_transport(0, "test").unwrap(), None);
        assert_eq!(
            decode_optional_transport(1, "test").unwrap(),
            Some(BluetoothLeTransport::Unknown)
        );
        assert_eq!(
            decode_optional_transport(2, "test").unwrap(),
            Some(BluetoothLeTransport::LowEnergy)
        );
        assert_eq!(
            decode_optional_transport(3, "test").unwrap(),
            Some(BluetoothLeTransport::DualMode)
        );
    }

    /// Encode one full Android scan filter into the structured host payload.
    #[test]
    fn test_encode_android_scan_filter_preserves_full_payload() {
        let filter = Some(scan_filter());

        let (header, bytes) = encode_android_scan_filter(&filter);

        assert_eq!(header.is_present, 1);
        assert_eq!(header.has_minimum_rssi, 1);
        assert_eq!(header.minimum_rssi_dbm, -70);
        assert_eq!(header.scan_mode, BluetoothScanMode::Passive as u32);
        assert_eq!(header.primary_phy, BluetoothPhy::Le2M as u32);
        assert_eq!(header.secondary_phy, BluetoothPhy::LeCoded as u32);
        assert_eq!(header.keep_repeated_devices, 1);
        assert_eq!(
            decode_payload_slice(&bytes, header.name_offset, header.name_len, "name", "test")
                .unwrap(),
            b"Destack Peripheral"
        );
        assert_eq!(
            decode_payload_slice(
                &bytes,
                header.name_prefix_offset,
                header.name_prefix_len,
                "namePrefix",
                "test",
            )
            .unwrap(),
            b"Destack"
        );
        assert_eq!(
            decode_string_list(
                decode_payload_slice(
                    &bytes,
                    header.service_uuids_offset,
                    header.service_uuids_len,
                    "serviceUuids",
                    "test",
                )
                .unwrap(),
                "test",
            )
            .unwrap(),
            vec![String::from("12345678-1234-1234-1234-1234567890ab")]
        );

        let manufacturer_data = encode_manufacturer_filters(&filter.unwrap().manufacturer_data);
        assert_eq!(
            decode_payload_slice(
                &bytes,
                header.manufacturer_data_offset,
                header.manufacturer_data_len,
                "manufacturerData",
                "test",
            )
            .unwrap(),
            manufacturer_data.as_slice()
        );
    }

    /// Encode one bridged Android scan filter into the expected host bit layout.
    #[test]
    fn test_android_filter_flags_encodes_scan_mode_phys_and_duplicate_policy() {
        let filter = Some(scan_filter());

        let flags = android_filter_flags(&filter);

        assert_eq!(
            flags,
            ((BluetoothScanMode::Passive as u32) << 8)
                | ((BluetoothPhy::Le2M as u32) << 12)
                | ((BluetoothPhy::LeCoded as u32) << 16)
                | (1 << 20)
        );
    }

    /// Decode one Android device descriptor with advertisement payloads.
    #[test]
    fn test_decode_device_descriptor_decodes_advertisement_payloads() {
        let mut bytes = Vec::new();
        let (id_offset, id_len) = append_payload_string(&mut bytes, "android.bluetooth.device");
        let (adapter_id_offset, adapter_id_len) =
            append_payload_string(&mut bytes, "android.bluetooth.adapter");
        let (name_offset, name_len) = append_payload_string(&mut bytes, "Destack Peripheral");
        let (local_name_offset, local_name_len) =
            append_payload_string(&mut bytes, "Destack Local");
        let (address_offset, address_len) = append_payload_string(&mut bytes, "01:23:45:67:89:AB");
        let service_uuids = encode_string_list(&[String::from("180d")]);
        let (service_uuids_offset, service_uuids_len) = append_payload(&mut bytes, &service_uuids);
        let manufacturer_data = encode_manufacturer_data(&[(0x1337, &[0xaa, 0xbb][..])]);
        let (manufacturer_data_offset, manufacturer_data_len) =
            append_payload(&mut bytes, &manufacturer_data);
        let service_data = encode_service_data(&[("180d", &[0xcc, 0xdd])]);
        let (service_data_offset, service_data_len) = append_payload(&mut bytes, &service_data);
        let header = AndroidHostBluetoothDeviceDescriptorHeader {
            id_offset,
            id_len,
            adapter_id_offset,
            adapter_id_len,
            name_offset,
            name_len,
            local_name_offset,
            local_name_len,
            manufacturer_offset: 0,
            manufacturer_len: 0,
            model_offset: 0,
            model_len: 0,
            address_offset,
            address_len,
            service_uuids_offset,
            service_uuids_len,
            manufacturer_data_offset,
            manufacturer_data_len,
            service_data_offset,
            service_data_len,
            transport: 2,
            rssi_dbm: -48,
            pair_state: 4,
            phy_flags: 0,
            is_connected: 1,
        };

        let descriptor = decode_device_descriptor(&header, &bytes, "test").unwrap();

        assert_eq!(descriptor.name.as_deref(), Some("Destack Peripheral"));
        assert_eq!(
            descriptor.advertisement.local_name.as_deref(),
            Some("Destack Local")
        );
        assert_eq!(
            descriptor.advertisement.service_uuids,
            vec![String::from("180d")]
        );
        assert_eq!(descriptor.advertisement.manufacturer_data.len(), 1);
        assert_eq!(
            descriptor.advertisement.manufacturer_data[0].company_id,
            0x1337
        );
        assert_eq!(
            descriptor.advertisement.manufacturer_data[0].data,
            vec![0xaa, 0xbb]
        );
        assert_eq!(descriptor.advertisement.service_data.len(), 1);
        assert_eq!(
            descriptor.advertisement.service_data[0].service_uuid,
            String::from("180d")
        );
        assert_eq!(
            descriptor.advertisement.service_data[0].data,
            vec![0xcc, 0xdd]
        );
    }
}
