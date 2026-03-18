use super::core::*;
use super::ffi::ffi;
use super::service::{UsbDeviceResource, UsbServiceState, libusb_error};
use crate::platform::device::{UsbControlTransferType, UsbDeviceSpeed};

/// One minimal timeout granularity for synchronous libusb transfers.
const USB_MIN_TIMEOUT_MILLIS: u32 = 1;

/// One maximum valid USB endpoint number.
const USB_MAX_ENDPOINT_NUMBER: u8 = 15;

/// Map one libusb async transfer status to one destack transfer status.
pub(crate) fn transfer_status_from_libusb_async_status(code: c_int) -> Option<UsbTransferStatus> {
    match code {
        ffi::LIBUSB_TRANSFER_COMPLETED => Some(UsbTransferStatus::Ok),
        ffi::LIBUSB_TRANSFER_STALL => Some(UsbTransferStatus::Stall),
        ffi::LIBUSB_TRANSFER_OVERFLOW => Some(UsbTransferStatus::Babble),
        _ => None,
    }
}

/// Convert one nanosecond timeout into one libusb millisecond timeout.
pub(crate) fn timeout_ns_to_millis(timeout_ns: u64) -> u32 {
    if timeout_ns == 0 {
        return USB_MIN_TIMEOUT_MILLIS;
    }

    let timeout_ms = timeout_ns.saturating_add(999_999) / 1_000_000;
    timeout_ms.clamp(u64::from(USB_MIN_TIMEOUT_MILLIS), u64::from(u32::MAX)) as u32
}

/// Convert one control-transfer setup payload into one bmRequestType.
pub(crate) fn request_type_bits(
    setup: &UsbControlSetupValue,
    direction: UsbEndpointDirection,
) -> u8 {
    let direction_bits = match direction {
        UsbEndpointDirection::In => ffi::LIBUSB_ENDPOINT_IN,
        UsbEndpointDirection::Out => ffi::LIBUSB_ENDPOINT_OUT,
    };
    let type_bits = match setup.transfer_type {
        UsbControlTransferType::Standard => ffi::LIBUSB_REQUEST_TYPE_STANDARD,
        UsbControlTransferType::Class => ffi::LIBUSB_REQUEST_TYPE_CLASS,
        UsbControlTransferType::Vendor => ffi::LIBUSB_REQUEST_TYPE_VENDOR,
    };
    let recipient_bits = match &setup.target {
        UsbControlTargetValue::UsbControlDeviceTarget(_) => ffi::LIBUSB_RECIPIENT_DEVICE,
        UsbControlTargetValue::UsbControlInterfaceTarget(_) => ffi::LIBUSB_RECIPIENT_INTERFACE,
        UsbControlTargetValue::UsbControlEndpointTarget(_) => ffi::LIBUSB_RECIPIENT_ENDPOINT,
        UsbControlTargetValue::UsbControlOtherTarget(_) => ffi::LIBUSB_RECIPIENT_OTHER,
    };

    direction_bits | type_bits | recipient_bits
}

/// Convert one control-transfer target payload into one wIndex field.
pub(crate) fn request_index(setup: &UsbControlSetupValue) -> u16 {
    match &setup.target {
        UsbControlTargetValue::UsbControlDeviceTarget(_) => 0,
        UsbControlTargetValue::UsbControlInterfaceTarget(target) => {
            u16::from(target.interface_number)
        }
        UsbControlTargetValue::UsbControlEndpointTarget(target) => {
            endpoint_address(&target.endpoint)
        }
        UsbControlTargetValue::UsbControlOtherTarget(target) => target.index,
    }
}

/// Convert one endpoint selector into one libusb endpoint address.
pub(crate) fn endpoint_address(endpoint: &UsbEndpointSelectorValue) -> u16 {
    let direction = match endpoint.direction {
        UsbEndpointDirection::In => u16::from(ffi::LIBUSB_ENDPOINT_IN),
        UsbEndpointDirection::Out => u16::from(ffi::LIBUSB_ENDPOINT_OUT),
    };

    direction | u16::from(endpoint.number & 0x0f)
}

