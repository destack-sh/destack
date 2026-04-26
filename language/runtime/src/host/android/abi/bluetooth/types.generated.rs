use super::callbacks::{
    AndroidHostBluetoothAdapterListCallback, AndroidHostBluetoothCloseCallback,
    AndroidHostBluetoothGattCharacteristicListCallback,
    AndroidHostBluetoothGattDescriptorListCallback, AndroidHostBluetoothGattMtuCallback,
    AndroidHostBluetoothGattReadCallback, AndroidHostBluetoothGattReadDescriptorCallback,
    AndroidHostBluetoothGattReadEventCallback, AndroidHostBluetoothGattServiceListCallback,
    AndroidHostBluetoothGattSubscribeCallback, AndroidHostBluetoothGattTryReadEventCallback,
    AndroidHostBluetoothGattUnsubscribeCallback, AndroidHostBluetoothGattWriteCallback,
    AndroidHostBluetoothGattWriteDescriptorCallback, AndroidHostBluetoothOpenCallback,
    AndroidHostBluetoothPairCallback, AndroidHostBluetoothReadRssiCallback,
    AndroidHostBluetoothScanCloseCallback, AndroidHostBluetoothScanOpenCallback,
    AndroidHostBluetoothScanReadCallback, AndroidHostBluetoothScanReadEventCallback,
    AndroidHostBluetoothScanTryReadCallback, AndroidHostBluetoothScanTryReadEventCallback,
    AndroidHostBluetoothSessionReadEventCallback, AndroidHostBluetoothSessionTryReadEventCallback,
    AndroidHostBluetoothUnpairCallback,
};

/// Fixed-size Android Bluetooth adapter descriptor header.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostBluetoothAdapterDescriptorHeader {
    /// Offset of the stable id string.
    pub id_offset: u32,
    /// Length of the stable id string.
    pub id_len: u32,
    /// Offset of the adapter name string.
    pub name_offset: u32,
    /// Length of the adapter name string.
    pub name_len: u32,
    /// Capability flags bitset.
    pub capability_flags: u32,
    /// Transport flags bitset.
    pub transport_flags: u32,
    /// Whether the adapter is powered.
    pub is_powered: u32,
    /// Whether the adapter is discoverable.
    pub is_discoverable: u32,
    /// Whether the adapter is currently discovering.
    pub is_discovering: u32,
}

/// Fixed-size Android Bluetooth device descriptor header.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostBluetoothDeviceDescriptorHeader {
    /// Offset of the stable id string.
    pub id_offset: u32,
    /// Length of the stable id string.
    pub id_len: u32,
    /// Offset of the adapter id string.
    pub adapter_id_offset: u32,
    /// Length of the adapter id string.
    pub adapter_id_len: u32,
    /// Offset of the device name string.
    pub name_offset: u32,
    /// Length of the device name string.
    pub name_len: u32,
    /// Offset of the local-name string.
    pub local_name_offset: u32,
    /// Length of the local-name string.
    pub local_name_len: u32,
    /// Offset of the manufacturer string.
    pub manufacturer_offset: u32,
    /// Length of the manufacturer string.
    pub manufacturer_len: u32,
    /// Offset of the model string.
    pub model_offset: u32,
    /// Length of the model string.
    pub model_len: u32,
    /// Offset of the hardware address string.
    pub address_offset: u32,
    /// Length of the hardware address string.
    pub address_len: u32,
    /// Offset of the serialized advertisement service-uuid payload.
    pub service_uuids_offset: u32,
    /// Length of the serialized advertisement service-uuid payload.
    pub service_uuids_len: u32,
    /// Offset of the serialized advertisement manufacturer-data payload.
    pub manufacturer_data_offset: u32,
    /// Length of the serialized advertisement manufacturer-data payload.
    pub manufacturer_data_len: u32,
    /// Offset of the serialized advertisement service-data payload.
    pub service_data_offset: u32,
    /// Length of the serialized advertisement service-data payload.
    pub service_data_len: u32,
    /// Transport code.
    pub transport: u32,
    /// RSSI in dBm.
    pub rssi_dbm: i32,
    /// Current pair-state code.
    pub pair_state: u32,
    /// Primary PHY flags.
    pub phy_flags: u32,
    /// Whether the device is connected.
    pub is_connected: u32,
}

/// Fixed-size Android Bluetooth scan-filter header.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostBluetoothScanFilterHeader {
    /// Whether the filter is present.
    pub is_present: u32,
    /// Offset of the exact-name string.
    pub name_offset: u32,
    /// Length of the exact-name string.
    pub name_len: u32,
    /// Offset of the name-prefix string.
    pub name_prefix_offset: u32,
    /// Length of the name-prefix string.
    pub name_prefix_len: u32,
    /// Whether the minimum RSSI is present.
    pub has_minimum_rssi: u32,
    /// Minimum RSSI threshold in dBm.
    pub minimum_rssi_dbm: i32,
    /// Requested scan-mode code.
    pub scan_mode: u32,
    /// Requested primary PHY code.
    pub primary_phy: u32,
    /// Requested secondary PHY code.
    pub secondary_phy: u32,
    /// Whether repeated devices should be kept.
    pub keep_repeated_devices: u32,
    /// Offset of the serialized service-uuid filter payload.
    pub service_uuids_offset: u32,
    /// Length of the serialized service-uuid filter payload.
    pub service_uuids_len: u32,
    /// Offset of the serialized manufacturer-data filter payload.
    pub manufacturer_data_offset: u32,
    /// Length of the serialized manufacturer-data filter payload.
    pub manufacturer_data_len: u32,
    /// Offset of the serialized service-data filter payload.
    pub service_data_offset: u32,
    /// Length of the serialized service-data filter payload.
    pub service_data_len: u32,
}

