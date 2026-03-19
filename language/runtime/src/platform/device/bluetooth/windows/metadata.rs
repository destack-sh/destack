use super::core::*;

/// One Bluetooth LE advertisement type code for service data with one 16-bit UUID.
const SERVICE_DATA_16BIT_UUIDS_TYPE: u8 = 0x16;

/// One Bluetooth LE advertisement type code for service data with one 32-bit UUID.
const SERVICE_DATA_32BIT_UUIDS_TYPE: u8 = 0x20;

/// One Bluetooth LE advertisement type code for service data with one 128-bit UUID.
const SERVICE_DATA_128BIT_UUIDS_TYPE: u8 = 0x21;

/// One Bluetooth LE advertisement type code for transmit power.
const TX_POWER_LEVEL_TYPE: u8 = 0x0a;

/// Build one runtime error for one WinRT bluetooth failure.
pub(super) fn windows_bluetooth_error(
    operation: &'static str,
    action: &str,
    error: &WinError,
) -> Box<RuntimeError> {
    core_platform::winrt_io_error(operation, action, error)
}

/// Convert one LE address into the public colon-delimited form.
pub(super) fn bluetooth_address_string(address: u64) -> String {
    let bytes = [
        ((address >> 40) & 0xff) as u8,
        ((address >> 32) & 0xff) as u8,
        ((address >> 24) & 0xff) as u8,
        ((address >> 16) & 0xff) as u8,
        ((address >> 8) & 0xff) as u8,
        (address & 0xff) as u8,
    ];

    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
    )
}

/// Return one stable suffix for one WinRT address type.
pub(super) fn bluetooth_address_type_suffix(address_type: BluetoothAddressType) -> &'static str {
    match address_type {
        BluetoothAddressType::Random => "random",
        BluetoothAddressType::Public => "public",
        _ => "unspecified",
    }
}

/// Build one stable device identifier from one address tuple.
pub(super) fn stable_device_id(address: u64, address_type: BluetoothAddressType) -> String {
    format!(
        "{WINDOWS_BLUETOOTH_DEVICE_ID_PREFIX}:{address:012x}:{}",
        bluetooth_address_type_suffix(address_type),
    )
}

/// Parse one stable device identifier into one address tuple.
pub(super) fn parse_stable_device_id(
    value: &str,
    operation: &'static str,
) -> DiagnosticResult<(u64, BluetoothAddressType)> {
    let Some(rest) = value.strip_prefix(&format!("{WINDOWS_BLUETOOTH_DEVICE_ID_PREFIX}:")) else {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "deviceId",
            "bluetooth device identifier is not one WinRT LE id",
        ))
        .boxed());
    };

    let mut parts = rest.split(':');
    let address = parts.next().ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "deviceId",
            "bluetooth device identifier is missing the address payload",
        ))
        .boxed()
    })?;
    let address_type = parts.next().ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "deviceId",
            "bluetooth device identifier is missing the address-type payload",
        ))
        .boxed()
    })?;

    let address = u64::from_str_radix(address, 16).map_err(|error| {
        core_platform::io_operation_error(
            operation,
            None,
            format!("invalid bluetooth address: {error}"),
        )
    })?;

    let address_type = match address_type {
        "public" => BluetoothAddressType::Public,
        "random" => BluetoothAddressType::Random,
        "unspecified" => BluetoothAddressType::Unspecified,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "deviceId",
                "bluetooth device identifier carries one unknown address type",
            ))
            .boxed());
        }
    };

    Ok((address, address_type))
}

/// Build one stable service identifier from one attribute handle.
pub(super) fn stable_service_id(attribute_handle: u16) -> String {
    format!("{WINDOWS_BLUETOOTH_SERVICE_ID_PREFIX}:{attribute_handle:04x}")
}

/// Build one stable characteristic identifier from one attribute handle.
pub(super) fn stable_characteristic_id(attribute_handle: u16) -> String {
    format!("{WINDOWS_BLUETOOTH_CHARACTERISTIC_ID_PREFIX}:{attribute_handle:04x}")
}