/// Validate one endpoint selector for one transfer direction.
pub(crate) fn validate_endpoint(
    endpoint: &UsbEndpointSelectorValue,
    expected_direction: UsbEndpointDirection,
    field: &'static str,
) -> RuntimeResult<()> {
    if endpoint.number == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "endpoint number must not be zero",
        ))
        .boxed());
    }

    // usb endpoint numbers use one 4 bit field
    if endpoint.number > USB_MAX_ENDPOINT_NUMBER {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "endpoint number must be in the range 1 through 15",
        ))
        .boxed());
    }

    if endpoint.direction != expected_direction {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "endpoint direction does not match the transfer operation",
        ))
        .boxed());
    }

    Ok(())
}

/// Build one portable usb identifier from one device location.
pub(crate) fn usb_device_id(
    descriptor: &ffi::LibusbDeviceDescriptor,
    bus_number: u8,
    ports: &[u8],
) -> String {
    let port_path = if ports.is_empty() {
        String::from("root")
    } else {
        ports
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(".")
    };

    format!(
        "usb:{bus_number}:{port_path}:{:04x}:{:04x}",
        descriptor.id_vendor, descriptor.id_product
    )
}

/// Return one default language identifier for optional snapshot string reads.
fn snapshot_language_id(
    service: &UsbServiceState,
    handle: *mut ffi::LibusbDeviceHandle,
) -> Option<u16> {
    let languages = read_string_languages(service, handle).ok()?;
    languages.into_iter().next()
}

/// Read one optional snapshot string without failing enumeration.
fn read_snapshot_string(
    service: &UsbServiceState,
    handle: *mut ffi::LibusbDeviceHandle,
    index: u8,
    language_id: u16,
    operation: &'static str,
) -> Option<String> {
    read_string_descriptor_utf16(service, handle, index, language_id, operation)
        .ok()
        .flatten()
}

/// Read one UTF-16LE usb string descriptor.
pub(crate) fn read_string_descriptor_utf16(
    service: &UsbServiceState,
    handle: *mut ffi::LibusbDeviceHandle,
    index: u8,
    language_id: u16,
    operation: &'static str,
) -> RuntimeResult<Option<String>> {
    if index == 0 {
        return Ok(None);
    }

    let mut buffer = [0u8; 512];
    let length = unsafe {
        (service.api.libusb_get_string_descriptor)(
            handle,
            index,
            language_id,
            buffer.as_mut_ptr(),
            buffer.len() as c_int,
        )
    };
    if length == ffi::LIBUSB_ERROR_NOT_FOUND || length == ffi::LIBUSB_ERROR_PIPE {
        return Ok(None);
    }
    if length < 0 {
        return Err(libusb_error(
            operation,
            "libusb_get_string_descriptor",
            length,
        ));
    }
    if length < 2 {
        return Ok(None);
    }

    let payload = &buffer[2..length as usize];
    if payload.len() % 2 != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: usb string descriptor payload length was odd"
        )))
        .boxed());
    }

    let units = payload
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>();

    let text = String::from_utf16(&units).map_err(|error| {
        RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: usb string descriptor was not valid utf16: {error}",
        )))
        .boxed()
    })?;

    if text.is_empty() {
        return Ok(None);
    }

    Ok(Some(text))
}

/// Read one list of supported string descriptor language ids.
pub(crate) fn read_string_languages(
    service: &UsbServiceState,
    handle: *mut ffi::LibusbDeviceHandle,
) -> RuntimeResult<Vec<u16>> {
    let mut buffer = [0u8; 256];
    let length = unsafe {
        (service.api.libusb_get_string_descriptor)(
            handle,
            0,
            0,
            buffer.as_mut_ptr(),
            buffer.len() as c_int,
        )
    };
    if length < 0 {
        return Err(libusb_error(
            "destack.device.usb.stringLanguageList",
            "libusb_get_string_descriptor",
            length,
        ));
    }
    if length < 2 {
        return Ok(Vec::new());
    }

    let payload = &buffer[2..length as usize];
    if payload.len() % 2 != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_data(
            "destack.device.usb.stringLanguageList: malformed language descriptor payload",
        ))
        .boxed());
    }

    Ok(payload
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect())
}

