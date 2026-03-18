use super::core::*;
use super::state::*;
use std::collections::BTreeMap;
use std::mem::size_of;

/// One owned SetupAPI device-info set.
struct WindowsDeviceInfoSet(HDEVINFO);

impl Drop for WindowsDeviceInfoSet {
    /// Destroy the underlying device-info set.
    fn drop(&mut self) {
        unsafe {
            SetupDiDestroyDeviceInfoList(self.0);
        }
    }
}

/// One owned registry key handle.
struct WindowsRegistryKey(HKEY);

impl Drop for WindowsRegistryKey {
    /// Close the underlying registry key.
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.0);
        }
    }
}

/// Decode one serial identifier into one COM port number.
pub(super) fn decode_serial_port_number(id: &str) -> RuntimeResult<u32> {
    let normalized = id.trim().to_ascii_uppercase();
    let port_suffix = if let Some(port_suffix) = normalized.strip_prefix(r"\\.\COM") {
        port_suffix
    } else if let Some(port_suffix) = normalized.strip_prefix("COM") {
        port_suffix
    } else {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "serial identifier must name one COM port",
        ))
        .boxed());
    };

    let port_number = port_suffix.parse::<u32>().map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "serial identifier must contain one decimal COM port number",
        ))
        .boxed()
    })?;
    if port_number == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "serial identifier must reference one positive COM port number",
        ))
        .boxed());
    }

    Ok(port_number)
}

/// Build one COM port name from one port number.
pub(super) fn serial_port_name(port_number: u32) -> String {
    format!("COM{port_number}")
}

/// Build one device-namespace path from one COM port name.
pub(super) fn serial_port_path(port_name: &str) -> String {
    format!(r"\\.\{port_name}")
}

/// Build one SetupAPI enumeration error.
fn setup_api_error(
    operation: &'static str,
    syscall: &'static str,
    message: &str,
) -> Box<RuntimeError> {
    serial_io_error(operation, syscall, message)
}

/// Return whether one SetupAPI or registry error means the property is absent.
fn is_optional_property_code(code: u32) -> bool {
    code == ERROR_FILE_NOT_FOUND
        || code == ERROR_NOT_FOUND
        || code == ERROR_INVALID_DATA
        || code == ERROR_PATH_NOT_FOUND
}

/// Read one nul-terminated utf16 string from one raw pointer.
unsafe fn wide_ptr_string(pointer: *const u16) -> String {
    let mut length = 0usize;

    // measure the string until the trailing nul
    while unsafe { *pointer.add(length) } != 0 {
        length += 1;
    }

    let units = unsafe { std::slice::from_raw_parts(pointer, length) };

    String::from_utf16_lossy(units)
}

/// Read one optional wide string from one SetupAPI registry property.
fn setup_registry_property_string(
    device_info_set: HDEVINFO,
    device_info: &SP_DEVINFO_DATA,
    property: u32,
    operation: &'static str,
) -> RuntimeResult<Option<String>> {
    let mut buffer = vec![0u16; INITIAL_SETUP_PROPERTY_CAPACITY];

    loop {
        let mut property_type = 0u32;
        let mut required_bytes = 0u32;

        // query the property into a reusable utf16 buffer
        let status = unsafe {
            SetupDiGetDeviceRegistryPropertyW(
                device_info_set,
                device_info,
                property,
                &mut property_type,
                buffer.as_mut_ptr().cast::<u8>(),
                (buffer.len() * size_of::<u16>()) as u32,
                &mut required_bytes,
            )
        };
        if status != 0 {
            let length = buffer
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(buffer.len());
            let value = String::from_utf16_lossy(&buffer[..length]);

            return Ok((!value.is_empty()).then_some(value));
        }

        let code = core_platform::last_error_code() as u32;

        // return absence quietly for optional metadata
        if is_optional_property_code(code) {
            return Ok(None);
        }

        // grow the buffer when SetupAPI reports truncation
        if code == ERROR_INSUFFICIENT_BUFFER && required_bytes != 0 {
            let required_units = required_bytes.div_ceil(size_of::<u16>() as u32) as usize;
            let next_capacity = usize::max(required_units, buffer.len().saturating_mul(2));
            if next_capacity <= buffer.len() {
                return Err(RuntimeError::from(PlatformError::io(
                    "SetupDiGetDeviceRegistryPropertyW reported one non-growing buffer requirement"
                        .to_string(),
                ))
                .boxed());
            }

            buffer.resize(next_capacity, 0);
            continue;
        }

        return Err(setup_api_error(
            operation,
            "SetupDiGetDeviceRegistryPropertyW",
            "failed to read serial device registry property",
        ));
    }
}