/// Build one stable descriptor identifier from one attribute handle.
pub(super) fn stable_descriptor_id(attribute_handle: u16) -> String {
    format!("{WINDOWS_BLUETOOTH_DESCRIPTOR_ID_PREFIX}:{attribute_handle:04x}")
}

/// Convert one GATT communication status into one runtime error when needed.
pub(super) fn ensure_gatt_status(
    operation: &'static str,
    action: &str,
    status: GattCommunicationStatus,
    protocol_error: Option<u8>,
) -> DiagnosticResult<()> {
    if status == GattCommunicationStatus::Success {
        return Ok(());
    }

    let detail = match protocol_error {
        Some(code) => format!("{action} failed with protocol error 0x{code:02x}"),
        None => format!("{action} failed with status {}", status.0),
    };

    Err(core_platform::io_operation_error(operation, None, detail))
}

/// Decode one optional protocol error reference into one byte.
pub(super) fn protocol_error_code(value: windows::Foundation::IReference<u8>) -> Option<u8> {
    value.Value().ok()
}

/// Return one 16-bit Bluetooth UUID in canonical string form.
fn bluetooth_uuid16_string(value: u16) -> String {
    format!("0000{value:04x}-0000-1000-8000-00805f9b34fb")
}

/// Return one 32-bit Bluetooth UUID in canonical string form.
fn bluetooth_uuid32_string(value: u32) -> String {
    format!("{value:08x}-0000-1000-8000-00805f9b34fb")
}

/// Return one 128-bit Bluetooth UUID in canonical string form.
fn bluetooth_uuid128_string(bytes: &[u8; 16]) -> String {
    let reordered = [
        bytes[15], bytes[14], bytes[13], bytes[12], bytes[11], bytes[10], bytes[9], bytes[8],
        bytes[7], bytes[6], bytes[5], bytes[4], bytes[3], bytes[2], bytes[1], bytes[0],
    ];

    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        reordered[0],
        reordered[1],
        reordered[2],
        reordered[3],
        reordered[4],
        reordered[5],
        reordered[6],
        reordered[7],
        reordered[8],
        reordered[9],
        reordered[10],
        reordered[11],
        reordered[12],
        reordered[13],
        reordered[14],
        reordered[15],
    )
}

/// Decode one service-data advertisement section into the public value type.
fn advertisement_service_data(
    section_type: u8,
    bytes: &[u8],
) -> Option<BluetoothAdvertisementServiceDataValue> {
    match section_type {
        // 16-bit service data: little-endian UUID followed by service bytes
        SERVICE_DATA_16BIT_UUIDS_TYPE if bytes.len() >= 2 => {
            let service_uuid = bluetooth_uuid16_string(u16::from_le_bytes([bytes[0], bytes[1]]));

            Some(BluetoothAdvertisementServiceDataValue {
                service_uuid,
                data: bytes[2..].to_vec(),
            })
        }

        // 32-bit service data: little-endian UUID followed by service bytes
        SERVICE_DATA_32BIT_UUIDS_TYPE if bytes.len() >= 4 => {
            let service_uuid = bluetooth_uuid32_string(u32::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3],
            ]));

            Some(BluetoothAdvertisementServiceDataValue {
                service_uuid,
                data: bytes[4..].to_vec(),
            })
        }

        // 128-bit service data: little-endian UUID followed by service bytes
        SERVICE_DATA_128BIT_UUIDS_TYPE if bytes.len() >= 16 => {
            let service_uuid = bluetooth_uuid128_string(bytes[..16].try_into().ok()?);

            Some(BluetoothAdvertisementServiceDataValue {
                service_uuid,
                data: bytes[16..].to_vec(),
            })
        }

        _ => None,
    }
}

