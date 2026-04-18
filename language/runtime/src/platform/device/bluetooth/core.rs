#![cfg_attr(windows, allow(dead_code))]

use parking_lot::Mutex;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::device::{
    BluetoothAdapterAttachedEventValue, BluetoothAdapterChangedEventValue,
    BluetoothAdapterDescriptorValue, BluetoothAdapterDetachedEventValue, BluetoothAdapterEvent,
    BluetoothAdapterEventMetadataValue, BluetoothAdapterEventValue, BluetoothDataFilterValue,
    BluetoothDeviceDescriptorValue, BluetoothGattValueEvent, BluetoothGattValueEventValue,
    BluetoothPairState, BluetoothScanDiscoveredEventValue, BluetoothScanEvent,
    BluetoothScanEventMetadataValue, BluetoothScanEventValue, BluetoothScanFilterValue,
    BluetoothScanLostEventValue, BluetoothScanUpdatedEventValue,
    BluetoothSessionDisconnectedEventValue, BluetoothSessionEvent, BluetoothSessionEventMetadata,
    BluetoothSessionEventValue, BluetoothSessionGattDatabaseChangedEventValue,
    BluetoothSessionPairStateChangedEventValue,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::ResourceKind;
use crate::platform::{NativeAbiCodec, core as core_platform, resource};
use crate::runtime::BindingCallContext;

/// Resource-table label for one bluetooth adapter watch stream.
pub(super) const BLUETOOTH_ADAPTER_WATCH_RESOURCE_LABEL: &str = "device.bluetooth.adapter.watch";

/// Resource-table label for one bluetooth scan session.
pub(super) const BLUETOOTH_SCAN_RESOURCE_LABEL: &str = "device.bluetooth.scan";

/// Resource-table label for one bluetooth device session.
pub(super) const BLUETOOTH_DEVICE_RESOURCE_LABEL: &str = "device.bluetooth.device";

/// Resource-table label for one bluetooth notification subscription.
pub(super) const BLUETOOTH_SUBSCRIPTION_RESOURCE_LABEL: &str = "device.bluetooth.subscription";

/// One scan event queue capacity.
#[cfg(not(target_os = "android"))]
pub(super) const BLUETOOTH_SCAN_EVENT_QUEUE_CAPACITY: usize = 256;

/// One session event queue capacity.
#[cfg(not(target_os = "android"))]
pub(super) const BLUETOOTH_SESSION_EVENT_QUEUE_CAPACITY: usize = 128;

/// One notification queue capacity.
#[cfg(not(target_os = "android"))]
pub(super) const BLUETOOTH_NOTIFICATION_QUEUE_CAPACITY: usize = 256;

/// Monotonic event sequencing for one bluetooth queue-backed resource.
#[derive(Debug, Clone, Copy)]
pub(super) struct BluetoothEventState {
    /// The next event sequence number to emit.
    pub(super) next_sequence: u64,
}

/// Build one invalid bluetooth handle error.
pub(super) fn invalid_bluetooth_handle(
    operation: &'static str,
    kind: &'static str,
) -> Box<RuntimeError> {
    core_platform::io_operation_error(
        operation,
        Some(PlatformErrorCode::InvalidArgumentValue),
        format!("unknown bluetooth {kind} handle"),
    )
}

/// Resolve one typed bluetooth payload from the resource table.
pub(super) fn bluetooth_payload<T: Clone + 'static>(
    binding: &BindingCallContext,
    handle: resource::ResourceId,
    kind: ResourceKind,
    operation: &'static str,
    label: &'static str,
) -> RuntimeResult<T> {
    let resolved = binding.worker().resources.with_entry(handle, |entry| {
        if entry.kind != kind {
            return None;
        }

        entry.payload_cloned::<T>()
    });

    resolved
        .flatten()
        .ok_or_else(|| invalid_bluetooth_handle(operation, label))
}

