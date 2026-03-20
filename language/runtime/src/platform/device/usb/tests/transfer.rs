use crate::platform::device::UsbTransferStatus;
use crate::platform::device::usb::descriptor::{
    timeout_ns_to_millis, transfer_status_from_libusb_async_status,
};
use crate::platform::device::usb::ffi::ffi;
use crate::platform::device::usb::service::transfer_status_from_libusb_result;

/// Map synchronous libusb completion results into explicit transfer statuses.
#[test]
fn test_transfer_status_from_libusb_result_maps_known_completion_codes() {
    assert_eq!(
        transfer_status_from_libusb_result(ffi::LIBUSB_SUCCESS),
        Some(UsbTransferStatus::Ok)
    );
    assert_eq!(
        transfer_status_from_libusb_result(ffi::LIBUSB_ERROR_PIPE),
        Some(UsbTransferStatus::Stall)
    );
    assert_eq!(
        transfer_status_from_libusb_result(ffi::LIBUSB_ERROR_OVERFLOW),
        Some(UsbTransferStatus::Babble)
    );
    assert_eq!(
        transfer_status_from_libusb_result(ffi::LIBUSB_ERROR_TIMEOUT),
        None
    );
}

/// Map asynchronous libusb completion results into explicit transfer statuses.
#[test]
fn test_transfer_status_from_libusb_async_status_maps_known_completion_codes() {
    assert_eq!(
        transfer_status_from_libusb_async_status(ffi::LIBUSB_TRANSFER_COMPLETED),
        Some(UsbTransferStatus::Ok)
    );
    assert_eq!(
        transfer_status_from_libusb_async_status(ffi::LIBUSB_TRANSFER_STALL),
        Some(UsbTransferStatus::Stall)
    );
    assert_eq!(
        transfer_status_from_libusb_async_status(ffi::LIBUSB_TRANSFER_OVERFLOW),
        Some(UsbTransferStatus::Babble)
    );
    assert_eq!(
        transfer_status_from_libusb_async_status(ffi::LIBUSB_TRANSFER_TIMED_OUT),
        None
    );
}

/// Round transfer timeouts up to the host millisecond granularity without overflowing.
#[test]
fn test_timeout_ns_to_millis_rounds_up_and_clamps() {
    assert_eq!(timeout_ns_to_millis(0), 1);
    assert_eq!(timeout_ns_to_millis(1), 1);
    assert_eq!(timeout_ns_to_millis(1_000_000), 1);
    assert_eq!(timeout_ns_to_millis(1_000_001), 2);
    assert_eq!(timeout_ns_to_millis(u64::MAX), u32::MAX);
}
