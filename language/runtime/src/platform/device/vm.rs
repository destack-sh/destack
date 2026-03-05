#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::device::*;
use crate::platform::{PlatformError, VmArray, VmSlice, fs, resource};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// Pair one Bluetooth device.
///
/// Pair one opened Bluetooth device session with host bonding APIs.
///
/// # Platform
/// Unix and Windows.
/// Uses host pairing and bonding APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `device.bluetooth.connect`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_device_bluetooth_pair(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BluetoothDeviceHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.session.pair is not available in the VM yet",
    ))
    .boxed())
}

/// Read link RSSI for one Bluetooth device session.
///
/// Read current received signal strength indicator for one connected device session.
///
/// # Platform
/// Unix and Windows.
/// Uses host Bluetooth RSSI query APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `device.bluetooth.connect`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_bluetooth_read_rssi(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BluetoothDeviceHandle,
    timeoutns: u64,
) -> RuntimeResult<i32> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.session.rssi is not available in the VM yet",
    ))
    .boxed())
}

/// Remove one Bluetooth device bond.
///
/// Remove host bond state for one Bluetooth device on one adapter.
///
/// # Platform
/// Unix and Windows.
/// Uses host unpair or remove-device APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.bluetooth.connect`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_device_bluetooth_unpair(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    adapterid: vm::StringHandle,
    deviceid: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (adapterid, deviceid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.session.unpair is not available in the VM yet",
    ))
    .boxed())
}

/// List supported stream configurations for one opened camera endpoint.
///
/// Enumerate host camera stream configurations for one opened endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses backend camera format and frame-rate enumeration APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `device.camera.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_camera_device_stream_config_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CameraDeviceHandle,
) -> RuntimeResult<VmSlice<CameraStreamConfigVm>> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.camera.device.streamConfigList is not available in the VM yet",
    ))
    .boxed())
}

/// List supported stream capabilities for one opened camera endpoint.
///
/// Enumerate host camera stream capability descriptors for one opened endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses backend camera capability enumeration APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `device.camera.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_camera_device_stream_capability_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CameraDeviceHandle,
) -> RuntimeResult<VmSlice<CameraStreamCapabilityVm>> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.camera.device.streamCapabilityList is not available in the VM yet",
    ))
    .boxed())
}

/// Read one camera control range.
///
/// Read one normalized camera control range descriptor for one opened stream.
///
/// # Platform
/// Unix and Windows.
/// Uses backend camera control capability query APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `device.camera.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_camera_stream_control_range(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CameraStreamHandle,
    control: CameraControl,
) -> RuntimeResult<CameraControlRangeVm> {
    let _ = (handle, control);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.camera.stream.controlRange is not available in the VM yet",
    ))
    .boxed())
}

/// Read one camera control value.
///
/// Read one normalized camera control value from one running stream.
///
/// # Platform
/// Unix and Windows.
/// Uses backend camera control query APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `device.camera.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_camera_stream_get_control(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CameraStreamHandle,
    control: CameraControl,
) -> RuntimeResult<f64> {
    let _ = (handle, control);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.camera.stream.getControl is not available in the VM yet",
    ))
    .boxed())
}

/// Read camera exposure mode.
///
/// Read one exposure mode from one opened camera stream.
///
/// # Platform
/// Unix and Windows.
/// Uses backend camera exposure mode query APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `device.camera.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_camera_stream_exposure_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraExposureMode> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.camera.stream.exposureMode is not available in the VM yet",
    ))
    .boxed())
}

/// Set camera exposure mode.
///
/// Apply one exposure mode on one opened camera stream.
///
/// # Platform
/// Unix and Windows.
/// Uses backend camera exposure mode APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.camera.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_camera_stream_set_exposure_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CameraStreamHandle,
    mode: CameraExposureMode,
) -> RuntimeResult<()> {
    let _ = (handle, mode);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.camera.stream.setExposureMode is not available in the VM yet",
    ))
    .boxed())
}

/// Set camera stabilization mode.
///
/// Apply one stabilization mode on one opened camera stream.
///
/// # Platform
/// Unix and Windows.
/// Uses backend stabilization APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.camera.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_camera_stream_set_stabilization_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CameraStreamHandle,
    mode: CameraStabilizationMode,
) -> RuntimeResult<()> {
    let _ = (handle, mode);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.camera.stream.setStabilizationMode is not available in the VM yet",
    ))
    .boxed())
}

