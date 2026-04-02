#[cfg(feature = "generator")]
use crate::host::abi::describe::host_abi_types;

#[cfg(not(feature = "generator"))]
pub(crate) use crate::host::android::abi::usb::types::{
    AndroidHostUsbDeviceDescriptorHeader, AndroidHostUsbHotplugEventHeader,
};

#[cfg(feature = "generator")]
host_abi_types! {
    fn host_abi_types() {
        /// Fixed-size host USB device descriptor header.
        struct AndroidHostUsbDeviceDescriptorHeader {
            /// Offset of the stable id string.
            id_offset: u32,
            /// Length of the stable id string.
            id_len: u32,
            /// Offset of the manufacturer string.
            manufacturer_offset: u32,
            /// Length of the manufacturer string.
            manufacturer_len: u32,
            /// Offset of the product string.
            product_offset: u32,
            /// Length of the product string.
            product_len: u32,
            /// Offset of the serial string.
            serial_number_offset: u32,
            /// Length of the serial string.
            serial_number_len: u32,
            /// Offset of the port path bytes.
            port_path_offset: u32,
            /// Length of the port path bytes.
            port_path_len: u32,
            /// USB specification version in binary coded decimal form.
            usb_version_bcd: option(u16),
            /// Device release version in binary coded decimal form.
            device_version_bcd: option(u16),
            /// USB vendor identifier.
            vendor_id: u16,
            /// USB product identifier.
            product_id: u16,
            /// Device class code.
            class_code: u8,
            /// Device subclass code.
            subclass_code: u8,
            /// Device protocol code.
            protocol_code: u8,
            /// Device speed code.
            speed: option(u32),
            /// Bus number when available.
            bus_number: option(u8),
        }

        /// Fixed-size host USB hotplug event header.
        struct AndroidHostUsbHotplugEventHeader {
            /// Event timestamp in monotonic nanoseconds.
            timestamp_ns: u64,
            /// Event kind code.
            kind: u32,
            /// Embedded device descriptor payload.
            descriptor: AndroidHostUsbDeviceDescriptorHeader,
        }
    }
}