/// Convert one WinRT buffer into owned bytes.
pub(super) fn buffer_to_vec(
    buffer: &IBuffer,
    operation: &'static str,
    action: &str,
) -> DiagnosticResult<Vec<u8>> {
    let reader = DataReader::FromBuffer(buffer)
        .map_err(|error| windows_bluetooth_error(operation, action, &error))?;
    let mut bytes = vec![0u8; buffer.Length().unwrap_or(0) as usize];
    reader
        .ReadBytes(&mut bytes)
        .map_err(|error| windows_bluetooth_error(operation, action, &error))?;

    Ok(bytes)
}

/// Convert owned bytes into one WinRT buffer.
pub(super) fn vec_to_buffer(
    bytes: &[u8],
    operation: &'static str,
    action: &str,
) -> DiagnosticResult<IBuffer> {
    let writer =
        DataWriter::new().map_err(|error| windows_bluetooth_error(operation, action, &error))?;
    writer
        .WriteBytes(bytes)
        .map_err(|error| windows_bluetooth_error(operation, action, &error))?;

    writer
        .DetachBuffer()
        .map_err(|error| windows_bluetooth_error(operation, action, &error))
}

/// Map one WinRT characteristic-property bitset into the public descriptor type.
pub(super) fn characteristic_properties(
    properties: windows::Devices::Bluetooth::GenericAttributeProfile::GattCharacteristicProperties,
) -> BluetoothGattCharacteristicProperties {
    let flags = properties.0;

    BluetoothGattCharacteristicProperties {
        broadcast: flags
            & windows::Devices::Bluetooth::GenericAttributeProfile::GattCharacteristicProperties::Broadcast.0
            != 0,
        read: flags
            & windows::Devices::Bluetooth::GenericAttributeProfile::GattCharacteristicProperties::Read.0
            != 0,
        write_without_response: flags
            & windows::Devices::Bluetooth::GenericAttributeProfile::GattCharacteristicProperties::WriteWithoutResponse.0
            != 0,
        write: flags
            & windows::Devices::Bluetooth::GenericAttributeProfile::GattCharacteristicProperties::Write.0
            != 0,
        notify: flags
            & windows::Devices::Bluetooth::GenericAttributeProfile::GattCharacteristicProperties::Notify.0
            != 0,
        indicate: flags
            & windows::Devices::Bluetooth::GenericAttributeProfile::GattCharacteristicProperties::Indicate.0
            != 0,
        authenticated_signed_writes: flags
            & windows::Devices::Bluetooth::GenericAttributeProfile::GattCharacteristicProperties::AuthenticatedSignedWrites.0
            != 0,
        reliable_write: flags
            & windows::Devices::Bluetooth::GenericAttributeProfile::GattCharacteristicProperties::ReliableWrites.0
            != 0,
        writable_auxiliaries: flags
            & windows::Devices::Bluetooth::GenericAttributeProfile::GattCharacteristicProperties::WritableAuxiliaries.0
            != 0,
    }
}

/// Build one adapter descriptor from one WinRT adapter object.
pub(super) fn adapter_descriptor(
    adapter: &BluetoothAdapter,
    operation: &'static str,
) -> DiagnosticResult<BluetoothAdapterDescriptorValue> {
    let id = adapter.DeviceId().map_err(|error| {
        windows_bluetooth_error(operation, "BluetoothAdapter::DeviceId", &error)
    })?;
    let radio = adapter
        .GetRadioAsync()
        .map_err(|error| {
            windows_bluetooth_error(operation, "BluetoothAdapter::GetRadioAsync", &error)
        })?
        .get()
        .map_err(|error| windows_bluetooth_error(operation, "IAsyncOperation::get", &error))?;
    let state = radio
        .State()
        .map_err(|error| windows_bluetooth_error(operation, "Radio::State", &error))?;

    Ok(BluetoothAdapterDescriptorValue {
        id: core_platform::hstring_to_string(&id),
        name: String::from("Windows Bluetooth"),
        powered: state == RadioState::On,
        discovering: None,
        extended_advertising: Some(adapter.IsExtendedAdvertisingSupported().map_err(|error| {
            windows_bluetooth_error(
                operation,
                "BluetoothAdapter::IsExtendedAdvertisingSupported",
                &error,
            )
        })?),
    })
}