/// Set camera torch mode.
///
/// Apply one torch mode on one opened camera stream.
///
/// # Platform
/// Unix and Windows.
/// Uses backend torch APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.camera.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_camera_stream_set_torch_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CameraStreamHandle,
    mode: CameraTorchMode,
) -> RuntimeResult<()> {
    let _ = (handle, mode);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.camera.stream.setTorchMode is not available in the VM yet",
    ))
    .boxed())
}

/// Read camera stabilization mode.
///
/// Read one stabilization mode from one opened camera stream.
///
/// # Platform
/// Unix and Windows.
/// Uses backend stabilization mode query APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `device.camera.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_camera_stream_stabilization_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraStabilizationMode> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.camera.stream.stabilizationMode is not available in the VM yet",
    ))
    .boxed())
}

/// Read camera torch mode.
///
/// Read one torch mode from one opened camera stream.
///
/// # Platform
/// Unix and Windows.
/// Uses backend torch mode query APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `device.camera.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_camera_stream_torch_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraTorchMode> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.camera.stream.torchMode is not available in the VM yet",
    ))
    .boxed())
}

/// Discard queued inbound serial bytes.
///
/// Drop queued unread inbound bytes for one opened serial endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses termios and Win32 purge APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.serial.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_serial_discard_input(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.serial.discardInput is not available in the VM yet",
    ))
    .boxed())
}

/// Discard queued outbound serial bytes.
///
/// Drop queued unwritten outbound bytes for one opened serial endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses termios and Win32 purge APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.serial.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_serial_discard_output(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.serial.discardOutput is not available in the VM yet",
    ))
    .boxed())
}

/// Wait for one serial event.
///
/// Wait for one pending serial event for one opened endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses backend serial event queues where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `device.serial.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_serial_read_event(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SerialPortHandle,
    timeoutns: u64,
) -> RuntimeResult<SerialEventVm> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.serial.readEvent is not available in the VM yet",
    ))
    .boxed())
}

/// Set serial break state.
///
/// Apply break signaling state for one opened serial endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses backend serial break-control APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.serial.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_serial_set_break(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SerialPortHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.serial.setBreak is not available in the VM yet",
    ))
    .boxed())
}

/// Clear halt condition on one endpoint.
///
/// Clear one endpoint STALL condition for one opened USB device.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB clear-halt endpoint APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.usb.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_clear_halt(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
    endpointaddress: u8,
) -> RuntimeResult<()> {
    let _ = (handle, endpointaddress);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.clearHalt is not available in the VM yet",
    ))
    .boxed())
}

/// Query whether kernel driver is active on one interface.
///
/// Query host kernel-driver attachment state for one interface.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB kernel-driver query APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `device.usb.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_kernel_driver_active(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
    interfacenumber: u8,
) -> RuntimeResult<bool> {
    let _ = (handle, interfacenumber);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.kernelDriverActive is not available in the VM yet",
    ))
    .boxed())
}

/// Attach kernel driver to one interface.
///
/// Reattach one host kernel driver to one interface where supported.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB kernel-driver attach APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.usb.control`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_device_usb_kernel_driver_attach(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
    interfacenumber: u8,
) -> RuntimeResult<()> {
    let _ = (handle, interfacenumber);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.kernelDriverAttach is not available in the VM yet",
    ))
    .boxed())
}

/// Detach kernel driver from one interface.
///
/// Detach one host kernel driver from one interface before userspace claiming where supported.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB kernel-driver detach APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.usb.control`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_device_usb_kernel_driver_detach(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
    interfacenumber: u8,
) -> RuntimeResult<()> {
    let _ = (handle, interfacenumber);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.kernelDriverDetach is not available in the VM yet",
    ))
    .boxed())
}

/// Reset one USB device.
///
/// Request one bus-level reset for one opened USB device.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB reset-device APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.usb.control`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_device_usb_reset(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.reset is not available in the VM yet",
    ))
    .boxed())
}

/// Set USB interface alternate setting.
///
/// Select one alternate setting for one claimed interface.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB alternate-setting APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.usb.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_set_interface_alternate_setting(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
    interfacenumber: u8,
    alternatesetting: u8,
) -> RuntimeResult<()> {
    let _ = (handle, interfacenumber, alternatesetting);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.setInterfaceAlternateSetting is not available in the VM yet",
    ))
    .boxed())
}