/// Read one device descriptor snapshot from one libusb device pointer.
pub(crate) fn read_device_descriptor_value(
    service: &UsbServiceState,
    device: *mut ffi::LibusbDevice,
    operation: &'static str,
) -> RuntimeResult<UsbDeviceDescriptorValue> {
    read_device_descriptor_value_with_strings(service, device, operation, true)
}

/// Read one hotplug-safe device descriptor snapshot from one libusb device pointer.
pub(crate) fn read_hotplug_descriptor_value(
    service: &UsbServiceState,
    device: *mut ffi::LibusbDevice,
    operation: &'static str,
) -> RuntimeResult<UsbDeviceDescriptorValue> {
    read_device_descriptor_value_with_strings(service, device, operation, false)
}

/// Read one device descriptor snapshot from one libusb device pointer.
fn read_device_descriptor_value_with_strings(
    service: &UsbServiceState,
    device: *mut ffi::LibusbDevice,
    operation: &'static str,
    include_strings: bool,
) -> RuntimeResult<UsbDeviceDescriptorValue> {
    let descriptor = read_raw_device_descriptor(service, device, operation)?;
    let bus_number = unsafe { (service.api.libusb_get_bus_number)(device) };
    let ports = read_port_numbers(service, device, operation)?;

    // optional string metadata
    let mut manufacturer = None;
    let mut product = None;
    let mut serial_number = None;

    // avoid extra device opens in hotplug callbacks
    if include_strings {
        let mut temporary_handle = ptr::null_mut();
        let open_status = unsafe { (service.api.libusb_open)(device, &mut temporary_handle) };
        if open_status == ffi::LIBUSB_SUCCESS && !temporary_handle.is_null() {
            if let Some(language_id) = snapshot_language_id(service, temporary_handle) {
                manufacturer = read_snapshot_string(
                    service,
                    temporary_handle,
                    descriptor.i_manufacturer,
                    language_id,
                    operation,
                );
                product = read_snapshot_string(
                    service,
                    temporary_handle,
                    descriptor.i_product,
                    language_id,
                    operation,
                );
                serial_number = read_snapshot_string(
                    service,
                    temporary_handle,
                    descriptor.i_serial_number,
                    language_id,
                    operation,
                );
            }

            unsafe {
                (service.api.libusb_close)(temporary_handle);
            }
        }
    }

    Ok(UsbDeviceDescriptorValue {
        id: usb_device_id(&descriptor, bus_number, &ports),
        usb_version_bcd: Some(descriptor.bcd_usb),
        device_version_bcd: Some(descriptor.bcd_device),
        vendor_id: descriptor.id_vendor,
        product_id: descriptor.id_product,
        class_code: descriptor.b_device_class,
        subclass_code: descriptor.b_device_sub_class,
        protocol_code: descriptor.b_device_protocol,
        manufacturer,
        product,
        serial_number,
        speed: usb_device_speed(unsafe { (service.api.libusb_get_device_speed)(device) }),
        bus_number: Some(bus_number),
        port_path: Some(ports),
    })
}

/// Map one libusb speed code into the public USB device speed enum.
fn usb_device_speed(speed: c_int) -> Option<UsbDeviceSpeed> {
    match speed {
        0 => Some(UsbDeviceSpeed::Unknown),
        1 => Some(UsbDeviceSpeed::Low),
        2 => Some(UsbDeviceSpeed::Full),
        3 => Some(UsbDeviceSpeed::High),
        4 => Some(UsbDeviceSpeed::Superspeed),
        5 | 6 => Some(UsbDeviceSpeed::SuperspeedPlus),
        _ => None,
    }
}

/// Read one raw libusb device descriptor.
pub(crate) fn read_raw_device_descriptor(
    service: &UsbServiceState,
    device: *mut ffi::LibusbDevice,
    operation: &'static str,
) -> RuntimeResult<ffi::LibusbDeviceDescriptor> {
    let mut descriptor = MaybeUninit::<ffi::LibusbDeviceDescriptor>::zeroed();
    let status =
        unsafe { (service.api.libusb_get_device_descriptor)(device, descriptor.as_mut_ptr()) };
    if status != ffi::LIBUSB_SUCCESS {
        return Err(libusb_error(
            operation,
            "libusb_get_device_descriptor",
            status,
        ));
    }

    Ok(unsafe { descriptor.assume_init() })
}

