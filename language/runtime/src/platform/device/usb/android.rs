use super::core::*;
use super::descriptor::open_usb_sys_device;
use super::service::{
    UsbDeviceFinalizer, UsbHotplugEventRecord, UsbWatchEventKind, UsbWatchResource,
    ensure_usb_service_runtime, invalid_usb_handle,
};
use super::transfer::hotplug_event_from_record;
use crate::host::HostStatus;
use crate::host::os::android::abi::usb::ffi::{
    destack_host_android_usb_device_list, destack_host_android_usb_open,
    destack_host_android_usb_watch_close, destack_host_android_usb_watch_open,
    destack_host_android_usb_watch_read, destack_host_android_usb_watch_try_read,
};
use crate::host::os::android::abi::usb::types::{
    AndroidHostUsbDeviceDescriptorHeader, AndroidHostUsbHotplugEventHeader,
};
use crate::platform::core::android::{
    checked_u32_length, host_session_id, host_status_result, invalid_data,
};
use crate::platform::core::{
    self as core_platform, decode_optional_string, decode_required_string,
};
use crate::platform::device::UsbDeviceSpeed;

/// Initial Android USB row scratch capacity.
const INITIAL_USB_ROW_CAPACITY: usize = 8;

/// Initial Android USB string scratch capacity.
const INITIAL_USB_STRING_CAPACITY: usize = 512;

/// Initial Android USB port-path scratch capacity.
const INITIAL_USB_PORT_CAPACITY: usize = 64;

/// Maximum Android USB row scratch capacity.
const MAX_USB_ROW_CAPACITY: usize = 4096;

/// Maximum Android USB string scratch capacity.
const MAX_USB_STRING_CAPACITY: usize = 1024 * 1024;

/// Maximum Android USB port-path scratch capacity.
const MAX_USB_PORT_CAPACITY: usize = 64 * 1024;

/// Android USB attached event code.
const ANDROID_USB_EVENT_ATTACHED: u32 = 1;

/// Android USB detached event code.
const ANDROID_USB_EVENT_DETACHED: u32 = 2;

/// One opened Android USB watch resource.
pub(crate) struct AndroidUsbWatchResource {
    /// The host watch identifier.
    watch_id: u64,
    /// Mutable watch state.
    state: Mutex<UsbWatchState>,
}

/// Finalizer for one Android USB hotplug watch.
pub(crate) struct AndroidUsbWatchFinalizer {
    /// The owning runtime identifier.
    runtime_id: u64,
    /// The host watch identifier.
    watch_id: u64,
}

impl ResourceFinalizer for AndroidUsbWatchFinalizer {
    /// Close one Android USB hotplug watch.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        unsafe {
            destack_host_android_usb_watch_close(self.runtime_id, self.watch_id);
        }
    }
}

/// Decode one Android USB speed code.
fn decode_usb_speed(code: u32, operation: &'static str) -> RuntimeResult<Option<UsbDeviceSpeed>> {
    let speed = match code {
        0 => return Ok(None),
        1 => UsbDeviceSpeed::Unknown,
        2 => UsbDeviceSpeed::Low,
        3 => UsbDeviceSpeed::Full,
        4 => UsbDeviceSpeed::High,
        5 => UsbDeviceSpeed::Superspeed,
        6 => UsbDeviceSpeed::SuperspeedPlus,
        _ => {
            return Err(invalid_data(
                operation,
                format!("android host returned one unknown usb speed code {code}"),
            ));
        }
    };

    Ok(Some(speed))
}