/// Read USB string descriptors.
///
/// Read manufacturer, product, and serial-number string descriptors for one language identifier.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB string-descriptor read APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `device.usb.enumerate`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_string_descriptor(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
    languageid: u16,
) -> RuntimeResult<UsbStringDescriptorVm> {
    let _ = (handle, languageid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.stringDescriptor is not available in the VM yet",
    ))
    .boxed())
}

/// List USB string-descriptor language identifiers.
///
/// Enumerate string-descriptor language identifiers for one opened USB device.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB string-descriptor language query APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `device.usb.enumerate`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_string_language_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<VmSlice<u16>> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.stringLanguageList is not available in the VM yet",
    ))
    .boxed())
}

/// Cancel pending transfers on one endpoint.
///
/// Cancel pending USB transfers for one endpoint on one opened USB device.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB transfer-cancellation APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `device.usb.transfer`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_device_usb_transfer_cancel(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
    endpointaddress: u8,
) -> RuntimeResult<()> {
    let _ = (handle, endpointaddress);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.transferCancel is not available in the VM yet",
    ))
    .boxed())
}

/// Cancel all pending transfers on one opened USB device.
///
/// Cancel pending USB transfers on all endpoints for one opened USB device.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB transfer-cancellation APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `device.usb.transfer`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_device_usb_transfer_cancel_all(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.transferCancelAll is not available in the VM yet",
    ))
    .boxed())
}

/// Close USB hotplug watch stream.
///
/// Close one USB hotplug watch stream and release host subscription resources.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB hotplug unsubscription APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.usb.enumerate`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_watch_close(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbWatchHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.watchClose is not available in the VM yet",
    ))
    .boxed())
}

/// Open USB hotplug watch stream.
///
/// Open one USB hotplug watch stream for attach and detach events.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB hotplug subscription APIs.
///
/// # Errors
/// Returns ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.usb.enumerate`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_watch_open(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<resource::UsbWatchHandle> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.watchOpen is not available in the VM yet",
    ))
    .boxed())
}

/// Wait for one USB hotplug event.
///
/// Wait for one queued USB hotplug event from one opened watch stream.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB hotplug event queues.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `device.usb.enumerate`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_watch_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<UsbHotplugEventVm> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.watchRead is not available in the VM yet",
    ))
    .boxed())
}

/// Poll one USB hotplug event without blocking.
///
/// Poll one queued USB hotplug event from one opened watch stream without waiting.
///
/// # Platform
/// Unix and Windows.
/// Uses host nonblocking USB hotplug event queue reads.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.usb.enumerate`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_watch_try_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbWatchHandle,
) -> RuntimeResult<UsbHotplugEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.watchTryRead is not available in the VM yet",
    ))
    .boxed())
}

/// List Bluetooth adapters.
///
/// Enumerate host Bluetooth adapters and return stable identifiers.
///
/// # Platform
/// Unix and Windows.
/// Uses host Bluetooth adapter enumeration APIs.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `device.bluetooth.scan`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_bluetooth_adapter_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<BluetoothAdapterDescriptorVm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.adapterList is not available in the VM yet",
    ))
    .boxed())
}

/// List discovered GATT characteristics for one service.
///
/// Enumerate GATT characteristics in one selected service.
///
/// # Platform
/// Unix and Windows.
/// Uses host Bluetooth GATT characteristic-discovery APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.bluetooth.gatt`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_bluetooth_gatt_characteristic_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BluetoothDeviceHandle,
    serviceuuid: vm::StringHandle,
) -> RuntimeResult<VmSlice<BluetoothGattCharacteristicDescriptorVm>> {
    let _ = (handle, serviceuuid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.gatt.characteristicList is not available in the VM yet",
    ))
    .boxed())
}

/// List discovered GATT descriptors for one characteristic.
///
/// Enumerate GATT descriptors in one selected service and characteristic.
///
/// # Platform
/// Unix and Windows.
/// Uses host Bluetooth GATT descriptor-discovery APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.bluetooth.gatt`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_bluetooth_gatt_descriptor_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BluetoothDeviceHandle,
    serviceuuid: vm::StringHandle,
    characteristicuuid: vm::StringHandle,
) -> RuntimeResult<VmSlice<BluetoothGattDescriptorDescriptorVm>> {
    let _ = (handle, serviceuuid, characteristicuuid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.gatt.descriptorList is not available in the VM yet",
    ))
    .boxed())
}

