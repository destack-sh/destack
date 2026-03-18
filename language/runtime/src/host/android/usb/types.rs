use super::callbacks::{
    AndroidHostUsbDeviceListCallback, AndroidHostUsbOpenCallback, AndroidHostUsbWatchCloseCallback,
    AndroidHostUsbWatchOpenCallback, AndroidHostUsbWatchReadCallback,
    AndroidHostUsbWatchTryReadCallback,
};

/// Fixed size Android USB device descriptor header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct AndroidHostUsbDeviceDescriptorHeader {
    /// Offset of the stable id string.
    pub id_offset: u32,
    /// Length of the stable id string.
    pub id_len: u32,
    /// Offset of the manufacturer string.
    pub manufacturer_offset: u32,
    /// Length of the manufacturer string.
    pub manufacturer_len: u32,
    /// Offset of the product string.
    pub product_offset: u32,
    /// Length of the product string.
    pub product_len: u32,
    /// Offset of the serial string.
    pub serial_number_offset: u32,
    /// Length of the serial string.
    pub serial_number_len: u32,
    /// Offset of the port path bytes.
    pub port_path_offset: u32,
    /// Length of the port path bytes.
    pub port_path_len: u32,
    /// USB specification version in binary coded decimal form.
    pub usb_version_bcd: u16,
    /// Whether the USB version is present.
    pub has_usb_version_bcd: u32,
    /// Device release version in binary coded decimal form.
    pub device_version_bcd: u16,
    /// Whether the device version is present.
    pub has_device_version_bcd: u32,
    /// USB vendor identifier.
    pub vendor_id: u16,
    /// USB product identifier.
    pub product_id: u16,
    /// Device class code.
    pub class_code: u8,
    /// Device subclass code.
    pub subclass_code: u8,
    /// Device protocol code.
    pub protocol_code: u8,
    /// Device speed code.
    pub speed: u32,
    /// Whether the speed is present.
    pub has_speed: u32,
    /// Bus number when available.
    pub bus_number: u8,
    /// Whether the bus number is present.
    pub has_bus_number: u32,
}

/// Fixed size Android USB hotplug event header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct AndroidHostUsbHotplugEventHeader {
    /// Event timestamp in monotonic nanoseconds.
    pub timestamp_ns: u64,
    /// Event kind code.
    pub kind: u32,
    /// Embedded device descriptor payload.
    pub descriptor: AndroidHostUsbDeviceDescriptorHeader,
}

/// Callback table for Android host USB interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostUsbCallbacks {
    /// Callback for device enumeration.
    pub device_list: Option<AndroidHostUsbDeviceListCallback>,
    /// Callback for watch open.
    pub watch_open: Option<AndroidHostUsbWatchOpenCallback>,
    /// Callback for watch close.
    pub watch_close: Option<AndroidHostUsbWatchCloseCallback>,
    /// Callback for blocking watch reads.
    pub watch_read: Option<AndroidHostUsbWatchReadCallback>,
    /// Callback for nonblocking watch reads.
    pub watch_try_read: Option<AndroidHostUsbWatchTryReadCallback>,
    /// Callback for device open.
    pub open: Option<AndroidHostUsbOpenCallback>,
}
