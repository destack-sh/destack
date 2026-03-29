use std::collections::BTreeSet;

use crate::diagnostic::RuntimeResult;
use crate::platform::VmSlice;
use crate::platform::abi::NativeSlice;
use crate::platform::device::tests::{DeviceHarnessContext, HarnessValue};
use crate::platform::device::{
    UsbDeviceDescriptor, UsbDeviceDescriptorValue, UsbDeviceDescriptorVm, UsbHotplugEvent,
    UsbHotplugEventValue, UsbHotplugEventVm,
};

/// Decode one listed USB descriptor slice from one native or VM harness result.
pub(super) fn usb_descriptor_list_value(
    context: &mut DeviceHarnessContext<'_>,
    value: HarnessValue<NativeSlice<UsbDeviceDescriptor>, VmSlice<UsbDeviceDescriptorVm>>,
) -> RuntimeResult<Vec<UsbDeviceDescriptorValue>> {
    context.harness_value_into(value)
}

/// Decode one USB hotplug event from one native or VM harness result.
pub(super) fn usb_hotplug_event_value(
    context: &mut DeviceHarnessContext<'_>,
    value: HarnessValue<UsbHotplugEvent, UsbHotplugEventVm>,
) -> RuntimeResult<UsbHotplugEventValue> {
    context.harness_value_into(value)
}

/// Assert one listed USB descriptor is self-consistent.
pub(super) fn assert_usb_descriptor_invariants(
    descriptor: &UsbDeviceDescriptorValue,
    ids: &mut BTreeSet<String>,
) {
    assert_usb_descriptor_shape(descriptor);

    // stable identity
    assert!(ids.insert(descriptor.id.clone()));
}

/// Assert one USB descriptor shape is self-consistent.
pub(super) fn assert_usb_descriptor_shape(descriptor: &UsbDeviceDescriptorValue) {
    // stable identity
    assert!(!descriptor.id.is_empty());

    // optional strings
    if let Some(manufacturer) = &descriptor.manufacturer {
        assert!(!manufacturer.is_empty());
    }
    if let Some(product) = &descriptor.product {
        assert!(!product.is_empty());
    }
    if let Some(serial_number) = &descriptor.serial_number {
        assert!(!serial_number.is_empty());
    }

    // optional topology metadata
    if let Some(bus_number) = descriptor.bus_number {
        assert!(bus_number > 0);
    }
    if let Some(port_path) = &descriptor.port_path {
        assert!(!port_path.is_empty());
    }
}