/// Read current ATT MTU.
///
/// Read current negotiated ATT MTU for one connected device session.
///
/// # Platform
/// Unix and Windows.
/// Uses host GATT session metadata APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `device.bluetooth.gatt`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_bluetooth_gatt_mtu(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BluetoothDeviceHandle,
) -> RuntimeResult<u16> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.gatt.mtu is not available in the VM yet",
    ))
    .boxed())
}

/// Read one GATT characteristic value.
///
/// Read one characteristic value from one connected Bluetooth device session.
///
/// # Platform
/// Unix and Windows.
/// Uses host GATT read APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.bluetooth.gatt`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_bluetooth_gatt_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BluetoothDeviceHandle,
    serviceuuid: vm::StringHandle,
    characteristicuuid: vm::StringHandle,
    timeoutns: u64,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, serviceuuid, characteristicuuid, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.gatt.read is not available in the VM yet",
    ))
    .boxed())
}

/// Read one GATT descriptor value.
///
/// Read one descriptor value from one connected Bluetooth device session.
///
/// # Platform
/// Unix and Windows.
/// Uses host GATT descriptor read APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.bluetooth.gatt`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_bluetooth_gatt_read_descriptor(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BluetoothDeviceHandle,
    serviceuuid: vm::StringHandle,
    characteristicuuid: vm::StringHandle,
    descriptoruuid: vm::StringHandle,
    timeoutns: u64,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (
        handle,
        serviceuuid,
        characteristicuuid,
        descriptoruuid,
        timeoutns,
    );
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.gatt.readDescriptor is not available in the VM yet",
    ))
    .boxed())
}

/// Wait for one GATT value event.
///
/// Wait for one characteristic value-notification event from one subscription.
///
/// # Platform
/// Unix and Windows.
/// Uses host GATT notification queues.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `device.bluetooth.gatt`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_bluetooth_gatt_read_event(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BluetoothSubscriptionHandle,
    timeoutns: u64,
) -> RuntimeResult<BluetoothGattValueEventVm> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.gatt.readEvent is not available in the VM yet",
    ))
    .boxed())
}

/// Request one target ATT MTU.
///
/// Request ATT MTU negotiation and return negotiated MTU for one connected device session.
///
/// # Platform
/// Unix and Windows.
/// Uses host GATT MTU negotiation APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.bluetooth.gatt`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_bluetooth_gatt_request_mtu(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BluetoothDeviceHandle,
    mtu: u16,
    timeoutns: u64,
) -> RuntimeResult<u16> {
    let _ = (handle, mtu, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.gatt.requestMtu is not available in the VM yet",
    ))
    .boxed())
}

/// List discovered GATT services.
///
/// Enumerate GATT services available on one connected device.
///
/// # Platform
/// Unix and Windows.
/// Uses CoreBluetooth, BluetoothGatt, and host Bluetooth stack service-discovery APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.bluetooth.gatt`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_bluetooth_gatt_service_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BluetoothDeviceHandle,
) -> RuntimeResult<VmSlice<BluetoothGattServiceDescriptorVm>> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.gatt.serviceList is not available in the VM yet",
    ))
    .boxed())
}

/// Subscribe one GATT characteristic.
///
/// Open one subscription for characteristic value notifications.
///
/// # Platform
/// Unix and Windows.
/// Uses host GATT notification subscribe APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.bluetooth.gatt`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_bluetooth_gatt_subscribe(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BluetoothDeviceHandle,
    serviceuuid: vm::StringHandle,
    characteristicuuid: vm::StringHandle,
) -> RuntimeResult<resource::BluetoothSubscriptionHandle> {
    let _ = (handle, serviceuuid, characteristicuuid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.gatt.subscribe is not available in the VM yet",
    ))
    .boxed())
}

/// Poll one GATT value event without blocking.
///
/// Poll one characteristic value-notification event from one subscription without waiting.
///
/// # Platform
/// Unix and Windows.
/// Uses host nonblocking GATT notification reads.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.bluetooth.gatt`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_bluetooth_gatt_try_read_event(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BluetoothSubscriptionHandle,
) -> RuntimeResult<BluetoothGattValueEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.gatt.tryReadEvent is not available in the VM yet",
    ))
    .boxed())
}

