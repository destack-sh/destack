use crate::platform::device::tests::{assert_supported_or_not_supported, with_harness_context};
use crate::platform::device::{UsbConfigurationDescriptorValue, UsbDeviceDescriptorValue};

use super::core::{assert_usb_descriptor_shape, usb_descriptor_list_value};

/// Open one listed USB device, query descriptor state, and close it again.
#[test]
fn test_device_usb_open_queries_descriptor_and_closes_first_device_when_present() {
    with_harness_context(|mut context| {
        let listed = assert_supported_or_not_supported(context.destack_device_usb_list())?;
        let Some(listed) = listed else {
            return Ok(());
        };

        let listed = usb_descriptor_list_value(&mut context, listed)?;
        let Some(first) = listed.first() else {
            return Ok(());
        };

        // open one listed device and always close it during cleanup
        let id = context.harness_value_from(first.id.clone())?;
        let handle = context.destack_device_usb_open(id)?;
        let result = (|| {
            let descriptor = context.destack_device_usb_descriptor(handle)?;
            let descriptor: UsbDeviceDescriptorValue = context.harness_value_into(descriptor)?;
            assert_eq!(descriptor.id, first.id);
            assert_usb_descriptor_shape(&descriptor);

            let configurations = context.destack_device_usb_configuration_list(handle)?;
            let configurations: Vec<UsbConfigurationDescriptorValue> =
                context.harness_value_into(configurations)?;
            let _ = configurations;

            context.destack_device_usb_transfer_cancel_all(handle)?;

            Ok(())
        })();

        context.destack_device_usb_close(handle)?;
        result
    });
}
