#[cfg(target_os = "android")]
mod android;
mod core;
mod descriptor;
mod ffi;
mod operations;
mod service;
mod transfer;

#[cfg(target_os = "android")]
pub(crate) use android::{
    destack_device_usb_list, destack_device_usb_open, destack_device_usb_watch_close,
    destack_device_usb_watch_open, destack_device_usb_watch_read,
    destack_device_usb_watch_try_read,
};
#[cfg(not(target_os = "android"))]
pub(crate) use operations::*;
#[cfg(target_os = "android")]
pub(crate) use operations::{
    destack_device_usb_bos_capability_list, destack_device_usb_bulk_read,
    destack_device_usb_bulk_write, destack_device_usb_claim_interface,
    destack_device_usb_clear_halt, destack_device_usb_close, destack_device_usb_configuration_get,
    destack_device_usb_configuration_list, destack_device_usb_configuration_set,
    destack_device_usb_control_read, destack_device_usb_control_write,
    destack_device_usb_descriptor, destack_device_usb_interrupt_read,
    destack_device_usb_interrupt_write, destack_device_usb_isochronous_read,
    destack_device_usb_isochronous_write, destack_device_usb_release_interface,
    destack_device_usb_reset, destack_device_usb_set_interface_alternate_setting,
    destack_device_usb_string_descriptor, destack_device_usb_string_language_list,
    destack_device_usb_transfer_cancel, destack_device_usb_transfer_cancel_all,
};
pub(crate) use service::{UsbService, usb_service};