/// Unsubscribe one GATT characteristic.
///
/// Close one characteristic value notification subscription.
///
/// # Platform
/// Unix and Windows.
/// Uses host GATT notification unsubscribe APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.bluetooth.gatt`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_bluetooth_gatt_unsubscribe(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BluetoothSubscriptionHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.gatt.unsubscribe is not available in the VM yet",
    ))
    .boxed())
}

/// Write one GATT characteristic value.
///
/// Write one characteristic value on one connected Bluetooth device session.
///
/// # Platform
/// Unix and Windows.
/// Uses host GATT write APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.bluetooth.gatt`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_bluetooth_gatt_write(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BluetoothDeviceHandle,
    serviceuuid: vm::StringHandle,
    characteristicuuid: vm::StringHandle,
    argument_value: VmSlice<u8>,
    withresponse: bool,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (
        handle,
        serviceuuid,
        characteristicuuid,
        argument_value,
        withresponse,
        timeoutns,
    );
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.gatt.write is not available in the VM yet",
    ))
    .boxed())
}

/// Write one GATT descriptor value.
///
/// Write one descriptor value on one connected Bluetooth device session.
///
/// # Platform
/// Unix and Windows.
/// Uses host GATT descriptor write APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.bluetooth.gatt`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_bluetooth_gatt_write_descriptor(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BluetoothDeviceHandle,
    serviceuuid: vm::StringHandle,
    characteristicuuid: vm::StringHandle,
    descriptoruuid: vm::StringHandle,
    argument_value: VmSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (
        handle,
        serviceuuid,
        characteristicuuid,
        descriptoruuid,
        argument_value,
        timeoutns,
    );
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.gatt.writeDescriptor is not available in the VM yet",
    ))
    .boxed())
}

/// Close Bluetooth scan session.
///
/// Close one scan session and stop host discovery operations.
///
/// # Platform
/// Unix and Windows.
/// Uses host Bluetooth scan-stop APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.bluetooth.scan`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_bluetooth_scan_close(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BluetoothScanHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.scan.close is not available in the VM yet",
    ))
    .boxed())
}

/// Open Bluetooth scan session.
///
/// Open one Bluetooth scan session on one adapter with one optional filter.
///
/// # Platform
/// Unix and Windows.
/// Uses host Bluetooth scan APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.bluetooth.scan`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_bluetooth_scan_open(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    adapterid: vm::StringHandle,
    filter: BluetoothScanFilterVm,
) -> RuntimeResult<resource::BluetoothScanHandle> {
    let _ = (adapterid, filter);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.scan.open is not available in the VM yet",
    ))
    .boxed())
}

/// Wait for one scanned device.
///
/// Wait for one scanned device advertisement from one opened scan session.
///
/// # Platform
/// Unix and Windows.
/// Uses host Bluetooth scan result queues.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `device.bluetooth.scan`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_bluetooth_scan_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BluetoothScanHandle,
    timeoutns: u64,
) -> RuntimeResult<BluetoothDeviceDescriptorVm> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.scan.read is not available in the VM yet",
    ))
    .boxed())
}

/// Poll one scanned device without blocking.
///
/// Poll one scanned device advertisement from one opened scan session without waiting.
///
/// # Platform
/// Unix and Windows.
/// Uses host Bluetooth nonblocking scan result reads.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.bluetooth.scan`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_bluetooth_scan_try_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BluetoothScanHandle,
) -> RuntimeResult<BluetoothDeviceDescriptorVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.scan.tryRead is not available in the VM yet",
    ))
    .boxed())
}

/// Close Bluetooth device session.
///
/// Close one opened Bluetooth device session and release host resources.
///
/// # Platform
/// Unix and Windows.
/// Uses host Bluetooth disconnect APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.bluetooth.connect`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_bluetooth_close(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BluetoothDeviceHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.session.close is not available in the VM yet",
    ))
    .boxed())
}

/// Open Bluetooth device session.
///
/// Open one device session for link and GATT operations.
///
/// # Platform
/// Unix and Windows.
/// Uses host Bluetooth connect APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.bluetooth.connect`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_bluetooth_open(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    adapterid: vm::StringHandle,
    deviceid: vm::StringHandle,
) -> RuntimeResult<resource::BluetoothDeviceHandle> {
    let _ = (adapterid, deviceid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.bluetooth.session.open is not available in the VM yet",
    ))
    .boxed())
}