/// Read one optional wide string from one SetupAPI device property.
fn setup_device_property_string(
    device_info_set: HDEVINFO,
    device_info: &SP_DEVINFO_DATA,
    property: &windows_sys::Win32::Devices::Properties::DEVPROPKEY,
    operation: &'static str,
) -> RuntimeResult<Option<String>> {
    let mut buffer = vec![0u16; INITIAL_SETUP_PROPERTY_CAPACITY];

    loop {
        let mut property_type: DEVPROPTYPE = 0;
        let mut required_bytes = 0u32;

        // query the property into a reusable utf16 buffer
        let status = unsafe {
            SetupDiGetDevicePropertyW(
                device_info_set,
                device_info,
                property,
                &mut property_type,
                buffer.as_mut_ptr().cast::<u8>(),
                (buffer.len() * size_of::<u16>()) as u32,
                &mut required_bytes,
                0,
            )
        };
        if status != 0 {
            let length = buffer
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(buffer.len());
            let value = String::from_utf16_lossy(&buffer[..length]);

            return Ok((!value.is_empty()).then_some(value));
        }

        let code = core_platform::last_error_code() as u32;

        // return absence quietly for optional metadata
        if is_optional_property_code(code) {
            return Ok(None);
        }

        // grow the buffer when SetupAPI reports truncation
        if code == ERROR_INSUFFICIENT_BUFFER && required_bytes != 0 {
            let required_units = required_bytes.div_ceil(size_of::<u16>() as u32) as usize;
            let next_capacity = usize::max(required_units, buffer.len().saturating_mul(2));
            if next_capacity <= buffer.len() {
                return Err(RuntimeError::from(PlatformError::io(
                    "SetupDiGetDevicePropertyW reported one non-growing buffer requirement"
                        .to_string(),
                ))
                .boxed());
            }

            buffer.resize(next_capacity, 0);
            continue;
        }

        return Err(setup_api_error(
            operation,
            "SetupDiGetDevicePropertyW",
            "failed to read serial device property",
        ));
    }
}

/// Read one optional string value from one registry key.
fn registry_value_string(
    key: HKEY,
    value_name: &str,
    operation: &'static str,
) -> RuntimeResult<Option<String>> {
    let value_name = core_platform::wide_from_str("valueName", value_name).map_err(|error| {
        RuntimeError::from(PlatformError::io(format!(
            "{operation}: failed to encode registry value name: {error}",
        )))
        .boxed()
    })?;
    let mut buffer = vec![0u16; INITIAL_SETUP_PROPERTY_CAPACITY];

    loop {
        let mut value_type = 0u32;
        let mut required_bytes = (buffer.len() * size_of::<u16>()) as u32;

        // query the registry value into a reusable utf16 buffer
        let status = unsafe {
            RegQueryValueExW(
                key,
                value_name.as_ptr(),
                std::ptr::null(),
                &mut value_type,
                buffer.as_mut_ptr().cast::<u8>(),
                &mut required_bytes,
            )
        };
        if status == ERROR_SUCCESS {
            if value_type != REG_SZ {
                return Ok(None);
            }

            let length = buffer
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(buffer.len());
            let value = String::from_utf16_lossy(&buffer[..length]);

            return Ok((!value.is_empty()).then_some(value));
        }

        // return absence quietly for optional metadata
        if is_optional_property_code(status) {
            return Ok(None);
        }

        // grow the buffer when the registry reports truncation
        if status == ERROR_MORE_DATA && required_bytes != 0 {
            let required_units = required_bytes.div_ceil(size_of::<u16>() as u32) as usize;
            let next_capacity = usize::max(required_units, buffer.len().saturating_mul(2));
            if next_capacity <= buffer.len() {
                return Err(RuntimeError::from(PlatformError::io(
                    "RegQueryValueExW reported one non-growing buffer requirement".to_string(),
                ))
                .boxed());
            }

            buffer.resize(next_capacity, 0);
            continue;
        }

        return Err(serial_io_error_with_code(
            operation,
            "RegQueryValueExW",
            status,
            "failed to read serial registry value",
        ));
    }
}