/// Read one device port path.
fn read_port_numbers(
    service: &UsbServiceState,
    device: *mut ffi::LibusbDevice,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let mut ports = [0u8; 8];
    let status = unsafe { (service.api.libusb_get_port_numbers)(device, ports.as_mut_ptr(), 8) };
    if status == 0 {
        return Ok(Vec::new());
    }
    if status < 0 {
        return Err(libusb_error(operation, "libusb_get_port_numbers", status));
    }

    Ok(ports[..status as usize].to_vec())
}

/// Enumerate one stable device-id map.
pub(crate) fn enumerate_usb_device_map(
    service: &UsbServiceState,
    operation: &'static str,
) -> RuntimeResult<BTreeMap<String, UsbDeviceDescriptorValue>> {
    let descriptors = enumerate_usb_devices(service, operation)?;
    let mut map = BTreeMap::new();

    for descriptor in descriptors {
        map.insert(descriptor.id.clone(), descriptor);
    }

    Ok(map)
}

/// Enumerate current usb device descriptors.
pub(crate) fn enumerate_usb_devices(
    service: &UsbServiceState,
    operation: &'static str,
) -> RuntimeResult<Vec<UsbDeviceDescriptorValue>> {
    let devices = LibusbDeviceList::load(service, operation)?;
    let mut descriptors = Vec::new();

    // deterministic order by stable id
    for &device in devices.devices() {
        descriptors.push(read_device_descriptor_value(service, device, operation)?);
    }
    descriptors.sort_by(|left, right| left.id.cmp(&right.id));

    Ok(descriptors)
}

/// One owned libusb device-list snapshot.
struct LibusbDeviceList<'service> {
    /// Shared service state.
    service: &'service UsbServiceState,
    /// Raw libusb device list pointer.
    list: *const *mut ffi::LibusbDevice,
    /// Visible devices.
    devices: Vec<*mut ffi::LibusbDevice>,
}

impl<'service> LibusbDeviceList<'service> {
    /// Load one libusb device list snapshot.
    fn load(service: &'service UsbServiceState, operation: &'static str) -> RuntimeResult<Self> {
        let mut list = ptr::null();
        let count =
            unsafe { (service.api.libusb_get_device_list)(service.context.as_ptr(), &mut list) };
        if count < 0 {
            return Err(libusb_error(
                operation,
                "libusb_get_device_list",
                count as c_int,
            ));
        }

        let mut devices = Vec::with_capacity(count as usize);
        for index in 0..count as usize {
            let device = unsafe { *list.add(index) };
            if device.is_null() {
                continue;
            }

            devices.push(device);
        }

        Ok(Self {
            service,
            list,
            devices,
        })
    }

    /// Return the visible devices.
    fn devices(&self) -> &[*mut ffi::LibusbDevice] {
        &self.devices
    }
}

impl Drop for LibusbDeviceList<'_> {
    /// Free the libusb list without unref'ing the device entries.
    fn drop(&mut self) {
        unsafe {
            (self.service.api.libusb_free_device_list)(self.list, 0);
        }
    }
}

/// Open one device by stable id.
#[cfg(not(target_os = "android"))]
pub(crate) fn open_usb_device(
    service: &Arc<UsbServiceState>,
    id: &str,
    operation: &'static str,
) -> RuntimeResult<UsbDeviceResource> {
    let devices = LibusbDeviceList::load(service, operation)?;

    for &device in devices.devices() {
        let descriptor = read_device_descriptor_value(service, device, operation)?;
        if descriptor.id != id {
            continue;
        }

        let mut handle = ptr::null_mut();
        let status = unsafe { (service.api.libusb_open)(device, &mut handle) };
        if status != ffi::LIBUSB_SUCCESS {
            return Err(libusb_error(operation, "libusb_open", status));
        }

        let device = unsafe { (service.api.libusb_ref_device)(device) };
        return Ok(UsbDeviceResource {
            service: service.clone(),
            handle,
            device,
            descriptor,
            active_transfers: Mutex::new(BTreeMap::new()),
            operation_lock: Mutex::new(()),
        });
    }

    Err(core_platform::io_not_found(
        operation,
        format!("usb device {id} was not found"),
    ))
}