/// Close camera endpoint.
///
/// Close one opened camera endpoint and release host resources.
///
/// # Platform
/// Unix and Windows.
/// Uses backend camera endpoint close operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.camera.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_camera_device_close(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CameraDeviceHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.camera.device.close is not available in the VM yet",
    ))
    .boxed())
}

/// List camera endpoints.
///
/// Enumerate host camera endpoints and return stable identifiers.
///
/// # Platform
/// Unix and Windows.
/// Uses backend camera device enumeration APIs such as AVFoundation, Media Foundation, or V4L2.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `device.camera.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_camera_device_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<CameraDeviceDescriptorVm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.camera.device.list is not available in the VM yet",
    ))
    .boxed())
}

/// Open camera endpoint.
///
/// Open one host camera endpoint for stream operations.
///
/// # Platform
/// Unix and Windows.
/// Uses backend camera endpoint open operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.camera.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_camera_device_open(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<resource::CameraDeviceHandle> {
    let _ = id;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.camera.device.open is not available in the VM yet",
    ))
    .boxed())
}

/// Close camera stream.
///
/// Close one opened camera stream and release host resources.
///
/// # Platform
/// Unix and Windows.
/// Uses backend stream close operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.camera.capture`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_camera_stream_close(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.camera.stream.close is not available in the VM yet",
    ))
    .boxed())
}

/// Open camera stream.
///
/// Open one camera stream with explicit stream configuration.
///
/// # Platform
/// Unix and Windows.
/// Uses backend stream configuration and negotiation APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.camera.capture`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_camera_stream_open(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::CameraDeviceHandle,
    config: CameraStreamConfigVm,
) -> RuntimeResult<resource::CameraStreamHandle> {
    let _ = (device, config);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.camera.stream.open is not available in the VM yet",
    ))
    .boxed())
}

/// Read camera frame.
///
/// Wait for one camera frame from one running stream.
///
/// # Platform
/// Unix and Windows.
/// Uses backend camera frame queue operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `device.camera.capture`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_camera_stream_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CameraStreamHandle,
    timeoutns: u64,
) -> RuntimeResult<CameraFrameVm> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.camera.stream.read is not available in the VM yet",
    ))
    .boxed())
}

/// Set one camera control value.
///
/// Apply one normalized camera control value in one running stream.
///
/// # Platform
/// Unix and Windows.
/// Uses backend camera control APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.camera.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_camera_stream_set_control(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CameraStreamHandle,
    control: CameraControl,
    argument_value: f64,
) -> RuntimeResult<()> {
    let _ = (handle, control, argument_value);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.camera.stream.setControl is not available in the VM yet",
    ))
    .boxed())
}

/// Start camera stream.
///
/// Start one opened camera stream.
///
/// # Platform
/// Unix and Windows.
/// Uses backend stream start operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.camera.capture`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_camera_stream_start(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.camera.stream.start is not available in the VM yet",
    ))
    .boxed())
}

/// Stop camera stream.
///
/// Stop one running camera stream.
///
/// # Platform
/// Unix and Windows.
/// Uses backend stream stop operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.camera.capture`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_camera_stream_stop(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.camera.stream.stop is not available in the VM yet",
    ))
    .boxed())
}

/// Poll camera frame without blocking.
///
/// Poll one camera frame from one running stream without waiting.
///
/// # Platform
/// Unix and Windows.
/// Uses backend nonblocking camera frame queue operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.camera.capture`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_camera_stream_try_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraFrameVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.camera.stream.tryRead is not available in the VM yet",
    ))
    .boxed())
}

/// Close serial endpoint.
///
/// Close one opened serial endpoint and release host resources.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific serial close operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.serial.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_serial_close(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.serial.close is not available in the VM yet",
    ))
    .boxed())
}

/// Reconfigure serial endpoint.
///
/// Apply one serial line configuration on one opened endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses termios reconfiguration on Unix-like hosts and DCB reconfiguration on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.serial.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_serial_configure(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SerialPortHandle,
    config: SerialPortConfigVm,
) -> RuntimeResult<()> {
    let _ = (handle, config);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.serial.configure is not available in the VM yet",
    ))
    .boxed())
}

/// Flush serial output.
///
/// Drain queued outbound bytes on one opened serial endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses backend serial flush operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.serial.write`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_serial_flush(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.serial.flush is not available in the VM yet",
    ))
    .boxed())
}