/// Fixed-size Android Bluetooth GATT service header.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostBluetoothGattServiceHeader {
    /// Offset of the service id string.
    pub id_offset: u32,
    /// Length of the service id string.
    pub id_len: u32,
    /// Offset of the UUID string.
    pub uuid_offset: u32,
    /// Length of the UUID string.
    pub uuid_len: u32,
    /// Whether the service is primary.
    pub is_primary: u32,
}

/// Fixed-size Android Bluetooth GATT characteristic header.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostBluetoothGattCharacteristicHeader {
    /// Offset of the characteristic id string.
    pub id_offset: u32,
    /// Length of the characteristic id string.
    pub id_len: u32,
    /// Offset of the service id string.
    pub service_id_offset: u32,
    /// Length of the service id string.
    pub service_id_len: u32,
    /// Offset of the UUID string.
    pub uuid_offset: u32,
    /// Length of the UUID string.
    pub uuid_len: u32,
    /// Property flags bitset.
    pub property_flags: u32,
}

/// Fixed-size Android Bluetooth GATT descriptor header.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostBluetoothGattDescriptorHeader {
    /// Offset of the descriptor id string.
    pub id_offset: u32,
    /// Length of the descriptor id string.
    pub id_len: u32,
    /// Offset of the characteristic id string.
    pub characteristic_id_offset: u32,
    /// Length of the characteristic id string.
    pub characteristic_id_len: u32,
    /// Offset of the UUID string.
    pub uuid_offset: u32,
    /// Length of the UUID string.
    pub uuid_len: u32,
}

/// Fixed-size Android Bluetooth scan event header.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostBluetoothScanEventHeader {
    /// Event timestamp in monotonic nanoseconds.
    pub timestamp_ns: u64,
    /// Event kind code.
    pub kind: u32,
    /// Embedded descriptor payload.
    pub descriptor: AndroidHostBluetoothDeviceDescriptorHeader,
}

/// Fixed-size Android Bluetooth session event header.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostBluetoothSessionEventHeader {
    /// Event timestamp in monotonic nanoseconds.
    pub timestamp_ns: u64,
    /// Event kind code.
    pub kind: u32,
    /// Pair-state code for pair-state events.
    pub pair_state: u32,
    /// Reserved flags.
    pub flags: u32,
}

/// Callback table for Android host Bluetooth interop.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostBluetoothCallbacks {
    /// Callback for adapter enumeration.
    pub adapter_list: Option<AndroidHostBluetoothAdapterListCallback>,
    /// Callback for scan open.
    pub scan_open: Option<AndroidHostBluetoothScanOpenCallback>,
    /// Callback for scan close.
    pub scan_close: Option<AndroidHostBluetoothScanCloseCallback>,
    /// Callback for scan descriptor reads.
    pub scan_read: Option<AndroidHostBluetoothScanReadCallback>,
    /// Callback for scan descriptor try-reads.
    pub scan_try_read: Option<AndroidHostBluetoothScanTryReadCallback>,
    /// Callback for scan event reads.
    pub scan_read_event: Option<AndroidHostBluetoothScanReadEventCallback>,
    /// Callback for scan event try-reads.
    pub scan_try_read_event: Option<AndroidHostBluetoothScanTryReadEventCallback>,
    /// Callback for device open.
    pub open: Option<AndroidHostBluetoothOpenCallback>,
    /// Callback for device close.
    pub close: Option<AndroidHostBluetoothCloseCallback>,
    /// Callback for pair.
    pub pair: Option<AndroidHostBluetoothPairCallback>,
    /// Callback for unpair.
    pub unpair: Option<AndroidHostBluetoothUnpairCallback>,
    /// Callback for RSSI reads.
    pub read_rssi: Option<AndroidHostBluetoothReadRssiCallback>,
    /// Callback for session event reads.
    pub session_read_event: Option<AndroidHostBluetoothSessionReadEventCallback>,
    /// Callback for session event try-reads.
    pub session_try_read_event: Option<AndroidHostBluetoothSessionTryReadEventCallback>,
    /// Callback for GATT service list.
    pub gatt_service_list: Option<AndroidHostBluetoothGattServiceListCallback>,
    /// Callback for GATT characteristic list.
    pub gatt_characteristic_list: Option<AndroidHostBluetoothGattCharacteristicListCallback>,
    /// Callback for GATT descriptor list.
    pub gatt_descriptor_list: Option<AndroidHostBluetoothGattDescriptorListCallback>,
    /// Callback for ATT MTU reads.
    pub gatt_mtu: Option<AndroidHostBluetoothGattMtuCallback>,
    /// Callback for GATT characteristic reads.
    pub gatt_read: Option<AndroidHostBluetoothGattReadCallback>,
    /// Callback for GATT descriptor reads.
    pub gatt_read_descriptor: Option<AndroidHostBluetoothGattReadDescriptorCallback>,
    /// Callback for GATT characteristic writes.
    pub gatt_write: Option<AndroidHostBluetoothGattWriteCallback>,
    /// Callback for GATT descriptor writes.
    pub gatt_write_descriptor: Option<AndroidHostBluetoothGattWriteDescriptorCallback>,
    /// Callback for GATT subscribe.
    pub gatt_subscribe: Option<AndroidHostBluetoothGattSubscribeCallback>,
    /// Callback for GATT unsubscribe.
    pub gatt_unsubscribe: Option<AndroidHostBluetoothGattUnsubscribeCallback>,
    /// Callback for GATT event reads.
    pub gatt_read_event: Option<AndroidHostBluetoothGattReadEventCallback>,
    /// Callback for GATT event try-reads.
    pub gatt_try_read_event: Option<AndroidHostBluetoothGattTryReadEventCallback>,
}
