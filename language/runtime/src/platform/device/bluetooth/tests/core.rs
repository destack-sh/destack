use std::collections::BTreeSet;

use crate::diagnostic::RuntimeResult;
use crate::platform::device::tests::{
    DeviceHarnessContext, HarnessValue, assert_ok_or_expected_error,
};
use crate::platform::device::{
    BluetoothAdapterDescriptor, BluetoothAdapterDescriptorValue, BluetoothAdapterDescriptorVm,
    BluetoothAdapterEvent, BluetoothAdapterEventValue, BluetoothAdapterEventVm,
    native as device_native,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{BluetoothDeviceHandle, BluetoothScanHandle};
use crate::platform::{NativeSlice, VmSlice};

/// Assert one Bluetooth result is success, not-supported, or permission-gated.
pub(super) fn assert_bluetooth_supported_not_supported_or_permission<T>(
    result: Result<T, Box<crate::diagnostic::RuntimeError>>,
) -> crate::diagnostic::RuntimeResult<Option<T>> {
    assert_ok_or_expected_error(
        result,
        &[
            PlatformErrorCode::NotSupported,
            PlatformErrorCode::IoPermissionDenied,
        ],
    )
}

/// Decode one listed bluetooth adapter descriptor slice from one native or VM harness result.
pub(super) fn bluetooth_adapter_descriptor_list_value(
    context: &mut DeviceHarnessContext<'_>,
    value: HarnessValue<
        NativeSlice<BluetoothAdapterDescriptor>,
        VmSlice<BluetoothAdapterDescriptorVm>,
    >,
) -> RuntimeResult<Vec<BluetoothAdapterDescriptorValue>> {
    context.harness_value_into(value)
}

/// Decode one bluetooth adapter event from one native or VM harness result.
pub(super) fn bluetooth_adapter_event_value(
    context: &mut DeviceHarnessContext<'_>,
    value: HarnessValue<BluetoothAdapterEvent, BluetoothAdapterEventVm>,
) -> RuntimeResult<BluetoothAdapterEventValue> {
    context.harness_value_into(value)
}

/// Assert one listed bluetooth adapter is self consistent and uniquely identified.
pub(super) fn assert_bluetooth_adapter_descriptor_invariants(
    descriptor: &BluetoothAdapterDescriptorValue,
    ids: &mut BTreeSet<String>,
) {
    assert_bluetooth_adapter_descriptor_shape(descriptor);

    assert!(ids.insert(descriptor.id.clone()));
}

/// Assert one bluetooth adapter descriptor shape is self consistent.
pub(super) fn assert_bluetooth_adapter_descriptor_shape(
    descriptor: &BluetoothAdapterDescriptorValue,
) {
    assert!(!descriptor.id.is_empty());
    assert!(!descriptor.name.is_empty());
}

/// Close one opened bluetooth scan handle during test cleanup.
#[cfg(any(unix, windows))]
pub(super) fn close_bluetooth_scan(
    call_context: &crate::runtime::BindingCallContext,
    handle: BluetoothScanHandle,
) {
    unsafe {
        drop(device_native::destack_device_bluetooth_scan_close(
            call_context,
            handle,
        ));
    }
}

/// Close one opened bluetooth device handle during test cleanup.
#[cfg(any(unix, windows))]
pub(super) fn close_bluetooth_device(
    call_context: &crate::runtime::BindingCallContext,
    handle: BluetoothDeviceHandle,
) {
    unsafe {
        drop(device_native::destack_device_bluetooth_close(
            call_context,
            handle,
        ));
    }
}
