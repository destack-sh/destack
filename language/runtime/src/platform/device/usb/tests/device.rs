use crate::diagnostic::RuntimeResult;
use crate::platform::VmSlice;
use crate::platform::abi::NativeSlice;
use crate::platform::device::tests::{
    DeviceHarnessContext, HarnessValue, assert_ok_or_expected_error,
    assert_supported_or_not_supported, with_harness_context,
};
use crate::platform::device::{
    UsbConfigurationDescriptorValue, UsbControlDeviceTargetValue, UsbControlSetup,
    UsbControlSetupValue, UsbControlSetupVm, UsbControlTargetValue, UsbControlTransferType,
    UsbDeviceDescriptorValue, UsbEndpointDirection, UsbEndpointSelector, UsbEndpointSelectorValue,
    UsbEndpointSelectorVm,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{ResourceId, UsbDeviceHandle, UsbWatchHandle};

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

/// Reject one unknown USB device and watch handle consistently.
#[test]
fn test_device_usb_rejects_unknown_handles() {
    with_harness_context(|mut context| {
        let unknown_device = UsbDeviceHandle(ResourceId::local(0));
        let unknown_watch = UsbWatchHandle(ResourceId::local(0));

        // reject every public device lane with one invalid handle
        assert_invalid_usb_device_handle(&mut context, unknown_device)?;

        // reject every public watch lane with one invalid handle
        assert_invalid_usb_watch_handle(&mut context, unknown_watch)?;

        Ok(())
    });
}

/// Reject one invalid USB device handle across the full public surface.
fn assert_invalid_usb_device_handle(
    context: &mut DeviceHarnessContext<'_>,
    handle: UsbDeviceHandle,
) -> RuntimeResult<()> {
    let expected = [PlatformErrorCode::InvalidArgumentValue];

    // descriptor and configuration lanes
    let close_result = context.destack_device_usb_close(handle);
    let close_result = assert_ok_or_expected_error(close_result, &expected)?;
    assert!(close_result.is_none());

    let descriptor_result = context.destack_device_usb_descriptor(handle);
    let descriptor_result = assert_ok_or_expected_error(descriptor_result, &expected)?;
    assert!(descriptor_result.is_none());

    let string_languages_result = context.destack_device_usb_string_language_list(handle);
    let string_languages_result = assert_ok_or_expected_error(string_languages_result, &expected)?;
    assert!(string_languages_result.is_none());

    let string_descriptor_result = context.destack_device_usb_string_descriptor(handle, 0x0409);
    let string_descriptor_result =
        assert_ok_or_expected_error(string_descriptor_result, &expected)?;
    assert!(string_descriptor_result.is_none());

    let bos_capabilities_result = context.destack_device_usb_bos_capability_list(handle);
    let bos_capabilities_result = assert_ok_or_expected_error(bos_capabilities_result, &expected)?;
    assert!(bos_capabilities_result.is_none());

    let configuration_list_result = context.destack_device_usb_configuration_list(handle);
    let configuration_list_result =
        assert_ok_or_expected_error(configuration_list_result, &expected)?;
    assert!(configuration_list_result.is_none());

    let configuration_get_result = context.destack_device_usb_configuration_get(handle);
    let configuration_get_result =
        assert_ok_or_expected_error(configuration_get_result, &expected)?;
    assert!(configuration_get_result.is_none());

    let configuration_set_result = context.destack_device_usb_configuration_set(handle, 1);
    let configuration_set_result =
        assert_ok_or_expected_error(configuration_set_result, &expected)?;
    assert!(configuration_set_result.is_none());

    // interface control lanes
    let claim_result = context.destack_device_usb_claim_interface(handle, 1);
    let claim_result = assert_ok_or_expected_error(claim_result, &expected)?;
    assert!(claim_result.is_none());

    let release_result = context.destack_device_usb_release_interface(handle, 1);
    let release_result = assert_ok_or_expected_error(release_result, &expected)?;
    assert!(release_result.is_none());

    let alternate_setting_result =
        context.destack_device_usb_set_interface_alternate_setting(handle, 1, 1);
    let alternate_setting_result =
        assert_ok_or_expected_error(alternate_setting_result, &expected)?;
    assert!(alternate_setting_result.is_none());

    // control transfer lanes
    let control_read_setup = usb_control_setup_argument(context, default_usb_control_setup())?;
    let control_read_result =
        context.destack_device_usb_control_read(handle, control_read_setup, 0);
    let control_read_result = assert_ok_or_expected_error(control_read_result, &expected)?;
    assert!(control_read_result.is_none());

    let control_write_setup = usb_control_setup_argument(context, default_usb_control_setup())?;
    let control_write_bytes = usb_bytes_argument(context, &[1, 2, 3, 4])?;
    let control_write_result = context.destack_device_usb_control_write(
        handle,
        control_write_setup,
        control_write_bytes,
        0,
    );
    let control_write_result = assert_ok_or_expected_error(control_write_result, &expected)?;
    assert!(control_write_result.is_none());

    // endpoint transfer lanes
    let endpoint_in = usb_endpoint_argument(
        context,
        UsbEndpointSelectorValue {
            number: 1,
            direction: UsbEndpointDirection::In,
        },
    )?;
    let bulk_read_result = context.destack_device_usb_bulk_read(handle, endpoint_in, 64, 0);
    let bulk_read_result = assert_ok_or_expected_error(bulk_read_result, &expected)?;
    assert!(bulk_read_result.is_none());

    let endpoint_out = usb_endpoint_argument(
        context,
        UsbEndpointSelectorValue {
            number: 1,
            direction: UsbEndpointDirection::Out,
        },
    )?;
    let bulk_write_bytes = usb_bytes_argument(context, &[1, 2, 3, 4])?;
    let bulk_write_result =
        context.destack_device_usb_bulk_write(handle, endpoint_out, bulk_write_bytes, 0);
    let bulk_write_result = assert_ok_or_expected_error(bulk_write_result, &expected)?;
    assert!(bulk_write_result.is_none());

    let endpoint_in = usb_endpoint_argument(
        context,
        UsbEndpointSelectorValue {
            number: 1,
            direction: UsbEndpointDirection::In,
        },
    )?;
    let interrupt_read_result =
        context.destack_device_usb_interrupt_read(handle, endpoint_in, 64, 0);
    let interrupt_read_result = assert_ok_or_expected_error(interrupt_read_result, &expected)?;
    assert!(interrupt_read_result.is_none());

    let endpoint_out = usb_endpoint_argument(
        context,
        UsbEndpointSelectorValue {
            number: 1,
            direction: UsbEndpointDirection::Out,
        },
    )?;
    let interrupt_write_bytes = usb_bytes_argument(context, &[1, 2, 3, 4])?;
    let interrupt_write_result =
        context.destack_device_usb_interrupt_write(handle, endpoint_out, interrupt_write_bytes, 0);
    let interrupt_write_result = assert_ok_or_expected_error(interrupt_write_result, &expected)?;
    assert!(interrupt_write_result.is_none());

    let endpoint_in = usb_endpoint_argument(
        context,
        UsbEndpointSelectorValue {
            number: 1,
            direction: UsbEndpointDirection::In,
        },
    )?;
    let packet_sizes = usb_packet_sizes_argument(context, &[8, 8])?;
    let isochronous_read_result =
        context.destack_device_usb_isochronous_read(handle, endpoint_in, packet_sizes, 0);
    let isochronous_read_result = assert_ok_or_expected_error(isochronous_read_result, &expected)?;
    assert!(isochronous_read_result.is_none());

    let endpoint_out = usb_endpoint_argument(
        context,
        UsbEndpointSelectorValue {
            number: 1,
            direction: UsbEndpointDirection::Out,
        },
    )?;
    let isochronous_write_bytes = usb_bytes_argument(context, &[1, 2, 3, 4])?;
    let packet_sizes = usb_packet_sizes_argument(context, &[2, 2])?;
    let isochronous_write_result = context.destack_device_usb_isochronous_write(
        handle,
        endpoint_out,
        isochronous_write_bytes,
        packet_sizes,
        0,
    );
    let isochronous_write_result =
        assert_ok_or_expected_error(isochronous_write_result, &expected)?;
    assert!(isochronous_write_result.is_none());

    // transfer management lanes
    let endpoint_out = usb_endpoint_argument(
        context,
        UsbEndpointSelectorValue {
            number: 1,
            direction: UsbEndpointDirection::Out,
        },
    )?;
    let transfer_cancel_result = context.destack_device_usb_transfer_cancel(handle, endpoint_out);
    let transfer_cancel_result = assert_ok_or_expected_error(transfer_cancel_result, &expected)?;
    assert!(transfer_cancel_result.is_none());

    let transfer_cancel_all_result = context.destack_device_usb_transfer_cancel_all(handle);
    let transfer_cancel_all_result =
        assert_ok_or_expected_error(transfer_cancel_all_result, &expected)?;
    assert!(transfer_cancel_all_result.is_none());

    let endpoint_out = usb_endpoint_argument(
        context,
        UsbEndpointSelectorValue {
            number: 1,
            direction: UsbEndpointDirection::Out,
        },
    )?;
    let clear_halt_result = context.destack_device_usb_clear_halt(handle, endpoint_out);
    let clear_halt_result = assert_ok_or_expected_error(clear_halt_result, &expected)?;
    assert!(clear_halt_result.is_none());

    let reset_result = context.destack_device_usb_reset(handle);
    let reset_result = assert_ok_or_expected_error(reset_result, &expected)?;
    assert!(reset_result.is_none());

    Ok(())
}

/// Reject one invalid USB watch handle across the full public watch surface.
fn assert_invalid_usb_watch_handle(
    context: &mut DeviceHarnessContext<'_>,
    handle: UsbWatchHandle,
) -> RuntimeResult<()> {
    let expected = [PlatformErrorCode::InvalidArgumentValue];

    let watch_close_result = context.destack_device_usb_watch_close(handle);
    let watch_close_result = assert_ok_or_expected_error(watch_close_result, &expected)?;
    assert!(watch_close_result.is_none());

    let watch_read_result = context.destack_device_usb_watch_read(handle, 0);
    let watch_read_result = assert_ok_or_expected_error(watch_read_result, &expected)?;
    assert!(watch_read_result.is_none());

    let watch_try_read_result = context.destack_device_usb_watch_try_read(handle);
    let watch_try_read_result = assert_ok_or_expected_error(watch_try_read_result, &expected)?;
    assert!(watch_try_read_result.is_none());

    Ok(())
}

/// Build one default control setup payload for invalid-handle tests.
fn default_usb_control_setup() -> UsbControlSetupValue {
    UsbControlSetupValue {
        transfer_type: UsbControlTransferType::Vendor,
        target: UsbControlTargetValue::UsbControlDeviceTarget(UsbControlDeviceTargetValue {
            kind: String::from("device"),
        }),
        request: 1,
        value: 2,
        length: 4,
    }
}

/// Encode one USB control setup payload for one harness call.
fn usb_control_setup_argument(
    context: &mut DeviceHarnessContext<'_>,
    setup: UsbControlSetupValue,
) -> RuntimeResult<HarnessValue<UsbControlSetup, UsbControlSetupVm>> {
    context.harness_value_from(setup)
}

/// Encode one USB endpoint selector for one harness call.
fn usb_endpoint_argument(
    context: &mut DeviceHarnessContext<'_>,
    endpoint: UsbEndpointSelectorValue,
) -> RuntimeResult<HarnessValue<UsbEndpointSelector, UsbEndpointSelectorVm>> {
    context.harness_value_from(endpoint)
}

/// Encode one byte payload for one harness call.
fn usb_bytes_argument(
    context: &mut DeviceHarnessContext<'_>,
    bytes: &[u8],
) -> RuntimeResult<HarnessValue<NativeSlice<u8>, VmSlice<u8>>> {
    context.harness_value_from::<NativeSlice<u8>, VmSlice<u8>>(bytes.to_vec())
}

/// Encode one packet-size payload for one harness call.
fn usb_packet_sizes_argument(
    context: &mut DeviceHarnessContext<'_>,
    packet_sizes: &[u32],
) -> RuntimeResult<HarnessValue<NativeSlice<u32>, VmSlice<u32>>> {
    context.harness_value_from::<NativeSlice<u32>, VmSlice<u32>>(packet_sizes.to_vec())
}