/// List serial endpoints.
///
/// Enumerate available host serial endpoints and return stable identifiers.
/// Enumeration ordering follows host backend behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses `/dev/tty*` style enumeration on Unix-like hosts and SetupAPI COM-port enumeration on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `device.serial.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_serial_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<SerialPortDescriptorVm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.serial.list is not available in the VM yet",
    ))
    .boxed())
}

/// Open serial endpoint.
///
/// Open one serial endpoint with explicit line configuration.
///
/// # Platform
/// Unix and Windows.
/// Uses termios-family open operations on Unix-like hosts and CreateFile serial APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.serial.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_serial_open(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
    config: SerialPortConfigVm,
) -> RuntimeResult<resource::SerialPortHandle> {
    let _ = (id, config);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.serial.open is not available in the VM yet",
    ))
    .boxed())
}

/// Read serial bytes.
///
/// Read up to `maxBytes` from one opened serial endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses backend read operations with timeout handling.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `device.serial.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_serial_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SerialPortHandle,
    maxbytes: u32,
    timeoutns: u64,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, maxbytes, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.serial.read is not available in the VM yet",
    ))
    .boxed())
}

/// Set DTR and RTS control lines.
///
/// Apply DTR and RTS line state for one opened endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses backend serial line-control APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.serial.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_serial_set_control_lines(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SerialPortHandle,
    dtr: bool,
    rts: bool,
) -> RuntimeResult<()> {
    let _ = (handle, dtr, rts);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.serial.setControlLines is not available in the VM yet",
    ))
    .boxed())
}

/// Read serial signal lines.
///
/// Return current serial signal-line bitmask for one opened endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses backend modem-status APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `device.serial.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_serial_signal_bits(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<u32> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.serial.signalBits is not available in the VM yet",
    ))
    .boxed())
}

/// Poll one serial event without blocking.
///
/// Poll one pending serial event for one opened endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses backend nonblocking serial event polling where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.serial.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_serial_try_event(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<SerialEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.serial.tryEvent is not available in the VM yet",
    ))
    .boxed())
}

/// Poll serial bytes without blocking.
///
/// Read up to `maxBytes` from one opened serial endpoint without waiting.
///
/// # Platform
/// Unix and Windows.
/// Uses backend nonblocking serial read operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.serial.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_serial_try_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SerialPortHandle,
    maxbytes: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, maxbytes);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.serial.tryRead is not available in the VM yet",
    ))
    .boxed())
}

/// Write serial bytes.
///
/// Write one byte sequence to one opened serial endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses backend serial write operations with timeout handling.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.serial.write`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_serial_write(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SerialPortHandle,
    data: VmSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<u32> {
    let _ = (handle, data, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.serial.write is not available in the VM yet",
    ))
    .boxed())
}

/// Read bulk endpoint bytes.
///
/// Read up to `maxBytes` from one bulk IN endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB bulk-transfer APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `device.usb.transfer`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_bulk_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
    endpointaddress: u8,
    maxbytes: u32,
    timeoutns: u64,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, endpointaddress, maxbytes, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.bulkRead is not available in the VM yet",
    ))
    .boxed())
}

/// Write bulk endpoint bytes.
///
/// Write bytes to one bulk OUT endpoint and return transferred byte count.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB bulk-transfer APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.usb.transfer`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_bulk_write(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
    endpointaddress: u8,
    argument_bytes: VmSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<u32> {
    let _ = (handle, endpointaddress, argument_bytes, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.bulkWrite is not available in the VM yet",
    ))
    .boxed())
}

/// Claim USB interface.
///
/// Claim one interface on one opened USB device.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB interface-claim APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.usb.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_claim_interface(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
    interfacenumber: u8,
) -> RuntimeResult<()> {
    let _ = (handle, interfacenumber);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.claimInterface is not available in the VM yet",
    ))
    .boxed())
}

/// Close USB device.
///
/// Close one opened USB device and release host resources.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB close APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.usb.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_close(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.close is not available in the VM yet",
    ))
    .boxed())
}

/// Read active USB configuration value.
///
/// Read active configuration value for one opened USB device.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB configuration-state APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `device.usb.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_configuration_get(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<u8> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.configurationGet is not available in the VM yet",
    ))
    .boxed())
}