/// Close one bluetooth resource and run finalization.
pub(super) fn close_bluetooth_resource(
    binding: &BindingCallContext,
    handle: resource::ResourceId,
    kind: ResourceKind,
    operation: &'static str,
    label: &'static str,
) -> RuntimeResult<()> {
    let entry_kind = binding
        .worker()
        .resources
        .with_entry(handle, |entry| entry.kind)
        .ok_or_else(|| invalid_bluetooth_handle(operation, label))?;
    if entry_kind != kind {
        return Err(invalid_bluetooth_handle(operation, label));
    }

    if !binding.worker().resources.remove_and_finalize(
        &binding.world(),
        handle,
        Some(binding.engine()),
    ) {
        return Err(invalid_bluetooth_handle(operation, label));
    }

    Ok(())
}

/// Build one discovered bluetooth scan event.
pub(super) fn scan_discovered_event(
    state: &Mutex<BluetoothEventState>,
    device: BluetoothDeviceDescriptorValue,
) -> BluetoothScanEventValue {
    BluetoothScanEventValue::BluetoothScanDiscoveredEvent(BluetoothScanDiscoveredEventValue {
        kind: String::from("discovered"),
        metadata: next_scan_metadata(state, device),
    })
}

/// Build one updated bluetooth scan event.
pub(super) fn scan_updated_event(
    state: &Mutex<BluetoothEventState>,
    device: BluetoothDeviceDescriptorValue,
) -> BluetoothScanEventValue {
    BluetoothScanEventValue::BluetoothScanUpdatedEvent(BluetoothScanUpdatedEventValue {
        kind: String::from("updated"),
        metadata: next_scan_metadata(state, device),
    })
}

/// Build one lost bluetooth scan event.
#[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
pub(super) fn scan_lost_event(
    state: &Mutex<BluetoothEventState>,
    device: BluetoothDeviceDescriptorValue,
) -> BluetoothScanEventValue {
    BluetoothScanEventValue::BluetoothScanLostEvent(BluetoothScanLostEventValue {
        kind: String::from("lost"),
        metadata: next_scan_metadata(state, device),
    })
}

/// Build one bluetooth adapter attached event.
pub(super) fn adapter_attached_event(
    state: &Mutex<BluetoothEventState>,
    adapter: BluetoothAdapterDescriptorValue,
) -> BluetoothAdapterEventValue {
    BluetoothAdapterEventValue::BluetoothAdapterAttachedEvent(BluetoothAdapterAttachedEventValue {
        kind: String::from("attached"),
        metadata: next_adapter_metadata(state, adapter),
    })
}

/// Build one bluetooth adapter changed event.
pub(super) fn adapter_changed_event(
    state: &Mutex<BluetoothEventState>,
    adapter: BluetoothAdapterDescriptorValue,
) -> BluetoothAdapterEventValue {
    BluetoothAdapterEventValue::BluetoothAdapterChangedEvent(BluetoothAdapterChangedEventValue {
        kind: String::from("changed"),
        metadata: next_adapter_metadata(state, adapter),
    })
}

/// Build one bluetooth adapter detached event.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) fn adapter_detached_event(
    state: &Mutex<BluetoothEventState>,
    adapter: BluetoothAdapterDescriptorValue,
) -> BluetoothAdapterEventValue {
    BluetoothAdapterEventValue::BluetoothAdapterDetachedEvent(BluetoothAdapterDetachedEventValue {
        kind: String::from("detached"),
        metadata: next_adapter_metadata(state, adapter),
    })
}

/// Build one bluetooth disconnected event.
pub(super) fn disconnected_event(state: &Mutex<BluetoothEventState>) -> BluetoothSessionEventValue {
    BluetoothSessionEventValue::BluetoothSessionDisconnectedEvent(
        BluetoothSessionDisconnectedEventValue {
            kind: String::from("disconnected"),
            metadata: next_session_metadata(state),
        },
    )
}

