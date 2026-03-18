use crate::platform::device::UsbTransferStatus;
use crate::platform::device::usb::service::transfer_status_from_libusb_result;

/// Map libusb pipe and overflow results into explicit transfer statuses.
#[test]
fn test_transfer_status_from_libusb_result_maps_pipe_and_overflow() {
    assert_eq!(
        transfer_status_from_libusb_result(-9),
        Some(UsbTransferStatus::Stall)
    );
    assert_eq!(
        transfer_status_from_libusb_result(-8),
        Some(UsbTransferStatus::Babble)
    );
}