/// Open one usb device from one host provided system handle.
#[cfg(target_os = "android")]
pub(crate) fn open_usb_sys_device(
    service: &Arc<UsbServiceState>,
    sys_device: isize,
    id: &str,
    operation: &'static str,
) -> RuntimeResult<UsbDeviceResource> {
    let wrap_sys_device = service.api.libusb_wrap_sys_device.ok_or_else(|| {
        RuntimeError::from(PlatformError::not_supported(format!(
            "{operation}: libusb_wrap_sys_device is unavailable",
        )))
        .boxed()
    })?;

    let mut handle = ptr::null_mut();
    let status = unsafe { wrap_sys_device(service.context.as_ptr(), sys_device, &mut handle) };
    if status != ffi::LIBUSB_SUCCESS {
        return Err(libusb_error(operation, "libusb_wrap_sys_device", status));
    }

    let device = unsafe { (service.api.libusb_get_device)(handle) };
    if device.is_null() {
        unsafe {
            (service.api.libusb_close)(handle);
        }

        return Err(RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: libusb_get_device returned one null device",
        )))
        .boxed());
    }

    let descriptor = read_device_descriptor_value(service, device, operation)?;
    let device = unsafe { (service.api.libusb_ref_device)(device) };
    let descriptor = UsbDeviceDescriptorValue {
        id: id.to_string(),
        ..descriptor
    };

    Ok(UsbDeviceResource {
        service: service.clone(),
        handle,
        device,
        descriptor,
        active_transfers: Mutex::new(BTreeMap::new()),
        operation_lock: Mutex::new(()),
    })
}

/// Read one configuration descriptor set.
pub(crate) fn read_configuration_descriptors(
    service: &UsbServiceState,
    handle: *mut ffi::LibusbDeviceHandle,
    device: *mut ffi::LibusbDevice,
    operation: &'static str,
) -> RuntimeResult<Vec<UsbConfigurationDescriptorValue>> {
    let raw_descriptor = read_raw_device_descriptor(service, device, operation)?;
    let language_id = snapshot_language_id(service, handle);
    let speed = unsafe { (service.api.libusb_get_device_speed)(device) };
    let mut configurations = Vec::new();

    for config_index in 0..raw_descriptor.b_num_configurations {
        let mut config = ptr::null_mut();
        let status = unsafe {
            (service.api.libusb_get_config_descriptor)(device, config_index, &mut config)
        };
        if status != ffi::LIBUSB_SUCCESS {
            return Err(libusb_error(
                operation,
                "libusb_get_config_descriptor",
                status,
            ));
        }

        let configuration = unsafe {
            build_configuration_descriptor(service, handle, config, language_id, speed, operation)
        };

        // free the raw descriptor even when decoding fails
        unsafe {
            (service.api.libusb_free_config_descriptor)(config);
        }

        let configuration = configuration?;
        configurations.push(configuration);
    }

    Ok(configurations)
}