/// Build one bluetooth pair-state-changed event.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) fn pair_state_changed_event(
    state: &Mutex<BluetoothEventState>,
    pair_state: BluetoothPairState,
) -> BluetoothSessionEventValue {
    BluetoothSessionEventValue::BluetoothSessionPairStateChangedEvent(
        BluetoothSessionPairStateChangedEventValue {
            kind: String::from("pairStateChanged"),
            metadata: next_session_metadata(state),
            pair_state,
        },
    )
}

/// Build one bluetooth GATT-database-changed event.
pub(super) fn gatt_database_changed_event(
    state: &Mutex<BluetoothEventState>,
) -> BluetoothSessionEventValue {
    BluetoothSessionEventValue::BluetoothSessionGattDatabaseChangedEvent(
        BluetoothSessionGattDatabaseChangedEventValue {
            kind: String::from("gattDatabaseChanged"),
            metadata: next_session_metadata(state),
        },
    )
}

/// Build one bluetooth value event.
pub(super) fn gatt_value_event(
    timestamp_ns: u64,
    service_id: String,
    characteristic_id: String,
    service_uuid: String,
    characteristic_uuid: String,
    value: Vec<u8>,
) -> BluetoothGattValueEventValue {
    BluetoothGattValueEventValue {
        timestamp_ns,
        service_id,
        characteristic_id,
        service_uuid,
        characteristic_uuid,
        value,
    }
}

/// Encode one bluetooth scan event for the binding surface.
pub(super) fn stored_scan_event(
    binding: &BindingCallContext,
    value: BluetoothScanEventValue,
) -> BluetoothScanEvent {
    <BluetoothScanEvent as NativeAbiCodec>::from_value(binding, value)
}

/// Encode one bluetooth adapter event for the binding surface.
pub(super) fn stored_adapter_event(
    binding: &BindingCallContext,
    value: BluetoothAdapterEventValue,
) -> BluetoothAdapterEvent {
    <BluetoothAdapterEvent as NativeAbiCodec>::from_value(binding, value)
}

/// Encode one bluetooth session event for the binding surface.
pub(super) fn stored_session_event(
    binding: &BindingCallContext,
    value: BluetoothSessionEventValue,
) -> BluetoothSessionEvent {
    <BluetoothSessionEvent as NativeAbiCodec>::from_value(binding, value)
}

/// Encode one bluetooth value event for the binding surface.
pub(super) fn stored_gatt_value_event(
    binding: &BindingCallContext,
    value: BluetoothGattValueEventValue,
) -> BluetoothGattValueEvent {
    <BluetoothGattValueEvent as NativeAbiCodec>::from_value(binding, value)
}

/// Return whether one binary payload matches one filter.
pub(super) fn matches_data_filter(data: &[u8], filter: &BluetoothDataFilterValue) -> bool {
    if data.len() < filter.data_prefix.len() {
        return false;
    }

    for (index, expected) in filter.data_prefix.iter().enumerate() {
        let actual = data[index];
        let mask = filter
            .mask
            .as_ref()
            .and_then(|mask| mask.get(index))
            .copied()
            .unwrap_or(0xff);
        if (actual & mask) != (*expected & mask) {
            return false;
        }
    }

    true
}