/// Read one serial port name from one device registry key.
fn registry_port_name(
    device_info_set: HDEVINFO,
    device_info: &SP_DEVINFO_DATA,
    operation: &'static str,
) -> RuntimeResult<Option<String>> {
    let key = unsafe {
        SetupDiOpenDevRegKey(
            device_info_set,
            device_info,
            DICS_FLAG_GLOBAL,
            0,
            DIREG_DEV,
            KEY_READ,
        )
    };
    if key == INVALID_HANDLE_VALUE as HKEY {
        let code = core_platform::last_error_code() as u32;
        if is_optional_property_code(code) {
            return Ok(None);
        }

        return Err(setup_api_error(
            operation,
            "SetupDiOpenDevRegKey",
            "failed to open serial device registry key",
        ));
    }

    let key = WindowsRegistryKey(key);

    registry_value_string(key.0, "PortName", operation)
}

/// Read one instance identifier for one SetupAPI device.
fn device_instance_id(
    device_info_set: HDEVINFO,
    device_info: &SP_DEVINFO_DATA,
    operation: &'static str,
) -> RuntimeResult<String> {
    let mut buffer = vec![0u16; INITIAL_INSTANCE_ID_CAPACITY];

    loop {
        let mut required_units = 0u32;

        // query the device instance identifier into a reusable utf16 buffer
        let status = unsafe {
            SetupDiGetDeviceInstanceIdW(
                device_info_set,
                device_info,
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                &mut required_units,
            )
        };
        if status != 0 {
            let length = buffer
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(buffer.len());

            return Ok(String::from_utf16_lossy(&buffer[..length]));
        }

        let code = core_platform::last_error_code() as u32;
        if code == ERROR_INSUFFICIENT_BUFFER && required_units != 0 {
            let next_capacity = usize::max(required_units as usize, buffer.len().saturating_mul(2));
            if next_capacity <= buffer.len() {
                return Err(RuntimeError::from(PlatformError::io(
                    "SetupDiGetDeviceInstanceIdW reported one non-growing buffer requirement"
                        .to_string(),
                ))
                .boxed());
            }

            buffer.resize(next_capacity, 0);
            continue;
        }

        return Err(setup_api_error(
            operation,
            "SetupDiGetDeviceInstanceIdW",
            "failed to read serial device instance identifier",
        ));
    }
}

/// Read one interface device path and paired devinfo record.
fn interface_detail(
    device_info_set: HDEVINFO,
    interface_data: &SP_DEVICE_INTERFACE_DATA,
    operation: &'static str,
) -> RuntimeResult<(Vec<u16>, SP_DEVINFO_DATA)> {
    let mut required_bytes = 0u32;

    // query the required detail buffer size first
    unsafe {
        SetupDiGetDeviceInterfaceDetailW(
            device_info_set,
            interface_data,
            std::ptr::null_mut(),
            0,
            &mut required_bytes,
            std::ptr::null_mut(),
        );
    }

    let code = core_platform::last_error_code() as u32;
    if code != ERROR_INSUFFICIENT_BUFFER || required_bytes == 0 {
        return Err(setup_api_error(
            operation,
            "SetupDiGetDeviceInterfaceDetailW",
            "failed to size serial interface detail buffer",
        ));
    }

    let mut detail_buffer = vec![0u8; required_bytes as usize];
    let detail = detail_buffer
        .as_mut_ptr()
        .cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
    let mut device_info = SP_DEVINFO_DATA {
        cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
        ClassGuid: unsafe { std::mem::zeroed() },
        DevInst: 0,
        Reserved: 0,
    };

    // populate the interface detail and paired device info
    unsafe {
        (*detail).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
    }

    let status = unsafe {
        SetupDiGetDeviceInterfaceDetailW(
            device_info_set,
            interface_data,
            detail,
            required_bytes,
            &mut required_bytes,
            &mut device_info,
        )
    };
    if status == 0 {
        return Err(setup_api_error(
            operation,
            "SetupDiGetDeviceInterfaceDetailW",
            "failed to read serial interface detail",
        ));
    }

    let path = unsafe { wide_ptr_string((*detail).DevicePath.as_ptr()) };

    Ok((path.encode_utf16().collect(), device_info))
}