/// Build one configuration descriptor value.
unsafe fn build_configuration_descriptor(
    service: &UsbServiceState,
    handle: *mut ffi::LibusbDeviceHandle,
    config: *mut ffi::LibusbConfigDescriptor,
    language_id: Option<u16>,
    speed: c_int,
    operation: &'static str,
) -> RuntimeResult<UsbConfigurationDescriptorValue> {
    let config = unsafe { &*config };
    let mut interfaces = Vec::new();

    // flatten alternate settings into one explicit interface list
    for interface_index in 0..usize::from(config.b_num_interfaces) {
        let interface = unsafe { &*config.interface.add(interface_index) };
        for alternate_index in 0..interface.num_altsetting.max(0) as usize {
            let descriptor = unsafe { &*interface.altsetting.add(alternate_index) };
            let mut endpoints = Vec::new();

            for endpoint_index in 0..usize::from(descriptor.b_num_endpoints) {
                let endpoint = unsafe { &*descriptor.endpoint.add(endpoint_index) };
                endpoints.push(UsbEndpointDescriptorValue {
                    endpoint_number: endpoint.b_endpoint_address & 0x0f,
                    direction: if endpoint.b_endpoint_address & ffi::LIBUSB_ENDPOINT_IN != 0 {
                        UsbEndpointDirection::In
                    } else {
                        UsbEndpointDirection::Out
                    },
                    transfer_type: match endpoint.bm_attributes
                        & ffi::LIBUSB_ENDPOINT_TRANSFER_TYPE_MASK
                    {
                        1 => UsbEndpointTransferType::Isochronous,
                        2 => UsbEndpointTransferType::Bulk,
                        _ => UsbEndpointTransferType::Interrupt,
                    },
                    max_packet_size: endpoint.w_max_packet_size,
                    interval: endpoint.b_interval,
                });
            }

            let name = match language_id {
                Some(language_id) => read_string_descriptor_utf16(
                    service,
                    handle,
                    descriptor.i_interface,
                    language_id,
                    operation,
                )?,
                None => None,
            };

            interfaces.push(UsbInterfaceDescriptorValue {
                number: descriptor.b_interface_number,
                alternate_setting: descriptor.b_alternate_setting,
                class_code: descriptor.b_interface_class,
                subclass_code: descriptor.b_interface_sub_class,
                protocol_code: descriptor.b_interface_protocol,
                name,
                endpoints,
            });
        }
    }

    let name = match language_id {
        Some(language_id) => read_string_descriptor_utf16(
            service,
            handle,
            config.i_configuration,
            language_id,
            operation,
        )?,
        None => None,
    };

    let power_unit = if speed >= ffi::LIBUSB_SPEED_SUPER {
        8
    } else {
        2
    };

    Ok(UsbConfigurationDescriptorValue {
        value: config.b_configuration_value,
        name,
        self_powered: (config.bm_attributes & ffi::USB_CONFIG_SELF_POWERED) != 0,
        remote_wakeup: (config.bm_attributes & ffi::USB_CONFIG_REMOTE_WAKEUP) != 0,
        max_power_milli_amps: u16::from(config.max_power) * power_unit,
        interfaces,
    })
}