/// Return whether one device descriptor matches one scan filter.
pub(super) fn matches_scan_filter(
    descriptor: &BluetoothDeviceDescriptorValue,
    filter: &BluetoothScanFilterValue,
) -> bool {
    if !filter.service_uuids.is_empty()
        && !filter.service_uuids.iter().all(|uuid| {
            descriptor
                .advertisement
                .service_uuids
                .iter()
                .any(|candidate| candidate == uuid)
        })
    {
        return false;
    }

    if let Some(name) = filter.name.as_ref()
        && descriptor.name.as_ref() != Some(name)
    {
        return false;
    }

    if let Some(prefix) = filter.name_prefix.as_ref() {
        let Some(name) = descriptor.name.as_ref() else {
            return false;
        };
        if !name.starts_with(prefix) {
            return false;
        }
    }

    if let Some(minimum_rssi) = filter.minimum_rssi
        && descriptor.rssi.unwrap_or(i32::MIN) < minimum_rssi
    {
        return false;
    }

    for expected in &filter.manufacturer_data {
        let Some(actual) = descriptor
            .advertisement
            .manufacturer_data
            .iter()
            .find(|candidate| candidate.company_id == expected.company_id)
        else {
            return false;
        };

        if let Some(data_filter) = expected.data.as_ref()
            && !matches_data_filter(&actual.data, data_filter)
        {
            return false;
        }
    }

    for expected in &filter.service_data {
        let Some(actual) = descriptor
            .advertisement
            .service_data
            .iter()
            .find(|candidate| candidate.service_uuid == expected.service_uuid)
        else {
            return false;
        };

        if let Some(data_filter) = expected.data.as_ref()
            && !matches_data_filter(&actual.data, data_filter)
        {
            return false;
        }
    }

    true
}

/// Return one fresh bluetooth adapter event metadata payload.
fn next_adapter_metadata(
    state: &Mutex<BluetoothEventState>,
    adapter: BluetoothAdapterDescriptorValue,
) -> BluetoothAdapterEventMetadataValue {
    let mut state = state.lock();
    let sequence = state.next_sequence;
    state.next_sequence = state.next_sequence.saturating_add(1);

    BluetoothAdapterEventMetadataValue {
        timestamp_ns: core_platform::monotonic_now_ns(),
        sequence,
        adapter,
    }
}

/// Return one fresh bluetooth scan event metadata payload.
fn next_scan_metadata(
    state: &Mutex<BluetoothEventState>,
    device: BluetoothDeviceDescriptorValue,
) -> BluetoothScanEventMetadataValue {
    let mut state = state.lock();
    let sequence = state.next_sequence;
    state.next_sequence = state.next_sequence.saturating_add(1);

    BluetoothScanEventMetadataValue {
        timestamp_ns: core_platform::monotonic_now_ns(),
        sequence,
        device,
    }
}