/// Enumerate WinRT Bluetooth adapters.
pub(super) fn enumerate_adapters(
    operation: &'static str,
) -> DiagnosticResult<Vec<(BluetoothAdapterDescriptorValue, BluetoothAdapter)>> {
    let selector = BluetoothAdapter::GetDeviceSelector().map_err(|error| {
        windows_bluetooth_error(operation, "BluetoothAdapter::GetDeviceSelector", &error)
    })?;
    let devices = DeviceInformation::FindAllAsyncAqsFilter(&selector)
        .map_err(|error| {
            windows_bluetooth_error(
                operation,
                "DeviceInformation::FindAllAsyncAqsFilter",
                &error,
            )
        })?
        .get()
        .map_err(|error| windows_bluetooth_error(operation, "IAsyncOperation::get", &error))?;
    let mut adapters = Vec::new();

    for device in &devices {
        let device_id = device
            .Id()
            .map_err(|error| windows_bluetooth_error(operation, "DeviceInformation::Id", &error))?;
        let adapter = BluetoothAdapter::FromIdAsync(&device_id)
            .map_err(|error| {
                windows_bluetooth_error(operation, "BluetoothAdapter::FromIdAsync", &error)
            })?
            .get()
            .map_err(|error| windows_bluetooth_error(operation, "IAsyncOperation::get", &error))?;
        let descriptor = adapter_descriptor(&adapter, operation)?;
        adapters.push((descriptor, adapter));
    }

    Ok(adapters)
}

