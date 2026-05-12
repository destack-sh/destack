use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::device::BluetoothGattWriteMode;
use crate::platform::device::tests::{assert_ok_or_expected_error, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{
    BluetoothAdapterWatchHandle, BluetoothDeviceHandle, BluetoothScanHandle,
    BluetoothSubscriptionHandle, ResourceId,
};

/// Assert that one bluetooth operation rejects an invalid handle.
fn assert_invalid_handle<T>(result: Result<T, Box<RuntimeError>>) -> RuntimeResult<()> {
    let result = assert_ok_or_expected_error(result, &[PlatformErrorCode::InvalidArgumentValue])?;
    assert!(result.is_none());

    Ok(())
}

/// Assert that one bluetooth operation rejects an invalid handle or reports one unsupported lane.
fn assert_invalid_handle_or_not_supported<T>(
    result: Result<T, Box<RuntimeError>>,
) -> RuntimeResult<()> {
    let result = assert_ok_or_expected_error(
        result,
        &[
            PlatformErrorCode::InvalidArgumentValue,
            PlatformErrorCode::NotSupported,
        ],
    )?;
    assert!(result.is_none());

    Ok(())
}

/// Reject one unknown adapter-watch handle across the full public watch surface.
#[cfg(any(unix, windows))]
#[test]
fn test_device_bluetooth_rejects_unknown_adapter_watch_handles() {
    with_harness_context(|mut context| {
        let unknown_watch = BluetoothAdapterWatchHandle(ResourceId::local(0));

        // reject one invalid adapter-watch close target
        assert_invalid_handle(context.destack_device_bluetooth_adapter_watch_close(unknown_watch))?;

        // reject one invalid blocking adapter-watch read target
        assert_invalid_handle(
            context.destack_device_bluetooth_adapter_watch_read(unknown_watch, 1),
        )?;

        // reject one invalid nonblocking adapter-watch read target
        assert_invalid_handle(
            context.destack_device_bluetooth_adapter_watch_try_read(unknown_watch),
        )?;

        Ok(())
    });
}

/// Reject one unknown scan handle across the full public scan-event surface.
#[cfg(any(unix, windows))]
#[test]
fn test_device_bluetooth_rejects_unknown_scan_handles() {
    with_harness_context(|mut context| {
        let unknown_scan = BluetoothScanHandle(ResourceId::local(0));

        // reject one invalid scan close target
        assert_invalid_handle(context.destack_device_bluetooth_scan_close(unknown_scan))?;

        // reject one invalid blocking scan-event read target
        assert_invalid_handle(context.destack_device_bluetooth_scan_read_event(unknown_scan, 1))?;

        // reject one invalid nonblocking scan-event read target
        assert_invalid_handle(context.destack_device_bluetooth_scan_try_read_event(unknown_scan))?;

        Ok(())
    });
}

/// Reject one unknown device handle across session and GATT operations.
#[cfg(any(unix, windows))]
#[test]
fn test_device_bluetooth_rejects_unknown_device_handles() {
    with_harness_context(|mut context| {
        let unknown_device = BluetoothDeviceHandle(ResourceId::local(0));
        let service_id = context.harness_value_from(String::from("service.1"))?;
        let characteristic_id = context.harness_value_from(String::from("characteristic.1"))?;

        // reject one invalid device close target
        assert_invalid_handle(context.destack_device_bluetooth_close(unknown_device))?;

        // reject one invalid session descriptor target
        assert_invalid_handle(context.destack_device_bluetooth_descriptor(unknown_device))?;

        // reject one invalid session pair target, or one unsupported pair lane on CoreBluetooth
        assert_invalid_handle_or_not_supported(
            context.destack_device_bluetooth_pair(unknown_device, 1),
        )?;

        // reject one invalid blocking session-event read target
        assert_invalid_handle(
            context.destack_device_bluetooth_session_read_event(unknown_device, 1),
        )?;

        // reject one invalid nonblocking session-event read target
        assert_invalid_handle(
            context.destack_device_bluetooth_session_try_read_event(unknown_device),
        )?;

        // reject one invalid RSSI target
        assert_invalid_handle(context.destack_device_bluetooth_read_rssi(unknown_device, 1))?;

        // reject one invalid GATT service-list target
        assert_invalid_handle(context.destack_device_bluetooth_gatt_service_list(unknown_device))?;

        // reject one invalid GATT characteristic-list target
        assert_invalid_handle(
            context.destack_device_bluetooth_gatt_characteristic_list(unknown_device, service_id),
        )?;

        // reject one invalid GATT descriptor-list target
        assert_invalid_handle(
            context
                .destack_device_bluetooth_gatt_descriptor_list(unknown_device, characteristic_id),
        )?;

        // reject one invalid MTU target, or one unsupported MTU lane on CoreBluetooth
        assert_invalid_handle_or_not_supported(
            context.destack_device_bluetooth_gatt_mtu(unknown_device),
        )?;

        // reject one invalid characteristic-read target
        let characteristic_id = context.harness_value_from(String::from("characteristic.1"))?;
        assert_invalid_handle(context.destack_device_bluetooth_gatt_read(
            unknown_device,
            characteristic_id,
            1,
        ))?;

        // reject one invalid descriptor-read target
        let descriptor_id_for_read = context.harness_value_from(String::from("descriptor.1"))?;
        assert_invalid_handle(context.destack_device_bluetooth_gatt_read_descriptor(
            unknown_device,
            descriptor_id_for_read,
            1,
        ))?;

        // reject one invalid characteristic-write target
        let characteristic_id_for_write =
            context.harness_value_from(String::from("characteristic.1"))?;
        let value_for_write = context.harness_value_from(vec![0xde, 0xad])?;
        assert_invalid_handle(context.destack_device_bluetooth_gatt_write(
            unknown_device,
            characteristic_id_for_write,
            value_for_write,
            BluetoothGattWriteMode::WithResponse,
            1,
        ))?;

        // reject one invalid descriptor-write target
        let descriptor_id_for_write = context.harness_value_from(String::from("descriptor.1"))?;
        let value_for_descriptor_write = context.harness_value_from(vec![0xde, 0xad])?;
        assert_invalid_handle(context.destack_device_bluetooth_gatt_write_descriptor(
            unknown_device,
            descriptor_id_for_write,
            value_for_descriptor_write,
            1,
        ))?;

        // reject one invalid subscribe target
        let characteristic_id_for_subscribe =
            context.harness_value_from(String::from("characteristic.1"))?;
        assert_invalid_handle(context.destack_device_bluetooth_gatt_subscribe(
            unknown_device,
            characteristic_id_for_subscribe,
        ))?;

        Ok(())
    });
}

/// Reject one unknown subscription handle across the notification surface.
#[cfg(any(unix, windows))]
#[test]
fn test_device_bluetooth_rejects_unknown_subscription_handles() {
    with_harness_context(|mut context| {
        let unknown_subscription = BluetoothSubscriptionHandle(ResourceId::local(0));

        // reject one invalid unsubscribe target
        assert_invalid_handle(
            context.destack_device_bluetooth_gatt_unsubscribe(unknown_subscription),
        )?;

        // reject one invalid blocking notification read target
        assert_invalid_handle(
            context.destack_device_bluetooth_gatt_read_event(unknown_subscription, 1),
        )?;

        // reject one invalid nonblocking notification read target
        assert_invalid_handle(
            context.destack_device_bluetooth_gatt_try_read_event(unknown_subscription),
        )?;

        Ok(())
    });
}