/// Extract one COM port name from one friendly-name string.
fn port_name_from_friendly_name(value: &str) -> Option<String> {
    let start = value.rfind("(COM")?;
    let suffix = &value[start + 1..];
    let end = suffix.find(')')?;
    let port_name = &suffix[..end];

    decode_serial_port_number(port_name)
        .ok()
        .map(serial_port_name)
}

/// Parse one hexadecimal usb identifier fragment from one instance id.
fn usb_id_fragment(instance_id: &str, marker: &str) -> Option<u16> {
    let upper = instance_id.to_ascii_uppercase();
    let index = upper.find(marker)?;
    let digits = upper.get(index + marker.len()..index + marker.len() + 4)?;

    u16::from_str_radix(digits, 16).ok()
}

/// Extract one portable serial transport from one instance id and interface path.
pub(super) fn classify_serial_transport(
    instance_id: &str,
    interface_path: &str,
) -> SerialPortTransport {
    let instance_upper = instance_id.to_ascii_uppercase();
    let interface_upper = interface_path.to_ascii_uppercase();

    // bluetooth virtual serial devices surface through bthenum
    if instance_upper.contains("BTHENUM\\") || interface_upper.contains("BTHENUM") {
        return SerialPortTransport::Bluetooth;
    }

    // usb serial devices carry vid and pid fragments
    if instance_upper.contains("USB\\")
        && instance_upper.contains("VID_")
        && instance_upper.contains("PID_")
    {
        return SerialPortTransport::Usb;
    }

    SerialPortTransport::Native
}

/// Extract one optional usb serial number from one instance identifier.
fn usb_serial_number(instance_id: &str) -> Option<String> {
    let tail = instance_id.rsplit('\\').next()?;

    // interface-local instance ids are location strings, not device serials
    if tail.contains('&') {
        return None;
    }

    (!tail.is_empty()).then(|| tail.to_string())
}

/// Extract one optional bluetooth service class identifier from one instance identifier.
fn bluetooth_service_class_id(instance_id: &str) -> Option<String> {
    let start = instance_id.find('{')?;
    let end = instance_id[start..].find('}')?;
    let value = &instance_id[start + 1..start + end];

    (!value.is_empty()).then(|| value.to_string())
}

