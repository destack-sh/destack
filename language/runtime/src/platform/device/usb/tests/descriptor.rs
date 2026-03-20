use crate::platform::device::usb::descriptor::{
    endpoint_address, request_index, request_type_bits, usb_device_id, validate_control_setup,
    validate_endpoint, validate_endpoint_selector,
};
use crate::platform::device::usb::ffi::ffi;
use crate::platform::device::{
    UsbControlDeviceTargetValue, UsbControlEndpointTargetValue, UsbControlSetupValue,
    UsbControlTargetValue, UsbControlTransferType, UsbEndpointDirection, UsbEndpointSelectorValue,
};

/// Device control-target discriminator.
const USB_CONTROL_DEVICE_TARGET_KIND: &str = "device";

/// Endpoint control-target discriminator.
const USB_CONTROL_ENDPOINT_TARGET_KIND: &str = "endpoint";

/// Encode one setup packet request type from direction, transfer type, and recipient.
#[test]
fn test_request_type_bits_encodes_direction_type_and_recipient() {
    let setup = UsbControlSetupValue {
        transfer_type: UsbControlTransferType::Vendor,
        target: UsbControlTargetValue::UsbControlDeviceTarget(UsbControlDeviceTargetValue {
            kind: String::from(USB_CONTROL_DEVICE_TARGET_KIND),
        }),
        request: 1,
        value: 2,
        length: 3,
    };

    let bits = request_type_bits(&setup, UsbEndpointDirection::In);

    assert_eq!(bits, 0x80 | (0x02 << 5));
}

/// Encode one endpoint target into the control-request index field.
#[test]
fn test_request_index_uses_endpoint_address_for_endpoint_targets() {
    let setup = UsbControlSetupValue {
        transfer_type: UsbControlTransferType::Standard,
        target: UsbControlTargetValue::UsbControlEndpointTarget(UsbControlEndpointTargetValue {
            kind: String::from(USB_CONTROL_ENDPOINT_TARGET_KIND),
            endpoint: UsbEndpointSelectorValue {
                number: 3,
                direction: UsbEndpointDirection::In,
            },
        }),
        request: 1,
        value: 2,
        length: 3,
    };

    assert_eq!(request_index(&setup), 0x83);
}

/// Encode one endpoint address from number and direction.
#[test]
fn test_endpoint_address_encodes_number_and_direction() {
    let endpoint = UsbEndpointSelectorValue {
        number: 9,
        direction: UsbEndpointDirection::Out,
    };

    assert_eq!(endpoint_address(&endpoint), 9);
}

/// Reject one endpoint number that would otherwise be truncated.
#[test]
fn test_validate_endpoint_rejects_endpoint_numbers_above_usb_range() {
    let endpoint = UsbEndpointSelectorValue {
        number: 16,
        direction: UsbEndpointDirection::In,
    };

    let error = validate_endpoint(&endpoint, UsbEndpointDirection::In, "endpoint")
        .expect_err("endpoint numbers above 15 must be rejected");

    assert_eq!(
        error
            .platform_error()
            .expect("endpoint validation should return one platform error")
            .code,
        crate::platform::diagnostic::PlatformErrorCode::InvalidArgumentValue
    );
}

/// Reject endpoint zero because control transfers do not use endpoint selectors.
#[test]
fn test_validate_endpoint_rejects_endpoint_zero() {
    let endpoint = UsbEndpointSelectorValue {
        number: 0,
        direction: UsbEndpointDirection::In,
    };

    let error = validate_endpoint(&endpoint, UsbEndpointDirection::In, "endpoint")
        .expect_err("endpoint zero must be rejected");

    assert_eq!(
        error
            .platform_error()
            .expect("endpoint validation should return one platform error")
            .code,
        crate::platform::diagnostic::PlatformErrorCode::InvalidArgumentValue
    );
}

/// Accept one valid endpoint selector when direction is not operation constrained.
#[test]
fn test_validate_endpoint_selector_accepts_one_valid_endpoint_without_direction_checks() {
    let endpoint = UsbEndpointSelectorValue {
        number: 3,
        direction: UsbEndpointDirection::Out,
    };

    validate_endpoint_selector(&endpoint, "endpoint")
        .expect("endpoint selector validation should accept one valid endpoint");
}

/// Reject endpoints whose direction does not match the transfer operation.
#[test]
fn test_validate_endpoint_rejects_direction_mismatches() {
    let endpoint = UsbEndpointSelectorValue {
        number: 3,
        direction: UsbEndpointDirection::Out,
    };

    let error = validate_endpoint(&endpoint, UsbEndpointDirection::In, "endpoint")
        .expect_err("direction mismatches must be rejected");

    assert_eq!(
        error
            .platform_error()
            .expect("endpoint validation should return one platform error")
            .code,
        crate::platform::diagnostic::PlatformErrorCode::InvalidArgumentValue
    );
}

/// Reject endpoint-recipient control transfers that target endpoint zero.
#[test]
fn test_validate_control_setup_rejects_endpoint_recipient_zero() {
    let setup = UsbControlSetupValue {
        transfer_type: UsbControlTransferType::Standard,
        target: UsbControlTargetValue::UsbControlEndpointTarget(UsbControlEndpointTargetValue {
            kind: String::from(USB_CONTROL_ENDPOINT_TARGET_KIND),
            endpoint: UsbEndpointSelectorValue {
                number: 0,
                direction: UsbEndpointDirection::In,
            },
        }),
        request: 1,
        value: 2,
        length: 3,
    };

    let error = validate_control_setup(&setup)
        .expect_err("endpoint-recipient control transfers must reject endpoint zero");

    assert_eq!(
        error
            .platform_error()
            .expect("control validation should return one platform error")
            .code,
        crate::platform::diagnostic::PlatformErrorCode::InvalidArgumentValue
    );
}

/// Build one stable usb device id from bus, path, and descriptor identity.
#[test]
fn test_usb_device_id_uses_location_and_descriptor_identity() {
    let descriptor = ffi::LibusbDeviceDescriptor {
        b_length: 0,
        b_descriptor_type: 0,
        bcd_usb: 0,
        b_device_class: 0,
        b_device_sub_class: 0,
        b_device_protocol: 0,
        b_max_packet_size0: 0,
        id_vendor: 0x1234,
        id_product: 0xabcd,
        bcd_device: 0,
        i_manufacturer: 0,
        i_product: 0,
        i_serial_number: 0,
        b_num_configurations: 0,
    };

    let id = usb_device_id(&descriptor, 2, &[1, 4]);

    assert_eq!(id, "usb:2:1.4:1234:abcd");
}