/// List USB configurations.
///
/// Enumerate USB configurations and nested interface descriptors for one opened device.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB configuration-descriptor query APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `device.usb.enumerate`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_configuration_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<VmSlice<UsbConfigurationDescriptorVm>> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.configurationList is not available in the VM yet",
    ))
    .boxed())
}

/// Set active USB configuration value.
///
/// Apply active configuration value for one opened USB device.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB set-configuration APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.usb.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_configuration_set(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
    configurationvalue: u8,
) -> RuntimeResult<()> {
    let _ = (handle, configurationvalue);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.configurationSet is not available in the VM yet",
    ))
    .boxed())
}

/// Read control-transfer response bytes.
///
/// Execute one control-transfer read and return response bytes.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB control-transfer APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `device.usb.transfer`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_control_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
    setup: UsbControlSetupVm,
    timeoutns: u64,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, setup, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.controlRead is not available in the VM yet",
    ))
    .boxed())
}

/// Write control-transfer request bytes.
///
/// Execute one control-transfer write and return transferred byte count.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB control-transfer APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.usb.transfer`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_control_write(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
    setup: UsbControlSetupVm,
    argument_bytes: VmSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<u32> {
    let _ = (handle, setup, argument_bytes, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.controlWrite is not available in the VM yet",
    ))
    .boxed())
}

/// Read active USB device descriptor.
///
/// Read descriptor metadata for one opened USB device.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB descriptor-query APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `device.usb.enumerate`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_descriptor(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<UsbDeviceDescriptorVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.descriptor is not available in the VM yet",
    ))
    .boxed())
}

/// Read interrupt endpoint bytes.
///
/// Read up to `maxBytes` from one interrupt IN endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB interrupt-transfer APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `device.usb.transfer`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_interrupt_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
    endpointaddress: u8,
    maxbytes: u32,
    timeoutns: u64,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, endpointaddress, maxbytes, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.interruptRead is not available in the VM yet",
    ))
    .boxed())
}

/// Write interrupt endpoint bytes.
///
/// Write bytes to one interrupt OUT endpoint and return transferred byte count.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB interrupt-transfer APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.usb.transfer`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_interrupt_write(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
    endpointaddress: u8,
    argument_bytes: VmSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<u32> {
    let _ = (handle, endpointaddress, argument_bytes, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.interruptWrite is not available in the VM yet",
    ))
    .boxed())
}

/// Read one isochronous transfer.
///
/// Read one isochronous transfer and return flattened bytes with per-packet status and length metadata.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB isochronous transfer APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `device.usb.transfer`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_isochronous_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
    endpointaddress: u8,
    packetsizes: VmSlice<u32>,
    timeoutns: u64,
) -> RuntimeResult<UsbIsochronousTransferResultVm> {
    let _ = (handle, endpointaddress, packetsizes, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.isochronousRead is not available in the VM yet",
    ))
    .boxed())
}

/// Write one isochronous transfer.
///
/// Write one flattened isochronous transfer and return per-packet status and length metadata.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB isochronous transfer APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.usb.transfer`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_isochronous_write(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
    endpointaddress: u8,
    argument_bytes: VmSlice<u8>,
    packetsizes: VmSlice<u32>,
    timeoutns: u64,
) -> RuntimeResult<UsbIsochronousTransferResultVm> {
    let _ = (
        handle,
        endpointaddress,
        argument_bytes,
        packetsizes,
        timeoutns,
    );
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.isochronousWrite is not available in the VM yet",
    ))
    .boxed())
}

/// List USB devices.
///
/// Enumerate attached USB devices and return stable descriptors.
///
/// # Platform
/// Unix and Windows.
/// Uses libusb-family enumeration or host USB APIs.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `device.usb.enumerate`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<UsbDeviceDescriptorVm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.list is not available in the VM yet",
    ))
    .boxed())
}

/// Open USB device.
///
/// Open one USB device for control and data transfers.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB open APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.usb.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_open(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<resource::UsbDeviceHandle> {
    let _ = id;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.open is not available in the VM yet",
    ))
    .boxed())
}

/// Release USB interface.
///
/// Release one interface previously claimed on one opened USB device.
///
/// # Platform
/// Unix and Windows.
/// Uses host USB interface-release APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `device.usb.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_device_usb_release_interface(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UsbDeviceHandle,
    interfacenumber: u8,
) -> RuntimeResult<()> {
    let _ = (handle, interfacenumber);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.device.usb.releaseInterface is not available in the VM yet",
    ))
    .boxed())
}