/// Build one serial descriptor snapshot from one SetupAPI interface entry.
fn descriptor_info_from_interface(
    device_info_set: HDEVINFO,
    interface_data: &SP_DEVICE_INTERFACE_DATA,
    operation: &'static str,
) -> RuntimeResult<WindowsSerialDescriptorInfo> {
    let (path_units, device_info) = interface_detail(device_info_set, interface_data, operation)?;
    let interface_path = String::from_utf16_lossy(&path_units);
    let instance_id = device_instance_id(device_info_set, &device_info, operation)?;

    // read the most stable direct identifiers first
    let port_name = registry_port_name(device_info_set, &device_info, operation)?
        .or_else(|| {
            setup_device_property_string(
                device_info_set,
                &device_info,
                &DEVPKEY_Device_FriendlyName,
                operation,
            )
            .ok()
            .flatten()
            .and_then(|value| port_name_from_friendly_name(&value))
        })
        .or_else(|| {
            setup_registry_property_string(
                device_info_set,
                &device_info,
                SPDRP_FRIENDLYNAME,
                operation,
            )
            .ok()
            .flatten()
            .and_then(|value| port_name_from_friendly_name(&value))
        })
        .ok_or_else(|| {
            RuntimeError::from(PlatformError::io_with(
                None,
                None,
                None,
                Some(operation.to_string()),
                Some(String::from("serialDescriptorSnapshot")),
                "failed to resolve COM port name from SetupAPI metadata".to_string(),
            ))
            .boxed()
        })?;

    // read the user-visible strings from the strongest available properties
    let friendly_name = setup_device_property_string(
        device_info_set,
        &device_info,
        &DEVPKEY_Device_FriendlyName,
        operation,
    )?
    .or(setup_registry_property_string(
        device_info_set,
        &device_info,
        SPDRP_FRIENDLYNAME,
        operation,
    )?);
    let manufacturer = setup_device_property_string(
        device_info_set,
        &device_info,
        &DEVPKEY_Device_Manufacturer,
        operation,
    )?
    .or(setup_registry_property_string(
        device_info_set,
        &device_info,
        SPDRP_MFG,
        operation,
    )?);
    let product = setup_device_property_string(
        device_info_set,
        &device_info,
        &DEVPKEY_Device_BusReportedDeviceDesc,
        operation,
    )?
    .or(setup_registry_property_string(
        device_info_set,
        &device_info,
        SPDRP_DEVICEDESC,
        operation,
    )?);

    // derive transport-specific identity from the instance path
    let transport = classify_serial_transport(&instance_id, &interface_path);
    let usb_vendor_id = (transport == SerialPortTransport::Usb)
        .then(|| usb_id_fragment(&instance_id, "VID_"))
        .flatten();
    let usb_product_id = (transport == SerialPortTransport::Usb)
        .then(|| usb_id_fragment(&instance_id, "PID_"))
        .flatten();
    let serial_number = (transport == SerialPortTransport::Usb)
        .then(|| usb_serial_number(&instance_id))
        .flatten();
    let bluetooth_service_class_id = (transport == SerialPortTransport::Bluetooth)
        .then(|| bluetooth_service_class_id(&instance_id))
        .flatten();

    Ok(WindowsSerialDescriptorInfo {
        id: port_name.clone(),
        transport,
        name: friendly_name.unwrap_or_else(|| port_name.clone()),
        manufacturer,
        product,
        serial_number,
        path_units,
        usb_vendor_id,
        usb_product_id,
        bluetooth_service_class_id,
    })
}

/// Build one SetupAPI device-info set for serial device interfaces.
fn serial_device_info_set(operation: &'static str) -> RuntimeResult<WindowsDeviceInfoSet> {
    let handle = unsafe {
        SetupDiGetClassDevsW(
            &GUID_DEVINTERFACE_COMPORT,
            std::ptr::null(),
            0,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )
    };
    if handle == INVALID_HANDLE_VALUE as HDEVINFO {
        return Err(setup_api_error(
            operation,
            "SetupDiGetClassDevsW",
            "failed to enumerate serial device interfaces",
        ));
    }

    Ok(WindowsDeviceInfoSet(handle))
}

/// Resolve one descriptor snapshot for one selected COM port name.
pub(super) fn descriptor_info_for_port_name(
    port_name: &str,
    operation: &'static str,
) -> RuntimeResult<WindowsSerialDescriptorInfo> {
    // prefer the live SetupAPI snapshot when the endpoint is currently visible
    let snapshot = serial_descriptor_snapshot(operation)?;
    if let Some(info) = snapshot.get(port_name) {
        return Ok(info.clone());
    }

    // fall back to one minimal descriptor for races during open
    Ok(WindowsSerialDescriptorInfo {
        id: port_name.to_string(),
        transport: SerialPortTransport::Native,
        name: port_name.to_string(),
        manufacturer: None,
        product: None,
        serial_number: None,
        path_units: serial_port_path(port_name).encode_utf16().collect(),
        usb_vendor_id: None,
        usb_product_id: None,
        bluetooth_service_class_id: None,
    })
}