/// Decode one Android USB descriptor row.
fn decode_device_descriptor(
    header: &AndroidHostUsbDeviceDescriptorHeader,
    string_bytes: &[u8],
    port_bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<UsbDeviceDescriptorValue> {
    let port_path = if header.port_path_len == 0 {
        None
    } else {
        let start = usize::try_from(header.port_path_offset).map_err(|_| {
            invalid_data(
                operation,
                "android host usb port offset exceeded usize range",
            )
        })?;
        let length = usize::try_from(header.port_path_len).map_err(|_| {
            invalid_data(
                operation,
                "android host usb port length exceeded usize range",
            )
        })?;
        let end = start
            .checked_add(length)
            .ok_or_else(|| invalid_data(operation, "android host usb port slice overflowed"))?;
        let bytes = port_bytes.get(start..end).ok_or_else(|| {
            invalid_data(operation, "android host usb port slice was out of bounds")
        })?;

        Some(bytes.to_vec())
    };

    Ok(UsbDeviceDescriptorValue {
        id: decode_required_string(
            string_bytes,
            header.id_offset,
            header.id_len,
            "id",
            operation,
        )?,
        usb_version_bcd: if header.has_usb_version_bcd != 0 {
            Some(header.usb_version_bcd)
        } else {
            None
        },
        device_version_bcd: if header.has_device_version_bcd != 0 {
            Some(header.device_version_bcd)
        } else {
            None
        },
        vendor_id: header.vendor_id,
        product_id: header.product_id,
        class_code: header.class_code,
        subclass_code: header.subclass_code,
        protocol_code: header.protocol_code,
        manufacturer: decode_optional_string(
            string_bytes,
            header.manufacturer_offset,
            header.manufacturer_len,
            "manufacturer",
            operation,
        )?,
        product: decode_optional_string(
            string_bytes,
            header.product_offset,
            header.product_len,
            "product",
            operation,
        )?,
        serial_number: decode_optional_string(
            string_bytes,
            header.serial_number_offset,
            header.serial_number_len,
            "serialNumber",
            operation,
        )?,
        speed: decode_usb_speed(header.speed, operation)?,
        bus_number: if header.has_bus_number != 0 {
            Some(header.bus_number)
        } else {
            None
        },
        port_path,
    })
}

/// Grow the Android USB row and byte buffers after one host truncation report.
fn grow_usb_buffers(
    rows: &mut Vec<AndroidHostUsbDeviceDescriptorHeader>,
    row_count: usize,
    string_bytes: &mut Vec<u8>,
    string_count: usize,
    port_bytes: &mut Vec<u8>,
    port_count: usize,
    operation: &'static str,
) -> RuntimeResult<()> {
    let next_row_capacity = rows.len().max(row_count).saturating_mul(2);
    let next_string_capacity = string_bytes.len().max(string_count).saturating_mul(2);
    let next_port_capacity = port_bytes.len().max(port_count).saturating_mul(2);

    if next_row_capacity > MAX_USB_ROW_CAPACITY {
        return Err(invalid_data(
            operation,
            "android host usb row output exceeded the maximum supported capacity",
        ));
    }

    if next_string_capacity > MAX_USB_STRING_CAPACITY {
        return Err(invalid_data(
            operation,
            "android host usb string output exceeded the maximum supported capacity",
        ));
    }

    if next_port_capacity > MAX_USB_PORT_CAPACITY {
        return Err(invalid_data(
            operation,
            "android host usb port output exceeded the maximum supported capacity",
        ));
    }

    rows.resize(
        next_row_capacity,
        AndroidHostUsbDeviceDescriptorHeader::default(),
    );
    string_bytes.resize(next_string_capacity, 0);
    port_bytes.resize(next_port_capacity, 0);

    Ok(())
}

/// Read one Android USB device snapshot list from the host bridge.
fn read_android_usb_descriptors(
    binding: &BindingCallContext,
    operation: &'static str,
) -> RuntimeResult<Vec<UsbDeviceDescriptorValue>> {
    let runtime_id = host_session_id(binding, operation)?;
    let mut rows = vec![AndroidHostUsbDeviceDescriptorHeader::default(); INITIAL_USB_ROW_CAPACITY];
    let mut string_bytes = vec![0u8; INITIAL_USB_STRING_CAPACITY];
    let mut port_bytes = vec![0u8; INITIAL_USB_PORT_CAPACITY];

    loop {
        let row_capacity = checked_u32_length(rows.len(), operation, "usb rows")?;
        let string_capacity = checked_u32_length(string_bytes.len(), operation, "usb strings")?;
        let port_capacity = checked_u32_length(port_bytes.len(), operation, "usb port bytes")?;
        let mut row_count_written = 0u32;
        let mut string_bytes_written = 0u32;
        let mut port_bytes_written = 0u32;

        let status = unsafe {
            destack_host_android_usb_device_list(
                runtime_id,
                NativeSlice {
                    data: rows.as_mut_ptr(),
                    len: row_capacity,
                },
                &mut row_count_written,
                NativeSlice {
                    data: string_bytes.as_mut_ptr(),
                    len: string_capacity,
                },
                &mut string_bytes_written,
                NativeSlice {
                    data: port_bytes.as_mut_ptr(),
                    len: port_capacity,
                },
                &mut port_bytes_written,
            )
        };

        // resize when the host reports truncation
        if status == HostStatus::BufferTooSmall.code() {
            grow_usb_buffers(
                &mut rows,
                row_count_written as usize,
                &mut string_bytes,
                string_bytes_written as usize,
                &mut port_bytes,
                port_bytes_written as usize,
                operation,
            )?;
            continue;
        }

        host_status_result(status, operation, "usb device list")?;

        let row_count = row_count_written as usize;
        let string_count = string_bytes_written as usize;
        let port_count = port_bytes_written as usize;
        rows.truncate(row_count);
        string_bytes.truncate(string_count);
        port_bytes.truncate(port_count);

        let mut descriptors = Vec::with_capacity(rows.len());

        // decode each returned descriptor
        for row in &rows {
            let descriptor = decode_device_descriptor(row, &string_bytes, &port_bytes, operation)?;
            descriptors.push(descriptor);
        }

        descriptors.sort_by(|left, right| left.id.cmp(&right.id));

        return Ok(descriptors);
    }
}

/// Resolve one Android USB watch resource.
fn usb_watch_resource(
    binding: &BindingCallContext,
    handle: resource::UsbWatchHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<AndroidUsbWatchResource>> {
    let resource = binding.worker().resources.with_entry(handle.0, |entry| {
        if entry.kind != ResourceKind::UsbWatch {
            return None;
        }

        entry.payload_cloned::<Arc<AndroidUsbWatchResource>>()
    });

    resource
        .flatten()
        .ok_or_else(|| invalid_usb_handle(operation, "unknown usb watch handle"))
}

/// Decode one host USB hotplug event into one queued record.
fn read_android_usb_hotplug_record(
    binding: &BindingCallContext,
    watch_id: u64,
    timeout_ns: Option<u64>,
    operation: &'static str,
) -> RuntimeResult<UsbHotplugEventRecord> {
    let runtime_id = host_session_id(binding, operation)?;
    let mut event = AndroidHostUsbHotplugEventHeader::default();
    let mut string_bytes = vec![0u8; INITIAL_USB_STRING_CAPACITY];
    let mut port_bytes = vec![0u8; INITIAL_USB_PORT_CAPACITY];

    loop {
        let string_capacity = checked_u32_length(string_bytes.len(), operation, "usb strings")?;
        let port_capacity = checked_u32_length(port_bytes.len(), operation, "usb port bytes")?;
        let mut string_bytes_written = 0u32;
        let mut port_bytes_written = 0u32;

        let status = match timeout_ns {
            Some(timeout_ns) => unsafe {
                destack_host_android_usb_watch_read(
                    runtime_id,
                    watch_id,
                    timeout_ns,
                    &mut event,
                    NativeSlice {
                        data: string_bytes.as_mut_ptr(),
                        len: string_capacity,
                    },
                    &mut string_bytes_written,
                    NativeSlice {
                        data: port_bytes.as_mut_ptr(),
                        len: port_capacity,
                    },
                    &mut port_bytes_written,
                )
            },
            None => unsafe {
                destack_host_android_usb_watch_try_read(
                    runtime_id,
                    watch_id,
                    &mut event,
                    NativeSlice {
                        data: string_bytes.as_mut_ptr(),
                        len: string_capacity,
                    },
                    &mut string_bytes_written,
                    NativeSlice {
                        data: port_bytes.as_mut_ptr(),
                        len: port_capacity,
                    },
                    &mut port_bytes_written,
                )
            },
        };

        // resize when the host reports truncation
        if status == HostStatus::BufferTooSmall.code() {
            let next_string_capacity = string_bytes
                .len()
                .max(string_bytes_written as usize)
                .saturating_mul(2);
            let next_port_capacity = port_bytes
                .len()
                .max(port_bytes_written as usize)
                .saturating_mul(2);

            if next_string_capacity > MAX_USB_STRING_CAPACITY {
                return Err(invalid_data(
                    operation,
                    "android host usb event strings exceeded the maximum supported capacity",
                ));
            }

            if next_port_capacity > MAX_USB_PORT_CAPACITY {
                return Err(invalid_data(
                    operation,
                    "android host usb event port bytes exceeded the maximum supported capacity",
                ));
            }

            string_bytes.resize(next_string_capacity, 0);
            port_bytes.resize(next_port_capacity, 0);
            continue;
        }

        host_status_result(status, operation, "usb watch read")?;

        let string_count = string_bytes_written as usize;
        let port_count = port_bytes_written as usize;
        string_bytes.truncate(string_count);
        port_bytes.truncate(port_count);

        let kind = match event.kind {
            ANDROID_USB_EVENT_ATTACHED => UsbWatchEventKind::Instance,
            ANDROID_USB_EVENT_DETACHED => UsbWatchEventKind::Detached,
            _ => {
                return Err(invalid_data(
                    operation,
                    format!(
                        "android host returned one unknown usb hotplug event kind {}",
                        event.kind
                    ),
                ));
            }
        };
        let descriptor =
            decode_device_descriptor(&event.descriptor, &string_bytes, &port_bytes, operation)?;

        return Ok(UsbHotplugEventRecord {
            timestamp_ns: event.timestamp_ns,
            kind,
            descriptor,
        });
    }
}

/// Enumerate visible Android USB devices.
pub(crate) unsafe fn destack_device_usb_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<UsbDeviceDescriptor>,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // decode host descriptors and store them in binding memory
    let descriptors = read_android_usb_descriptors(binding, "destack.device.usb.list")?;
    let descriptors = descriptors
        .into_iter()
        .map(|descriptor| UsbDeviceDescriptor::from_value(binding, descriptor))
        .collect::<Vec<_>>();

    unsafe {
        out.write(binding.store_slice(descriptors));
    }

    Ok(())
}

/// Open one Android USB hotplug watch.
pub(crate) unsafe fn destack_device_usb_watch_open(
    binding: &BindingCallContext,
    out: *mut resource::UsbWatchHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // create one host watch and store per stream sequence state
    let runtime_id = host_session_id(binding, "destack.device.usb.watchOpen")?;
    let mut watch_id = 0u64;
    let status = unsafe { destack_host_android_usb_watch_open(runtime_id, &mut watch_id) };
    host_status_result(status, "destack.device.usb.watchOpen", "usb watch open")?;

    let resource = Arc::new(AndroidUsbWatchResource {
        watch_id,
        state: Mutex::new(UsbWatchState {
            next_sequence: 1,
            reported_dropped_count: 0,
        }),
    });
    let entry = ResourceEntry::new(ResourceKind::UsbWatch)
        .with_label(USB_WATCH_RESOURCE_LABEL)
        .with_payload(resource)
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            AndroidUsbWatchFinalizer {
                runtime_id,
                watch_id,
            },
        ));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::UsbWatchHandle(resource_id));
    }

    Ok(())
}

