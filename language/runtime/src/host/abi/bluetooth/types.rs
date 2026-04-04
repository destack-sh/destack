#[cfg(feature = "generator")]
use crate::host::abi::describe::host_abi_types;

#[cfg(not(feature = "generator"))]
#[allow(unused_imports)]
pub(crate) use crate::host::os::android::abi::bluetooth::types::{
    AndroidHostBluetoothAdapterDescriptorHeader, AndroidHostBluetoothDeviceDescriptorHeader,
    AndroidHostBluetoothGattCharacteristicHeader, AndroidHostBluetoothGattDescriptorHeader,
    AndroidHostBluetoothGattServiceHeader, AndroidHostBluetoothScanEventHeader,
    AndroidHostBluetoothScanFilterHeader, AndroidHostBluetoothSessionEventHeader,
};

#[cfg(feature = "generator")]
host_abi_types! {
    fn host_abi_types() {
        /// Fixed-size host bluetooth adapter descriptor header.
        struct AndroidHostBluetoothAdapterDescriptorHeader {
            /// Offset of the stable id string.
            id_offset: u32,
            /// Length of the stable id string.
            id_len: u32,
            /// Offset of the adapter name string.
            name_offset: u32,
            /// Length of the adapter name string.
            name_len: u32,
            /// Capability flags bitset.
            capability_flags: u32,
            /// Transport flags bitset.
            transport_flags: u32,
            /// Whether the adapter is powered.
            is_powered: u32,
            /// Whether the adapter is discoverable.
            is_discoverable: u32,
            /// Whether the adapter is currently discovering.
            is_discovering: u32,
        }

        /// Fixed-size host bluetooth device descriptor header.
        struct AndroidHostBluetoothDeviceDescriptorHeader {
            /// Offset of the stable id string.
            id_offset: u32,
            /// Length of the stable id string.
            id_len: u32,
            /// Offset of the adapter id string.
            adapter_id_offset: u32,
            /// Length of the adapter id string.
            adapter_id_len: u32,
            /// Offset of the device name string.
            name_offset: u32,
            /// Length of the device name string.
            name_len: u32,
            /// Offset of the local-name string.
            local_name_offset: u32,
            /// Length of the local-name string.
            local_name_len: u32,
            /// Offset of the manufacturer string.
            manufacturer_offset: u32,
            /// Length of the manufacturer string.
            manufacturer_len: u32,
            /// Offset of the model string.
            model_offset: u32,
            /// Length of the model string.
            model_len: u32,
            /// Offset of the hardware address string.
            address_offset: u32,
            /// Length of the hardware address string.
            address_len: u32,
            /// Offset of the serialized advertisement service-uuid payload.
            service_uuids_offset: u32,
            /// Length of the serialized advertisement service-uuid payload.
            service_uuids_len: u32,
            /// Offset of the serialized advertisement manufacturer-data payload.
            manufacturer_data_offset: u32,
            /// Length of the serialized advertisement manufacturer-data payload.
            manufacturer_data_len: u32,
            /// Offset of the serialized advertisement service-data payload.
            service_data_offset: u32,
            /// Length of the serialized advertisement service-data payload.
            service_data_len: u32,
            /// Transport code.
            transport: u32,
            /// RSSI in dBm.
            rssi_dbm: i32,
            /// Current pair-state code.
            pair_state: u32,
            /// Primary PHY flags.
            phy_flags: u32,
            /// Whether the device is connected.
            is_connected: u32,
        }

        /// Fixed-size host bluetooth scan-filter header.
        struct AndroidHostBluetoothScanFilterHeader {
            /// Whether the filter is present.
            is_present: u32,
            /// Offset of the exact-name string.
            name_offset: u32,
            /// Length of the exact-name string.
            name_len: u32,
            /// Offset of the name-prefix string.
            name_prefix_offset: u32,
            /// Length of the name-prefix string.
            name_prefix_len: u32,
            /// Minimum RSSI threshold in dBm.
            minimum_rssi_dbm: option(i32),
            /// Requested scan-mode code.
            scan_mode: u32,
            /// Requested primary PHY code.
            primary_phy: u32,
            /// Requested secondary PHY code.
            secondary_phy: u32,
            /// Whether repeated devices should be kept.
            keep_repeated_devices: u32,
            /// Offset of the serialized service-uuid filter payload.
            service_uuids_offset: u32,
            /// Length of the serialized service-uuid filter payload.
            service_uuids_len: u32,
            /// Offset of the serialized manufacturer-data filter payload.
            manufacturer_data_offset: u32,
            /// Length of the serialized manufacturer-data filter payload.
            manufacturer_data_len: u32,
            /// Offset of the serialized service-data filter payload.
            service_data_offset: u32,
            /// Length of the serialized service-data filter payload.
            service_data_len: u32,
        }

        /// Fixed-size host bluetooth GATT service header.
        struct AndroidHostBluetoothGattServiceHeader {
            /// Offset of the service id string.
            id_offset: u32,
            /// Length of the service id string.
            id_len: u32,
            /// Offset of the UUID string.
            uuid_offset: u32,
            /// Length of the UUID string.
            uuid_len: u32,
            /// Whether the service is primary.
            is_primary: u32,
        }

        /// Fixed-size host bluetooth GATT characteristic header.
        struct AndroidHostBluetoothGattCharacteristicHeader {
            /// Offset of the characteristic id string.
            id_offset: u32,
            /// Length of the characteristic id string.
            id_len: u32,
            /// Offset of the service id string.
            service_id_offset: u32,
            /// Length of the service id string.
            service_id_len: u32,
            /// Offset of the UUID string.
            uuid_offset: u32,
            /// Length of the UUID string.
            uuid_len: u32,
            /// Property flags bitset.
            property_flags: u32,
        }

        /// Fixed-size host bluetooth GATT descriptor header.
        struct AndroidHostBluetoothGattDescriptorHeader {
            /// Offset of the descriptor id string.
            id_offset: u32,
            /// Length of the descriptor id string.
            id_len: u32,
            /// Offset of the characteristic id string.
            characteristic_id_offset: u32,
            /// Length of the characteristic id string.
            characteristic_id_len: u32,
            /// Offset of the UUID string.
            uuid_offset: u32,
            /// Length of the UUID string.
            uuid_len: u32,
        }

        /// Fixed-size host bluetooth scan event header.
        struct AndroidHostBluetoothScanEventHeader {
            /// Event timestamp in monotonic nanoseconds.
            timestamp_ns: u64,
            /// Event kind code.
            kind: u32,
            /// Embedded descriptor payload.
            descriptor: AndroidHostBluetoothDeviceDescriptorHeader,
        }

        /// Fixed-size host bluetooth session event header.
        struct AndroidHostBluetoothSessionEventHeader {
            /// Event timestamp in monotonic nanoseconds.
            timestamp_ns: u64,
            /// Event kind code.
            kind: u32,
            /// Pair-state code for pair-state events.
            pair_state: u32,
            /// Reserved flags.
            flags: u32,
        }
    }
}