/// Return one fresh bluetooth session event metadata payload.
fn next_session_metadata(state: &Mutex<BluetoothEventState>) -> BluetoothSessionEventMetadata {
    let mut state = state.lock();
    let sequence = state.next_sequence;
    state.next_sequence = state.next_sequence.saturating_add(1);

    BluetoothSessionEventMetadata {
        timestamp_ns: core_platform::monotonic_now_ns(),
        sequence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::device::{
        BluetoothAdvertisementDataValue, BluetoothAdvertisementManufacturerDataValue,
        BluetoothAdvertisementServiceDataValue, BluetoothLeTransport,
        BluetoothManufacturerDataFilterValue, BluetoothServiceDataFilterValue,
    };

    /// Build one descriptor fixture for shared bluetooth filter tests.
    fn test_device_descriptor() -> BluetoothDeviceDescriptorValue {
        BluetoothDeviceDescriptorValue {
            id: String::from("device.1"),
            address: Some(String::from("01:23:45:67:89:ab")),
            name: Some(String::from("Destack Controller")),
            rssi: Some(-42),
            pair_state: None,
            connected: false,
            connectable: Some(true),
            transport: Some(BluetoothLeTransport::LowEnergy),
            advertisement: BluetoothAdvertisementDataValue {
                local_name: Some(String::from("Destack Controller")),
                tx_power: Some(-4),
                service_uuids: vec![String::from("180d"), String::from("1234")],
                manufacturer_data: vec![BluetoothAdvertisementManufacturerDataValue {
                    company_id: 0x1337,
                    data: vec![0xaa, 0xb5, 0x10, 0x20],
                }],
                service_data: vec![BluetoothAdvertisementServiceDataValue {
                    service_uuid: String::from("1234"),
                    data: vec![0xde, 0xad, 0xbe, 0xef],
                }],
            },
        }
    }

    /// Build one filter fixture that exercises the full shared matcher.
    fn test_scan_filter() -> BluetoothScanFilterValue {
        BluetoothScanFilterValue {
            service_uuids: vec![String::from("1234")],
            name: Some(String::from("Destack Controller")),
            name_prefix: Some(String::from("Destack")),
            manufacturer_data: vec![BluetoothManufacturerDataFilterValue {
                company_id: 0x1337,
                data: Some(BluetoothDataFilterValue {
                    data_prefix: vec![0xaa, 0xb0],
                    mask: Some(vec![0xff, 0xf0]),
                }),
            }],
            service_data: vec![BluetoothServiceDataFilterValue {
                service_uuid: String::from("1234"),
                data: Some(BluetoothDataFilterValue {
                    data_prefix: vec![0xde, 0xad],
                    mask: None,
                }),
            }],
            keep_repeated_devices: true,
            minimum_rssi: Some(-60),
            scan_mode: None,
            primary_phy: None,
            secondary_phy: None,
        }
    }

    /// Match one descriptor only when every shared scan filter clause is satisfied.
    #[test]
    fn test_matches_scan_filter_requires_full_match() {
        let descriptor = test_device_descriptor();
        let filter = test_scan_filter();

        // full match
        assert!(matches_scan_filter(&descriptor, &filter));

        // manufacturer mismatch
        let mut mismatched_manufacturer = descriptor.clone();
        mismatched_manufacturer.advertisement.manufacturer_data[0].data[1] = 0x01;
        assert!(!matches_scan_filter(&mismatched_manufacturer, &filter));

        // service data mismatch
        let mut mismatched_service_data = descriptor.clone();
        mismatched_service_data.advertisement.service_data[0].data[0] = 0x00;
        assert!(!matches_scan_filter(&mismatched_service_data, &filter));

        // rssi mismatch
        let mut mismatched_rssi = descriptor;
        mismatched_rssi.rssi = Some(-120);
        assert!(!matches_scan_filter(&mismatched_rssi, &filter));
    }

    /// Increment scan event sequence numbers for each emitted variant.
    #[test]
    fn test_scan_event_sequence_increments() {
        let state = Mutex::new(BluetoothEventState { next_sequence: 7 });
        let descriptor = test_device_descriptor();

        // first discovered event
        let discovered = scan_discovered_event(&state, descriptor.clone());
        let BluetoothScanEventValue::BluetoothScanDiscoveredEvent(discovered) = discovered else {
            panic!("expected discovered scan event");
        };
        assert_eq!(discovered.metadata.sequence, 7);

        // second updated event
        let updated = scan_updated_event(&state, descriptor);
        let BluetoothScanEventValue::BluetoothScanUpdatedEvent(updated) = updated else {
            panic!("expected updated scan event");
        };
        assert_eq!(updated.metadata.sequence, 8);
    }

    /// Increment session event sequence numbers for each emitted variant.
    #[test]
    fn test_session_event_sequence_increments() {
        let state = Mutex::new(BluetoothEventState { next_sequence: 3 });

        // first disconnect event
        let disconnected = disconnected_event(&state);
        let BluetoothSessionEventValue::BluetoothSessionDisconnectedEvent(disconnected) =
            disconnected
        else {
            panic!("expected disconnected session event");
        };
        assert_eq!(disconnected.metadata.sequence, 3);

        // second database-change event
        let database_changed = gatt_database_changed_event(&state);
        let BluetoothSessionEventValue::BluetoothSessionGattDatabaseChangedEvent(database_changed) =
            database_changed
        else {
            panic!("expected gatt database changed session event");
        };
        assert_eq!(database_changed.metadata.sequence, 4);
    }
}