/// Read one list of BOS capability descriptors.
pub(crate) fn read_bos_capabilities(
    service: &UsbServiceState,
    handle: *mut ffi::LibusbDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<Vec<UsbBosCapabilityDescriptorValue>> {
    let mut bos = ptr::null_mut();
    let status = unsafe { (service.api.libusb_get_bos_descriptor)(handle, &mut bos) };
    if status == ffi::LIBUSB_ERROR_NOT_FOUND || status == ffi::LIBUSB_ERROR_NOT_SUPPORTED {
        return Ok(Vec::new());
    }
    if status != ffi::LIBUSB_SUCCESS {
        return Err(libusb_error(operation, "libusb_get_bos_descriptor", status));
    }

    let bos = unsafe { &*bos };
    let result = (|| {
        let mut capabilities = Vec::new();
        for index in 0..usize::from(bos.b_num_device_caps) {
            let capability = unsafe { *bos.dev_capability.as_ptr().add(index) };
            if capability.is_null() {
                continue;
            }

            capabilities.push(read_bos_capability(service, capability, operation)?);
        }

        Ok(capabilities)
    })();

    unsafe {
        (service.api.libusb_free_bos_descriptor)(bos as *const _ as *mut _);
    }

    result
}

/// Read one BOS capability value.
fn read_bos_capability(
    service: &UsbServiceState,
    capability: *mut ffi::LibusbBosDevCapabilityDescriptor,
    operation: &'static str,
) -> RuntimeResult<UsbBosCapabilityDescriptorValue> {
    let capability_ref = unsafe { &*capability };
    let raw_bytes = unsafe {
        std::slice::from_raw_parts(
            capability_ref.dev_capability_data.as_ptr(),
            usize::from(capability_ref.b_length.saturating_sub(3)),
        )
    }
    .to_vec();

    let mut platform_uuid = None;
    let kind = match capability_ref.b_dev_capability_type {
        ffi::LIBUSB_BT_USB_2_0_EXTENSION => UsbBosCapabilityKind::Usb20Extension,
        ffi::LIBUSB_BT_SS_USB_DEVICE_CAPABILITY => UsbBosCapabilityKind::SuperspeedUsb,
        ffi::LIBUSB_BT_CONTAINER_ID => UsbBosCapabilityKind::ContainerId,
        ffi::LIBUSB_BT_PLATFORM_DESCRIPTOR => {
            let mut platform = ptr::null_mut();
            let status = unsafe {
                (service.api.libusb_get_platform_descriptor)(
                    service.context.as_ptr(),
                    capability,
                    &mut platform,
                )
            };
            if status == ffi::LIBUSB_SUCCESS && !platform.is_null() {
                let uuid = unsafe { (*platform).platform_capability_uuid };
                platform_uuid = Some(uuid_string(&uuid));
                unsafe {
                    (service.api.libusb_free_platform_descriptor)(platform);
                }
            }

            UsbBosCapabilityKind::Platform
        }
        1 => UsbBosCapabilityKind::WirelessUsb,
        6 => UsbBosCapabilityKind::Billboard,
        7 => UsbBosCapabilityKind::ConfigurationSummary,
        _ => UsbBosCapabilityKind::Unknown,
    };

    if matches!(kind, UsbBosCapabilityKind::Usb20Extension) {
        let mut extension = ptr::null_mut();
        let status = unsafe {
            (service.api.libusb_get_usb_2_0_extension_descriptor)(
                service.context.as_ptr(),
                capability,
                &mut extension,
            )
        };
        if status == ffi::LIBUSB_SUCCESS && !extension.is_null() {
            unsafe {
                (service.api.libusb_free_usb_2_0_extension_descriptor)(extension);
            }
        } else if status != ffi::LIBUSB_SUCCESS && status != ffi::LIBUSB_ERROR_NOT_SUPPORTED {
            return Err(libusb_error(
                operation,
                "libusb_get_usb_2_0_extension_descriptor",
                status,
            ));
        }
    }

    if matches!(kind, UsbBosCapabilityKind::SuperspeedUsb) {
        let mut superspeed = ptr::null_mut();
        let status = unsafe {
            (service.api.libusb_get_ss_usb_device_capability_descriptor)(
                service.context.as_ptr(),
                capability,
                &mut superspeed,
            )
        };
        if status == ffi::LIBUSB_SUCCESS && !superspeed.is_null() {
            unsafe {
                (service.api.libusb_free_ss_usb_device_capability_descriptor)(superspeed);
            }
        } else if status != ffi::LIBUSB_SUCCESS && status != ffi::LIBUSB_ERROR_NOT_SUPPORTED {
            return Err(libusb_error(
                operation,
                "libusb_get_ss_usb_device_capability_descriptor",
                status,
            ));
        }
    }

    if matches!(kind, UsbBosCapabilityKind::ContainerId) {
        let mut container = ptr::null_mut();
        let status = unsafe {
            (service.api.libusb_get_container_id_descriptor)(
                service.context.as_ptr(),
                capability,
                &mut container,
            )
        };
        if status == ffi::LIBUSB_SUCCESS && !container.is_null() {
            unsafe {
                (service.api.libusb_free_container_id_descriptor)(container);
            }
        } else if status != ffi::LIBUSB_SUCCESS && status != ffi::LIBUSB_ERROR_NOT_SUPPORTED {
            return Err(libusb_error(
                operation,
                "libusb_get_container_id_descriptor",
                status,
            ));
        }
    }

    Ok(UsbBosCapabilityDescriptorValue {
        kind,
        capability_type: capability_ref.b_dev_capability_type,
        platform_uuid,
        bytes: raw_bytes,
    })
}

/// Convert one UUID byte array into one canonical string.
fn uuid_string(bytes: &[u8; 16]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15],
    )
}