/// Build one stable windows serial descriptor snapshot keyed by identifier.
pub(super) fn serial_descriptor_snapshot(
    operation: &'static str,
) -> RuntimeResult<BTreeMap<String, WindowsSerialDescriptorInfo>> {
    let device_info_set = serial_device_info_set(operation)?;
    let mut snapshot = BTreeMap::new();
    let mut member_index = 0u32;

    loop {
        let mut interface_data = SP_DEVICE_INTERFACE_DATA {
            cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            InterfaceClassGuid: unsafe { std::mem::zeroed() },
            Flags: 0,
            Reserved: 0,
        };

        // enumerate one live serial device interface at a time
        let status = unsafe {
            SetupDiEnumDeviceInterfaces(
                device_info_set.0,
                std::ptr::null(),
                &GUID_DEVINTERFACE_COMPORT,
                member_index,
                &mut interface_data,
            )
        };
        if status == 0 {
            let code = core_platform::last_error_code() as u32;
            if code == ERROR_NO_MORE_ITEMS {
                break;
            }

            return Err(setup_api_error(
                operation,
                "SetupDiEnumDeviceInterfaces",
                "failed to enumerate serial device interfaces",
            ));
        }

        // materialize one stable descriptor from the current interface
        let info = descriptor_info_from_interface(device_info_set.0, &interface_data, operation)?;
        snapshot.insert(info.id.clone(), info);
        member_index += 1;
    }

    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Extract one COM port name from one friendly-name suffix.
    #[test]
    fn test_port_name_from_friendly_name_reads_com_suffixes() {
        assert_eq!(
            port_name_from_friendly_name("USB Serial Device (COM7)"),
            Some(String::from("COM7")),
        );
        assert_eq!(
            port_name_from_friendly_name("Bluetooth Link (COM42)"),
            Some(String::from("COM42")),
        );
    }

    /// Reject friendly names that do not contain one COM suffix.
    #[test]
    fn test_port_name_from_friendly_name_rejects_non_port_labels() {
        assert_eq!(port_name_from_friendly_name("USB Serial Device"), None);
        assert_eq!(
            port_name_from_friendly_name("USB Serial Device (LPT1)"),
            None
        );
    }

    /// Parse usb identity fragments from one instance identifier.
    #[test]
    fn test_usb_identity_extracts_vid_pid_and_serial() {
        let instance_id = r"USB\VID_1A86&PID_7523\SER123";

        assert_eq!(usb_id_fragment(instance_id, "VID_"), Some(0x1a86));
        assert_eq!(usb_id_fragment(instance_id, "PID_"), Some(0x7523));
        assert_eq!(usb_serial_number(instance_id).as_deref(), Some("SER123"));
    }

    /// Reject usb serial extraction for interface-local instance ids.
    #[test]
    fn test_usb_serial_number_rejects_location_segments() {
        let instance_id = r"USB\VID_2341&PID_0043&MI_00\6&2B5A0E1&0&0000";

        assert_eq!(usb_serial_number(instance_id), None);
    }

    /// Classify windows serial transports from instance ids and interface paths.
    #[test]
    fn test_classify_serial_transport_detects_usb_and_bluetooth() {
        assert_eq!(
            classify_serial_transport(r"USB\VID_1A86&PID_7523\SER123", r"\\?\usb#vid_1a86"),
            SerialPortTransport::Usb,
        );
        assert_eq!(
            classify_serial_transport(
                r"BTHENUM\{00001101-0000-1000-8000-00805F9B34FB}_VID&000205AC_PID&828F\8&ABC&0&001122334455_C00000000",
                r"\\?\BTHENUM#..."
            ),
            SerialPortTransport::Bluetooth,
        );
        assert_eq!(
            classify_serial_transport(r"ACPI\PNP0501\1", r"\\?\ACPI#PNP0501"),
            SerialPortTransport::Native,
        );
    }

    /// Extract bluetooth service class identifiers from bthenum instance ids.
    #[test]
    fn test_bluetooth_service_class_id_reads_braced_guid() {
        let instance_id =
            r"BTHENUM\{00001101-0000-1000-8000-00805F9B34FB}_VID&000205AC_PID&828F\...";

        assert_eq!(
            bluetooth_service_class_id(instance_id).as_deref(),
            Some("00001101-0000-1000-8000-00805F9B34FB"),
        );
    }
}