/// Decode one advertisement payload into the public value type.
pub(super) fn advertisement_data(
    args: &BluetoothLEAdvertisementReceivedEventArgs,
    operation: &'static str,
) -> DiagnosticResult<BluetoothAdvertisementDataValue> {
    let advertisement = args.Advertisement().map_err(|error| {
        windows_bluetooth_error(
            operation,
            "BluetoothLEAdvertisementReceivedEventArgs::Advertisement",
            &error,
        )
    })?;
    let local_name = advertisement
        .LocalName()
        .ok()
        .map(|value| value.to_string())
        .filter(|value| !value.is_empty());

    let mut service_uuids = Vec::new();
    if let Ok(values) = advertisement.ServiceUuids() {
        for value in &values {
            service_uuids.push(core_platform::guid_to_string(value));
        }
    }

    let mut manufacturer_data = Vec::new();
    if let Ok(values) = advertisement.ManufacturerData() {
        for value in &values {
            let company_id = value.CompanyId().unwrap_or_default();
            let bytes = value
                .Data()
                .ok()
                .and_then(|data| {
                    buffer_to_vec(&data, operation, "BluetoothLEManufacturerData::Data").ok()
                })
                .unwrap_or_default();

            manufacturer_data.push(BluetoothAdvertisementManufacturerDataValue {
                company_id,
                data: bytes,
            });
        }
    }

    let mut tx_power = args
        .TransmitPowerLevelInDBm()
        .ok()
        .and_then(|value| value.Value().ok());
    let mut service_data = Vec::<BluetoothAdvertisementServiceDataValue>::new();

    if let Ok(data_sections) = advertisement.DataSections() {
        for section in &data_sections {
            let section_type = section.DataType().map_err(|error| {
                windows_bluetooth_error(
                    operation,
                    "BluetoothLEAdvertisementDataSection::DataType",
                    &error,
                )
            })?;
            let bytes = section
                .Data()
                .map_err(|error| {
                    windows_bluetooth_error(
                        operation,
                        "BluetoothLEAdvertisementDataSection::Data",
                        &error,
                    )
                })
                .and_then(|data| {
                    buffer_to_vec(
                        &data,
                        operation,
                        "BluetoothLEAdvertisementDataSection::Data",
                    )
                })?;

            // transmit power fallback
            if tx_power.is_none() && section_type == TX_POWER_LEVEL_TYPE && !bytes.is_empty() {
                tx_power = Some(i16::from(i8::from_le_bytes([bytes[0]])));
            }

            // service data rows
            if let Some(value) = advertisement_service_data(section_type, &bytes) {
                service_data.push(value);
            }
        }
    }

    Ok(BluetoothAdvertisementDataValue {
        local_name,
        tx_power,
        service_uuids,
        manufacturer_data,
        service_data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Decode one 16-bit service-data section into one public advertisement row.
    #[test]
    fn test_advertisement_service_data_decodes_16bit_uuid() {
        let value =
            advertisement_service_data(SERVICE_DATA_16BIT_UUIDS_TYPE, &[0x0d, 0x18, 1, 2, 3])
                .unwrap();

        assert_eq!(value.service_uuid, "0000180d-0000-1000-8000-00805f9b34fb");
        assert_eq!(value.data, vec![1, 2, 3]);
    }

    /// Decode one little-endian 128-bit UUID into canonical string form.
    #[test]
    fn test_bluetooth_uuid128_string_reorders_advertisement_bytes() {
        let bytes = [
            0xfb, 0x34, 0x9b, 0x5f, 0x80, 0x00, 0x00, 0x80, 0x00, 0x10, 0x00, 0x00, 0x0d, 0x18,
            0x00, 0x00,
        ];

        let value = bluetooth_uuid128_string(&bytes);

        assert_eq!(value, "0000180d-0000-1000-8000-00805f9b34fb");
    }
}

/// Decode one received advertisement row into one device descriptor.
pub(super) fn received_device_descriptor(
    args: &BluetoothLEAdvertisementReceivedEventArgs,
    operation: &'static str,
) -> DiagnosticResult<BluetoothDeviceDescriptorValue> {
    let address = args.BluetoothAddress().map_err(|error| {
        windows_bluetooth_error(
            operation,
            "BluetoothLEAdvertisementReceivedEventArgs::BluetoothAddress",
            &error,
        )
    })?;
    let address_type = args.BluetoothAddressType().map_err(|error| {
        windows_bluetooth_error(
            operation,
            "BluetoothLEAdvertisementReceivedEventArgs::BluetoothAddressType",
            &error,
        )
    })?;
    let advertisement = advertisement_data(args, operation)?;
    let local_name = advertisement.local_name.clone();

    Ok(BluetoothDeviceDescriptorValue {
        id: stable_device_id(address, address_type),
        address: Some(bluetooth_address_string(address)),
        name: local_name,
        rssi: Some(i32::from(args.RawSignalStrengthInDBm().map_err(
            |error| {
                windows_bluetooth_error(
                    operation,
                    "BluetoothLEAdvertisementReceivedEventArgs::RawSignalStrengthInDBm",
                    &error,
                )
            },
        )?)),
        pair_state: None,
        connected: false,
        connectable: Some(args.IsConnectable().map_err(|error| {
            windows_bluetooth_error(
                operation,
                "BluetoothLEAdvertisementReceivedEventArgs::IsConnectable",
                &error,
            )
        })?),
        transport: Some(BluetoothLeTransport::LowEnergy),
        advertisement,
    })
}

/// Build one descriptor snapshot from one opened LE device.
pub(super) fn opened_device_descriptor(
    device: &BluetoothLEDevice,
    operation: &'static str,
) -> DiagnosticResult<BluetoothDeviceDescriptorValue> {
    let device_information = device.DeviceInformation().map_err(|error| {
        windows_bluetooth_error(operation, "BluetoothLEDevice::DeviceInformation", &error)
    })?;
    let pairing = device_information.Pairing().map_err(|error| {
        windows_bluetooth_error(operation, "DeviceInformation::Pairing", &error)
    })?;
    let is_paired = pairing.IsPaired().map_err(|error| {
        windows_bluetooth_error(operation, "DeviceInformationPairing::IsPaired", &error)
    })?;
    let address = device.BluetoothAddress().map_err(|error| {
        windows_bluetooth_error(operation, "BluetoothLEDevice::BluetoothAddress", &error)
    })?;
    let address_type = device.BluetoothAddressType().map_err(|error| {
        windows_bluetooth_error(operation, "BluetoothLEDevice::BluetoothAddressType", &error)
    })?;
    let name = device
        .Name()
        .map_err(|error| windows_bluetooth_error(operation, "BluetoothLEDevice::Name", &error))?
        .to_string();

    Ok(BluetoothDeviceDescriptorValue {
        id: stable_device_id(address, address_type),
        address: Some(bluetooth_address_string(address)),
        name: if name.is_empty() { None } else { Some(name) },
        rssi: None,
        pair_state: Some(if is_paired {
            BluetoothPairState::Paired
        } else {
            BluetoothPairState::Unpaired
        }),
        connected: device.ConnectionStatus().map_err(|error| {
            windows_bluetooth_error(operation, "BluetoothLEDevice::ConnectionStatus", &error)
        })? == BluetoothConnectionStatus::Connected,
        connectable: None,
        transport: Some(BluetoothLeTransport::LowEnergy),
        advertisement: BluetoothAdvertisementDataValue {
            local_name: None,
            tx_power: None,
            service_uuids: Vec::new(),
            manufacturer_data: Vec::new(),
            service_data: Vec::new(),
        },
    })
}

/// Refresh one GATT graph from one opened LE device.
pub(super) fn refresh_gatt_cache(
    device: &BluetoothLEDevice,
    operation: &'static str,
) -> DiagnosticResult<WindowsBluetoothGattCache> {
    let services_result = device
        .GetGattServicesWithCacheModeAsync(BluetoothCacheMode::Uncached)
        .map_err(|error| {
            windows_bluetooth_error(
                operation,
                "BluetoothLEDevice::GetGattServicesWithCacheModeAsync",
                &error,
            )
        })?
        .get()
        .map_err(|error| windows_bluetooth_error(operation, "IAsyncOperation::get", &error))?;
    ensure_gatt_status(
        operation,
        "BluetoothLEDevice::GetGattServicesWithCacheModeAsync",
        services_result.Status().map_err(|error| {
            windows_bluetooth_error(operation, "GattDeviceServicesResult::Status", &error)
        })?,
        services_result
            .ProtocolError()
            .ok()
            .and_then(protocol_error_code),
    )?;

    let mut cache = WindowsBluetoothGattCache::default();
    let services = services_result.Services().map_err(|error| {
        windows_bluetooth_error(operation, "GattDeviceServicesResult::Services", &error)
    })?;

    for service in &services {
        let service_handle = service.AttributeHandle().map_err(|error| {
            windows_bluetooth_error(operation, "GattDeviceService::AttributeHandle", &error)
        })?;
        let service_id = stable_service_id(service_handle);
        let service_uuid = service
            .Uuid()
            .map(core_platform::guid_to_string)
            .map_err(|error| {
                windows_bluetooth_error(operation, "GattDeviceService::Uuid", &error)
            })?;
        cache.services.insert(
            service_id.clone(),
            BluetoothGattServiceValue {
                id: service_id.clone(),
                uuid: service_uuid,
                primary: true,
                included_service_ids: Vec::new(),
            },
        );
        cache
            .service_objects
            .insert(service_id.clone(), service.clone());

        let characteristics_result = service
            .GetCharacteristicsWithCacheModeAsync(BluetoothCacheMode::Uncached)
            .map_err(|error| {
                windows_bluetooth_error(
                    operation,
                    "GattDeviceService::GetCharacteristicsWithCacheModeAsync",
                    &error,
                )
            })?
            .get()
            .map_err(|error| windows_bluetooth_error(operation, "IAsyncOperation::get", &error))?;
        ensure_gatt_status(
            operation,
            "GattDeviceService::GetCharacteristicsWithCacheModeAsync",
            characteristics_result.Status().map_err(|error| {
                windows_bluetooth_error(operation, "GattCharacteristicsResult::Status", &error)
            })?,
            characteristics_result
                .ProtocolError()
                .ok()
                .and_then(protocol_error_code),
        )?;

        let characteristics = characteristics_result.Characteristics().map_err(|error| {
            windows_bluetooth_error(
                operation,
                "GattCharacteristicsResult::Characteristics",
                &error,
            )
        })?;

        for characteristic in &characteristics {
            let characteristic_handle = characteristic.AttributeHandle().map_err(|error| {
                windows_bluetooth_error(operation, "GattCharacteristic::AttributeHandle", &error)
            })?;
            let characteristic_id = stable_characteristic_id(characteristic_handle);
            let characteristic_uuid = characteristic
                .Uuid()
                .map(core_platform::guid_to_string)
                .map_err(|error| {
                    windows_bluetooth_error(operation, "GattCharacteristic::Uuid", &error)
                })?;
            let characteristic_properties = characteristic
                .CharacteristicProperties()
                .map(characteristic_properties)
                .map_err(|error| {
                    windows_bluetooth_error(
                        operation,
                        "GattCharacteristic::CharacteristicProperties",
                        &error,
                    )
                })?;
            cache.characteristics.insert(
                characteristic_id.clone(),
                BluetoothGattCharacteristicValue {
                    id: characteristic_id.clone(),
                    service_id: service_id.clone(),
                    uuid: characteristic_uuid,
                    properties: characteristic_properties,
                },
            );
            cache
                .characteristic_objects
                .insert(characteristic_id.clone(), characteristic.clone());

            let descriptors_result = characteristic
                .GetDescriptorsWithCacheModeAsync(BluetoothCacheMode::Uncached)
                .map_err(|error| {
                    windows_bluetooth_error(
                        operation,
                        "GattCharacteristic::GetDescriptorsWithCacheModeAsync",
                        &error,
                    )
                })?
                .get()
                .map_err(|error| {
                    windows_bluetooth_error(operation, "IAsyncOperation::get", &error)
                })?;
            ensure_gatt_status(
                operation,
                "GattCharacteristic::GetDescriptorsWithCacheModeAsync",
                descriptors_result.Status().map_err(|error| {
                    windows_bluetooth_error(operation, "GattDescriptorsResult::Status", &error)
                })?,
                descriptors_result
                    .ProtocolError()
                    .ok()
                    .and_then(protocol_error_code),
            )?;

            let descriptors = descriptors_result.Descriptors().map_err(|error| {
                windows_bluetooth_error(operation, "GattDescriptorsResult::Descriptors", &error)
            })?;

            for descriptor in &descriptors {
                let descriptor_handle = descriptor.AttributeHandle().map_err(|error| {
                    windows_bluetooth_error(operation, "GattDescriptor::AttributeHandle", &error)
                })?;
                let descriptor_id = stable_descriptor_id(descriptor_handle);
                let descriptor_uuid = descriptor
                    .Uuid()
                    .map(core_platform::guid_to_string)
                    .map_err(|error| {
                        windows_bluetooth_error(operation, "GattDescriptor::Uuid", &error)
                    })?;

                cache.descriptors.insert(
                    descriptor_id.clone(),
                    BluetoothGattDescriptorValue {
                        id: descriptor_id.clone(),
                        service_id: service_id.clone(),
                        characteristic_id: characteristic_id.clone(),
                        uuid: descriptor_uuid,
                    },
                );
                cache
                    .descriptor_objects
                    .insert(descriptor_id, descriptor.clone());
            }
        }
    }

    if let Some(service) = cache.service_objects.values().next() {
        if let Ok(session) = service.Session() {
            cache.mtu = session.MaxPduSize().unwrap_or_default();
        }
    }

    Ok(cache)
}

/// Convert one write mode into one WinRT write option.
pub(super) fn gatt_write_option(mode: BluetoothGattWriteMode) -> GattWriteOption {
    match mode {
        BluetoothGattWriteMode::WithResponse => GattWriteOption::WriteWithResponse,
        BluetoothGattWriteMode::WithoutResponse => GattWriteOption::WriteWithoutResponse,
    }
}
