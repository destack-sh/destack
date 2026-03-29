use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    fn host_abi_module() -> "bluetooth" {
        platforms: [android];
        types: super::types::host_abi_types();
        requests {
            /// List one slice of host bluetooth adapters.
            fn adapter_list(
                session_handle: session_handle,
                adapters: slice(AndroidHostBluetoothAdapterDescriptorHeader),
                adapter_count_written: output(u32),
                string_bytes: slice(u8),
                string_bytes_written: output(u32),
            ) -> host_status;

            /// Open one host bluetooth scan session.
            fn scan_open(
                session_handle: session_handle,
                adapter_id: string_ref,
                filter: AndroidHostBluetoothScanFilterHeader,
                filter_flags: u32,
                filter_bytes: slice(u8),
                session_id: output(u64),
            ) -> host_status;

            /// Close one host bluetooth scan session.
            fn scan_close(
                session_handle: session_handle,
                session_id: u64,
            ) -> host_status;

            /// Read one batch of host bluetooth scan results.
            fn scan_read(
                session_handle: session_handle,
                session_id: u64,
                timeout_ns: u64,
                devices: slice(AndroidHostBluetoothDeviceDescriptorHeader),
                device_count_written: output(u32),
                string_bytes: slice(u8),
                string_bytes_written: output(u32),
            ) -> host_status;

            /// Try one nonblocking host bluetooth scan read.
            fn scan_try_read(
                session_handle: session_handle,
                session_id: u64,
                devices: slice(AndroidHostBluetoothDeviceDescriptorHeader),
                device_count_written: output(u32),
                string_bytes: slice(u8),
                string_bytes_written: output(u32),
            ) -> host_status;

            /// Read one host bluetooth scan event.
            fn scan_read_event(
                session_handle: session_handle,
                session_id: u64,
                timeout_ns: u64,
                event: output(AndroidHostBluetoothScanEventHeader),
                string_bytes: slice(u8),
                string_bytes_written: output(u32),
            ) -> host_status;

            /// Try one nonblocking host bluetooth scan event read.
            fn scan_try_read_event(
                session_handle: session_handle,
                session_id: u64,
                event: output(AndroidHostBluetoothScanEventHeader),
                string_bytes: slice(u8),
                string_bytes_written: output(u32),
            ) -> host_status;

            /// Open one host bluetooth device session.
            fn open(
                session_handle: session_handle,
                adapter_id: string_ref,
                device_id: string_ref,
                session_id: output(u64),
                descriptor: output(AndroidHostBluetoothDeviceDescriptorHeader),
                string_bytes: slice(u8),
                string_bytes_written: output(u32),
            ) -> host_status;

            /// Close one host bluetooth device session.
            fn close(
                session_handle: session_handle,
                session_id: u64,
            ) -> host_status;

            /// Pair one host bluetooth device session.
            fn pair(
                session_handle: session_handle,
                session_id: u64,
                timeout_ns: u64,
            ) -> host_status;

            /// Unpair one host bluetooth device.
            fn unpair(
                session_handle: session_handle,
                adapter_id: string_ref,
                device_id: string_ref,
            ) -> host_status;

            /// Read one host bluetooth RSSI sample.
            fn read_rssi(
                session_handle: session_handle,
                session_id: u64,
                timeout_ns: u64,
                rssi_dbm: output(i16),
            ) -> host_status;

            /// Read one host bluetooth session event.
            fn session_read_event(
                session_handle: session_handle,
                session_id: u64,
                timeout_ns: u64,
                event: output(AndroidHostBluetoothSessionEventHeader),
            ) -> host_status;

            /// Try one nonblocking host bluetooth session event read.
            fn session_try_read_event(
                session_handle: session_handle,
                session_id: u64,
                event: output(AndroidHostBluetoothSessionEventHeader),
            ) -> host_status;

            /// List one host bluetooth GATT service slice.
            fn gatt_service_list(
                session_handle: session_handle,
                session_id: u64,
                services: slice(AndroidHostBluetoothGattServiceHeader),
                service_count_written: output(u32),
                string_bytes: slice(u8),
                string_bytes_written: output(u32),
            ) -> host_status;

            /// List one host bluetooth GATT characteristic slice.
            fn gatt_characteristic_list(
                session_handle: session_handle,
                session_id: u64,
                service_id: string_ref,
                characteristics: slice(AndroidHostBluetoothGattCharacteristicHeader),
                characteristic_count_written: output(u32),
                string_bytes: slice(u8),
                string_bytes_written: output(u32),
            ) -> host_status;

            /// List one host bluetooth GATT descriptor slice.
            fn gatt_descriptor_list(
                session_handle: session_handle,
                session_id: u64,
                characteristic_id: string_ref,
                descriptors: slice(AndroidHostBluetoothGattDescriptorHeader),
                descriptor_count_written: output(u32),
                string_bytes: slice(u8),
                string_bytes_written: output(u32),
            ) -> host_status;

            /// Query one host bluetooth GATT MTU.
            fn gatt_mtu(
                session_handle: session_handle,
                session_id: u64,
                mtu: output(u16),
            ) -> host_status;

            /// Read one host bluetooth GATT characteristic value.
            fn gatt_read(
                session_handle: session_handle,
                session_id: u64,
                characteristic_id: string_ref,
                timeout_ns: u64,
                bytes: slice(u8),
                bytes_written: output(u32),
            ) -> host_status;

            /// Read one host bluetooth GATT descriptor value.
            fn gatt_read_descriptor(
                session_handle: session_handle,
                session_id: u64,
                descriptor_id: string_ref,
                timeout_ns: u64,
                bytes: slice(u8),
                bytes_written: output(u32),
            ) -> host_status;

            /// Write one host bluetooth GATT characteristic value.
            fn gatt_write(
                session_handle: session_handle,
                session_id: u64,
                characteristic_id: string_ref,
                mode: u32,
                value: slice(u8),
                timeout_ns: u64,
            ) -> host_status;

            /// Write one host bluetooth GATT descriptor value.
            fn gatt_write_descriptor(
                session_handle: session_handle,
                session_id: u64,
                descriptor_id: string_ref,
                value: slice(u8),
                timeout_ns: u64,
            ) -> host_status;

            /// Subscribe to one host bluetooth GATT characteristic.
            fn gatt_subscribe(
                session_handle: session_handle,
                session_id: u64,
                characteristic_id: string_ref,
                subscription_id: output(u64),
            ) -> host_status;

            /// Unsubscribe from one host bluetooth GATT characteristic.
            fn gatt_unsubscribe(
                session_handle: session_handle,
                subscription_id: u64,
            ) -> host_status;

            /// Read one host bluetooth GATT subscription event.
            fn gatt_read_event(
                session_handle: session_handle,
                subscription_id: u64,
                timeout_ns: u64,
                event: output(slice(u8)),
                timestamp_ns: output(u64),
            ) -> host_status;

            /// Try one nonblocking host bluetooth GATT event read.
            fn gatt_try_read_event(
                session_handle: session_handle,
                subscription_id: u64,
                event: output(slice(u8)),
                timestamp_ns: output(u64),
            ) -> host_status;
        }
        ingress {}
    }
}