/// Close one Android USB hotplug watch.
pub(crate) unsafe fn destack_device_usb_watch_close(
    binding: &BindingCallContext,
    handle: resource::UsbWatchHandle,
) -> RuntimeResult<()> {
    let kind = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| entry.kind)
        .ok_or_else(|| {
            invalid_usb_handle("destack.device.usb.watchClose", "unknown usb watch handle")
        })?;
    if kind != ResourceKind::UsbWatch {
        return Err(invalid_usb_handle(
            "destack.device.usb.watchClose",
            "unknown usb watch handle",
        ));
    }

    if !binding.worker().resources.remove_and_finalize(
        binding.world(),
        handle.0,
        Some(binding.engine()),
    ) {
        return Err(invalid_usb_handle(
            "destack.device.usb.watchClose",
            "unknown usb watch handle",
        ));
    }

    Ok(())
}

/// Wait for one Android USB hotplug event.
pub(crate) unsafe fn destack_device_usb_watch_read(
    binding: &BindingCallContext,
    out: *mut UsbHotplugEvent,
    handle: resource::UsbWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let watch = usb_watch_resource(binding, handle, "destack.device.usb.watchRead")?;
    let record = read_android_usb_hotplug_record(
        binding,
        watch.watch_id,
        Some(timeoutns),
        "destack.device.usb.watchRead",
    )?;

    let desktop_watch = UsbWatchResource {
        queue: BoundedQueue::new(1),
        state: Mutex::new(*watch.state.lock()),
    };
    let event = hotplug_event_from_record(binding, &desktop_watch, record);
    *watch.state.lock() = *desktop_watch.state.lock();

    unsafe {
        out.write(event);
    }

    Ok(())
}

