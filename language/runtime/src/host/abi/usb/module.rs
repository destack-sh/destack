use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    fn host_abi_module() -> "usb" {
        platforms: [android];
        types: super::types::host_abi_types();
        requests {
            /// List one slice of host USB devices.
            fn device_list(
                session_handle: session_handle,
                devices: slice(AndroidHostUsbDeviceDescriptorHeader),
                device_count_written: output(u32),
                string_bytes: slice(u8),
                string_bytes_written: output(u32),
                port_bytes: slice(u8),
                port_bytes_written: output(u32),
            ) -> host_status;

            /// Open one host USB hotplug watch.
            fn watch_open(
                session_handle: session_handle,
                watch_id: output(u64),
            ) -> host_status;

            /// Close one host USB hotplug watch.
            fn watch_close(
                session_handle: session_handle,
                watch_id: u64,
            ) -> host_status;

            /// Read one host USB hotplug event.
            fn watch_read(
                session_handle: session_handle,
                watch_id: u64,
                timeout_ns: u64,
                event: output(AndroidHostUsbHotplugEventHeader),
                string_bytes: slice(u8),
                string_bytes_written: output(u32),
                port_bytes: slice(u8),
                port_bytes_written: output(u32),
            ) -> host_status;

            /// Try one nonblocking host USB hotplug event read.
            fn watch_try_read(
                session_handle: session_handle,
                watch_id: u64,
                event: output(AndroidHostUsbHotplugEventHeader),
                string_bytes: slice(u8),
                string_bytes_written: output(u32),
                port_bytes: slice(u8),
                port_bytes_written: output(u32),
            ) -> host_status;

            /// Open one permission-checked host USB device.
            fn open(
                session_handle: session_handle,
                id: string_ref,
                file_descriptor: output(i32),
            ) -> host_status;
        }
        ingress {}
    }
}