/// Poll one Android USB hotplug event without blocking.
pub(crate) unsafe fn destack_device_usb_watch_try_read(
    binding: &BindingCallContext,
    out: *mut UsbHotplugEvent,
    handle: resource::UsbWatchHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let watch = usb_watch_resource(binding, handle, "destack.device.usb.watchTryRead")?;
    let record = read_android_usb_hotplug_record(
        binding,
        watch.watch_id,
        None,
        "destack.device.usb.watchTryRead",
    )?;

    let desktop_watch = UsbWatchResource {
        queue: BoundedQueue::new(1),
        state: Mutex::new(*watch.state.lock()),
    };
    let event = hotplug_event_from_record(binding, &desktop_watch, record);
    *watch.state.lock() = *desktop_watch.state.lock();

    unsafe {
        out.write(event);
    }

    Ok(())
}

/// Open one Android USB device session.
pub(crate) unsafe fn destack_device_usb_open(
    binding: &BindingCallContext,
    out: *mut resource::UsbDeviceHandle,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // open one permission checked host device and wrap its system handle in libusb
    let runtime_id = host_session_id(binding, "destack.device.usb.open")?;
    let id = unsafe { id.as_str()? };
    let mut file_descriptor = -1i32;
    let status = unsafe {
        destack_host_android_usb_open(runtime_id, NativeStringRef::from(id), &mut file_descriptor)
    };
    host_status_result(status, "destack.device.usb.open", "usb device open")?;

    let service = binding
        .worker()
        .platform_state
        .device
        .usb_service("destack.device.usb.open")?;
    ensure_usb_service_runtime(&service)?;

    let resource = open_usb_sys_device(
        &service.state,
        file_descriptor as isize,
        id,
        "destack.device.usb.open",
    )?;
    let handle = resource.handle;
    let device = resource.device;
    let resource = Arc::new(resource);
    let entry = ResourceEntry::new(ResourceKind::UsbDevice)
        .with_label(USB_DEVICE_RESOURCE_LABEL)
        .with_payload(resource)
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            UsbDeviceFinalizer {
                service: service.state.clone(),
                handle,
                device,
            },
        ));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::UsbDeviceHandle(resource_id));
    }

    Ok(())
}
