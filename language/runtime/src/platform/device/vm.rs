use super::host as device_host;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
use crate::platform::core::{NativeAbiCodec, VmAbiCodec, call_out};
use crate::platform::device::{
    BluetoothAdapterDescriptor, BluetoothAdapterDescriptorVm, BluetoothAdapterEvent,
    BluetoothAdapterEventVm, BluetoothDeviceDescriptor, BluetoothDeviceDescriptorVm,
    BluetoothGattCharacteristic, BluetoothGattCharacteristicVm, BluetoothGattDescriptor,
    BluetoothGattDescriptorVm, BluetoothGattService, BluetoothGattServiceVm,
    BluetoothGattValueEvent, BluetoothGattValueEventVm, BluetoothGattWriteMode, BluetoothScanEvent,
    BluetoothScanEventVm, BluetoothScanFilterVm, BluetoothSessionEvent, BluetoothSessionEventVm,
    CameraControlCapabilities, CameraControlCapabilitiesVm, CameraControlPatch,
    CameraControlPatchVm, CameraControlState, CameraControlStateValue, CameraControlStateVm,
    CameraDeviceDescriptor, CameraDeviceDescriptorVm, CameraExposureCompensationRange,
    CameraExposureCompensationRangeVm, CameraExposureMode, CameraExposureTimeRange,
    CameraExposureTimeRangeVm, CameraFloatControlRange, CameraFloatControlRangeVm,
    CameraFocusDistanceRange, CameraFocusDistanceRangeVm, CameraFocusMode, CameraFrame,
    CameraFrameVm, CameraPanAngleRange, CameraPanAngleRangeVm, CameraPhoto,
    CameraPhotoCapabilities, CameraPhotoCapabilitiesVm, CameraPhotoSettings, CameraPhotoSettingsVm,
    CameraPhotoState, CameraPhotoStateVm, CameraPhotoVm, CameraRecording,
    CameraRecordingCapabilities, CameraRecordingCapabilitiesVm, CameraRecordingOptions,
    CameraRecordingOptionsVm, CameraRecordingState, CameraRecordingStateVm, CameraRecordingVm,
    CameraSensorIsoRange, CameraSensorIsoRangeVm, CameraStabilizationMode, CameraStreamCapability,
    CameraStreamCapabilityVm, CameraStreamConfig, CameraStreamConfigVm, CameraTiltAngleRange,
    CameraTiltAngleRangeVm, CameraTorchMode, CameraWatchEvent, CameraWatchEventVm,
    CameraWhiteBalanceMode, CameraWhiteBalanceRange, CameraWhiteBalanceRangeVm,
    CameraZoomRatioRange, CameraZoomRatioRangeVm, SerialEvent, SerialEventVm, SerialInputSignals,
    SerialInputSignalsVm, SerialOutputSignals, SerialOutputSignalsVm, SerialPortConfig,
    SerialPortConfigVm, SerialPortDescriptor, SerialPortDescriptorVm, SerialPortOpenOptions,
    SerialPortOpenOptionsVm, SerialWatchEvent, SerialWatchEventVm, UsbBosCapabilityDescriptor,
    UsbBosCapabilityDescriptorVm, UsbConfigurationDescriptor, UsbConfigurationDescriptorVm,
    UsbControlSetup, UsbControlSetupVm, UsbDeviceDescriptor, UsbDeviceDescriptorVm,
    UsbEndpointSelector, UsbEndpointSelectorVm, UsbHotplugEvent, UsbHotplugEventVm,
    UsbInTransferResult, UsbInTransferResultVm, UsbIsochronousTransferResult,
    UsbIsochronousTransferResultVm, UsbOutTransferResult, UsbOutTransferResultVm,
    UsbStringDescriptor, UsbStringDescriptorVm,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{VmSlice, resource};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// Decode one VM binding payload into one native binding payload.
fn native_value_from_vm<Native, Vm>(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    value: Vm,
) -> RuntimeResult<Native>
where
    Native: NativeAbiCodec<Value = Vm::Value>,
    Vm: VmAbiCodec,
{
    let value = value.into_value(&context.read())?;

    Ok(Native::from_value(binding, value))
}

/// Encode one native binding payload into one VM binding payload.
fn vm_value_from_native<Native, Vm>(
    context: &mut vm::BindingContext<'_>,
    value: Native,
) -> RuntimeResult<Vm>
where
    Native: NativeAbiCodec,
    Vm: VmAbiCodec<Value = Native::Value>,
{
    let value = unsafe { value.into_value()? };

    Vm::from_value(&mut context.write(), value)
}

/// Call one native out-parameter binding and encode the result for the VM.
fn vm_call_out<Native, Vm>(
    context: &mut vm::BindingContext<'_>,
    call: impl FnOnce(*mut Native) -> RuntimeResult<()>,
) -> RuntimeResult<Vm>
where
    Native: NativeAbiCodec,
    Vm: VmAbiCodec<Value = Native::Value>,
{
    let value = call_out(call)?;

    vm_value_from_native(context, value)
}

/// Build one mutable native byte slice over one VM-owned byte buffer copy.
fn native_bytes_from_vec(bytes: &mut Vec<u8>) -> NativeSlice<u8> {
    NativeSlice {
        data: bytes.as_mut_ptr(),
        len: bytes.len() as u32,
    }
}

/// Return whether one runtime error reports one unsupported platform operation.
fn is_not_supported_error(error: &RuntimeError) -> bool {
    error
        .platform_error()
        .is_some_and(|platform| platform.code == PlatformErrorCode::NotSupported)
}

/// Read one optional native binding value.
fn optional_native_value<T>(
    read: impl FnOnce() -> RuntimeResult<T>,
) -> RuntimeResult<Option<T::Value>>
where
    T: NativeAbiCodec,
{
    match read() {
        Ok(value) => {
            let value = unsafe { T::into_value(value)? };

            Ok(Some(value))
        }
        Err(error) if is_not_supported_error(error.as_ref()) => Ok(None),
        Err(error) => Err(error),
    }
}

/// Read one optional VM binding value and decode it into one native binding value.
fn optional_vm_value<Native, Vm>(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    value: RuntimeResult<Vm>,
) -> RuntimeResult<Option<Native::Value>>
where
    Native: NativeAbiCodec<Value = Vm::Value>,
    Vm: VmAbiCodec,
{
    match value {
        Ok(value) => {
            let value: Native = native_value_from_vm(binding, context, value)?;
            let value = unsafe { Native::into_value(value)? };

            Ok(Some(value))
        }
        Err(error) if is_not_supported_error(error.as_ref()) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) use super::midi::vm::*;

/// List Bluetooth adapters.
pub(crate) fn destack_device_bluetooth_adapter_list(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<BluetoothAdapterDescriptorVm>> {
    vm_call_out(
        context,
        |out: *mut NativeSlice<BluetoothAdapterDescriptor>| unsafe {
            device_host::destack_device_bluetooth_adapter_list(binding, out)
        },
    )
}

/// Close a Bluetooth adapter watch stream.
pub(crate) fn destack_device_bluetooth_adapter_watch_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothAdapterWatchHandle,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_bluetooth_adapter_watch_close(binding, handle) }
}

/// Open a Bluetooth adapter watch stream.
pub(crate) fn destack_device_bluetooth_adapter_watch_open(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::BluetoothAdapterWatchHandle> {
    call_out(|out| unsafe {
        device_host::destack_device_bluetooth_adapter_watch_open(binding, out)
    })
}

/// Read one Bluetooth adapter event.
pub(crate) fn destack_device_bluetooth_adapter_watch_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothAdapterWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<BluetoothAdapterEventVm> {
    vm_call_out(context, |out: *mut BluetoothAdapterEvent| unsafe {
        device_host::destack_device_bluetooth_adapter_watch_read(binding, out, handle, timeoutns)
    })
}

/// Poll one Bluetooth adapter event without blocking.
pub(crate) fn destack_device_bluetooth_adapter_watch_try_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothAdapterWatchHandle,
) -> RuntimeResult<BluetoothAdapterEventVm> {
    vm_call_out(context, |out: *mut BluetoothAdapterEvent| unsafe {
        device_host::destack_device_bluetooth_adapter_watch_try_read(binding, out, handle)
    })
}

/// List discovered GATT characteristics for one service.
pub(crate) fn destack_device_bluetooth_gatt_characteristic_list(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothDeviceHandle,
    serviceid: vm::StringHandle,
) -> RuntimeResult<VmSlice<BluetoothGattCharacteristicVm>> {
    let service_id = native_value_from_vm(binding, context, serviceid)?;

    vm_call_out(
        context,
        |out: *mut NativeSlice<BluetoothGattCharacteristic>| unsafe {
            device_host::destack_device_bluetooth_gatt_characteristic_list(
                binding, out, handle, service_id,
            )
        },
    )
}

/// List discovered GATT descriptors for one characteristic.
pub(crate) fn destack_device_bluetooth_gatt_descriptor_list(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothDeviceHandle,
    characteristicid: vm::StringHandle,
) -> RuntimeResult<VmSlice<BluetoothGattDescriptorVm>> {
    let characteristic_id = native_value_from_vm(binding, context, characteristicid)?;

    vm_call_out(
        context,
        |out: *mut NativeSlice<BluetoothGattDescriptor>| unsafe {
            device_host::destack_device_bluetooth_gatt_descriptor_list(
                binding,
                out,
                handle,
                characteristic_id,
            )
        },
    )
}

/// Read current ATT MTU.
pub(crate) fn destack_device_bluetooth_gatt_mtu(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothDeviceHandle,
) -> RuntimeResult<u16> {
    call_out(|out| unsafe { device_host::destack_device_bluetooth_gatt_mtu(binding, out, handle) })
}

/// Read one GATT characteristic value.
pub(crate) fn destack_device_bluetooth_gatt_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothDeviceHandle,
    characteristicid: vm::StringHandle,
    timeoutns: u64,
) -> RuntimeResult<VmSlice<u8>> {
    let characteristic_id = native_value_from_vm(binding, context, characteristicid)?;

    vm_call_out(context, |out: *mut NativeSlice<u8>| unsafe {
        device_host::destack_device_bluetooth_gatt_read(
            binding,
            out,
            handle,
            characteristic_id,
            timeoutns,
        )
    })
}

/// Read one GATT descriptor value.
pub(crate) fn destack_device_bluetooth_gatt_read_descriptor(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothDeviceHandle,
    descriptorid: vm::StringHandle,
    timeoutns: u64,
) -> RuntimeResult<VmSlice<u8>> {
    let descriptor_id = native_value_from_vm(binding, context, descriptorid)?;

    vm_call_out(context, |out: *mut NativeSlice<u8>| unsafe {
        device_host::destack_device_bluetooth_gatt_read_descriptor(
            binding,
            out,
            handle,
            descriptor_id,
            timeoutns,
        )
    })
}

/// Wait for one GATT value event.
pub(crate) fn destack_device_bluetooth_gatt_read_event(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothSubscriptionHandle,
    timeoutns: u64,
) -> RuntimeResult<BluetoothGattValueEventVm> {
    vm_call_out(context, |out: *mut BluetoothGattValueEvent| unsafe {
        device_host::destack_device_bluetooth_gatt_read_event(binding, out, handle, timeoutns)
    })
}

/// List discovered GATT services.
pub(crate) fn destack_device_bluetooth_gatt_service_list(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothDeviceHandle,
) -> RuntimeResult<VmSlice<BluetoothGattServiceVm>> {
    vm_call_out(
        context,
        |out: *mut NativeSlice<BluetoothGattService>| unsafe {
            device_host::destack_device_bluetooth_gatt_service_list(binding, out, handle)
        },
    )
}

/// Subscribe one GATT characteristic.
pub(crate) fn destack_device_bluetooth_gatt_subscribe(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothDeviceHandle,
    characteristicid: vm::StringHandle,
) -> RuntimeResult<resource::BluetoothSubscriptionHandle> {
    let characteristic_id = native_value_from_vm(binding, context, characteristicid)?;

    call_out(|out| unsafe {
        device_host::destack_device_bluetooth_gatt_subscribe(
            binding,
            out,
            handle,
            characteristic_id,
        )
    })
}

/// Poll one GATT value event without blocking.
pub(crate) fn destack_device_bluetooth_gatt_try_read_event(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothSubscriptionHandle,
) -> RuntimeResult<BluetoothGattValueEventVm> {
    vm_call_out(context, |out: *mut BluetoothGattValueEvent| unsafe {
        device_host::destack_device_bluetooth_gatt_try_read_event(binding, out, handle)
    })
}

/// Unsubscribe one GATT characteristic.
pub(crate) fn destack_device_bluetooth_gatt_unsubscribe(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothSubscriptionHandle,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_bluetooth_gatt_unsubscribe(binding, handle) }
}

/// Write one GATT characteristic value.
pub(crate) fn destack_device_bluetooth_gatt_write(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothDeviceHandle,
    characteristicid: vm::StringHandle,
    argument_value: VmSlice<u8>,
    mode: BluetoothGattWriteMode,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let characteristic_id = native_value_from_vm(binding, context, characteristicid)?;
    let argument_value: NativeSlice<u8> = native_value_from_vm(binding, context, argument_value)?;

    unsafe {
        device_host::destack_device_bluetooth_gatt_write(
            binding,
            handle,
            characteristic_id,
            argument_value,
            mode,
            timeoutns,
        )
    }
}

/// Write one GATT descriptor value.
pub(crate) fn destack_device_bluetooth_gatt_write_descriptor(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothDeviceHandle,
    descriptorid: vm::StringHandle,
    argument_value: VmSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let descriptor_id = native_value_from_vm(binding, context, descriptorid)?;
    let argument_value: NativeSlice<u8> = native_value_from_vm(binding, context, argument_value)?;

    unsafe {
        device_host::destack_device_bluetooth_gatt_write_descriptor(
            binding,
            handle,
            descriptor_id,
            argument_value,
            timeoutns,
        )
    }
}

/// Close Bluetooth scan session.
pub(crate) fn destack_device_bluetooth_scan_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothScanHandle,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_bluetooth_scan_close(binding, handle) }
}

/// Open Bluetooth scan session.
pub(crate) fn destack_device_bluetooth_scan_open(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    adapterid: vm::StringHandle,
    filter: Option<BluetoothScanFilterVm>,
) -> RuntimeResult<resource::BluetoothScanHandle> {
    let adapter_id = native_value_from_vm(binding, context, adapterid)?;
    let filter = filter
        .map(|filter| native_value_from_vm(binding, context, filter))
        .transpose()?;

    call_out(|out| unsafe {
        device_host::destack_device_bluetooth_scan_open(binding, out, adapter_id, filter)
    })
}

/// Wait for one Bluetooth scan event.
pub(crate) fn destack_device_bluetooth_scan_read_event(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothScanHandle,
    timeoutns: u64,
) -> RuntimeResult<BluetoothScanEventVm> {
    vm_call_out(context, |out: *mut BluetoothScanEvent| unsafe {
        device_host::destack_device_bluetooth_scan_read_event(binding, out, handle, timeoutns)
    })
}

/// Poll one Bluetooth scan event without blocking.
pub(crate) fn destack_device_bluetooth_scan_try_read_event(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothScanHandle,
) -> RuntimeResult<BluetoothScanEventVm> {
    vm_call_out(context, |out: *mut BluetoothScanEvent| unsafe {
        device_host::destack_device_bluetooth_scan_try_read_event(binding, out, handle)
    })
}

/// Close Bluetooth device session.
pub(crate) fn destack_device_bluetooth_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothDeviceHandle,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_bluetooth_close(binding, handle) }
}

/// Read Bluetooth device descriptor.
pub(crate) fn destack_device_bluetooth_descriptor(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothDeviceHandle,
) -> RuntimeResult<BluetoothDeviceDescriptorVm> {
    vm_call_out(context, |out: *mut BluetoothDeviceDescriptor| unsafe {
        device_host::destack_device_bluetooth_descriptor(binding, out, handle)
    })
}

/// Open Bluetooth device session.
pub(crate) fn destack_device_bluetooth_open(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    adapterid: vm::StringHandle,
    deviceid: vm::StringHandle,
) -> RuntimeResult<resource::BluetoothDeviceHandle> {
    let adapter_id = native_value_from_vm(binding, context, adapterid)?;
    let device_id = native_value_from_vm(binding, context, deviceid)?;

    call_out(|out| unsafe {
        device_host::destack_device_bluetooth_open(binding, out, adapter_id, device_id)
    })
}

/// Pair one Bluetooth device.
pub(crate) fn destack_device_bluetooth_pair(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothDeviceHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_bluetooth_pair(binding, handle, timeoutns) }
}

/// Wait for one Bluetooth session event.
pub(crate) fn destack_device_bluetooth_session_read_event(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothDeviceHandle,
    timeoutns: u64,
) -> RuntimeResult<BluetoothSessionEventVm> {
    vm_call_out(context, |out: *mut BluetoothSessionEvent| unsafe {
        device_host::destack_device_bluetooth_session_read_event(binding, out, handle, timeoutns)
    })
}

/// Read link RSSI for one Bluetooth device session.
pub(crate) fn destack_device_bluetooth_read_rssi(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothDeviceHandle,
    timeoutns: u64,
) -> RuntimeResult<i32> {
    call_out(|out| unsafe {
        device_host::destack_device_bluetooth_read_rssi(binding, out, handle, timeoutns)
    })
}

/// Poll one Bluetooth session event without blocking.
pub(crate) fn destack_device_bluetooth_session_try_read_event(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::BluetoothDeviceHandle,
) -> RuntimeResult<BluetoothSessionEventVm> {
    vm_call_out(context, |out: *mut BluetoothSessionEvent| unsafe {
        device_host::destack_device_bluetooth_session_try_read_event(binding, out, handle)
    })
}

/// Remove one Bluetooth device bond.
pub(crate) fn destack_device_bluetooth_unpair(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    adapterid: vm::StringHandle,
    deviceid: vm::StringHandle,
) -> RuntimeResult<()> {
    let adapter_id = native_value_from_vm(binding, context, adapterid)?;
    let device_id = native_value_from_vm(binding, context, deviceid)?;

    unsafe { device_host::destack_device_bluetooth_unpair(binding, adapter_id, device_id) }
}

/// Close camera endpoint.
pub(crate) fn destack_device_camera_device_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraDeviceHandle,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_camera_device_close(binding, handle) }
}

/// List camera endpoints.
pub(crate) fn destack_device_camera_device_list(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<CameraDeviceDescriptorVm>> {
    vm_call_out(
        context,
        |out: *mut NativeSlice<CameraDeviceDescriptor>| unsafe {
            device_host::destack_device_camera_device_list(binding, out)
        },
    )
}

/// Open camera endpoint.
pub(crate) fn destack_device_camera_device_open(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<resource::CameraDeviceHandle> {
    let id = native_value_from_vm(binding, context, id)?;

    call_out(|out| unsafe { device_host::destack_device_camera_device_open(binding, out, id) })
}

/// Close a camera watch stream.
pub(crate) fn destack_device_camera_device_watch_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraWatchHandle,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_camera_device_watch_close(binding, handle) }
}

/// Open a camera watch stream.
pub(crate) fn destack_device_camera_device_watch_open(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::CameraWatchHandle> {
    call_out(|out| unsafe { device_host::destack_device_camera_device_watch_open(binding, out) })
}

/// Read one camera watch event.
pub(crate) fn destack_device_camera_device_watch_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<CameraWatchEventVm> {
    vm_call_out(context, |out: *mut CameraWatchEvent| unsafe {
        device_host::destack_device_camera_device_watch_read(binding, out, handle, timeoutns)
    })
}

/// Poll one camera watch event without blocking.
pub(crate) fn destack_device_camera_device_watch_try_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraWatchHandle,
) -> RuntimeResult<CameraWatchEventVm> {
    vm_call_out(context, |out: *mut CameraWatchEvent| unsafe {
        device_host::destack_device_camera_device_watch_try_read(binding, out, handle)
    })
}

/// List supported stream capabilities for one opened camera endpoint.
pub(crate) fn destack_device_camera_device_stream_capability_list(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraDeviceHandle,
) -> RuntimeResult<VmSlice<CameraStreamCapabilityVm>> {
    vm_call_out(
        context,
        |out: *mut NativeSlice<CameraStreamCapability>| unsafe {
            device_host::destack_device_camera_device_stream_capability_list(binding, out, handle)
        },
    )
}

/// List supported stream configurations for one opened camera endpoint.
pub(crate) fn destack_device_camera_stream_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_camera_stream_close(binding, handle) }
}

/// Read current camera stream configuration.
pub(crate) fn destack_device_camera_stream_config(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraStreamConfigVm> {
    vm_call_out(context, |out: *mut CameraStreamConfig| unsafe {
        device_host::destack_device_camera_stream_config(binding, out, handle)
    })
}

/// Read camera recording capabilities.
pub(crate) fn destack_device_camera_stream_recording_capabilities(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraRecordingCapabilitiesVm> {
    vm_call_out(context, |out: *mut CameraRecordingCapabilities| unsafe {
        device_host::destack_device_camera_stream_recording_capabilities(binding, out, handle)
    })
}

/// Read camera recording state.
pub(crate) fn destack_device_camera_stream_recording_state(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraRecordingStateVm> {
    vm_call_out(context, |out: *mut CameraRecordingState| unsafe {
        device_host::destack_device_camera_stream_recording_state(binding, out, handle)
    })
}

/// Start camera recording.
pub(crate) fn destack_device_camera_stream_start_recording(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    options: CameraRecordingOptionsVm,
) -> RuntimeResult<()> {
    let options: CameraRecordingOptions = native_value_from_vm(binding, context, options)?;

    unsafe { device_host::destack_device_camera_stream_start_recording(binding, handle, options) }
}

/// Pause camera recording.
pub(crate) fn destack_device_camera_stream_pause_recording(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_camera_stream_pause_recording(binding, handle) }
}

/// Resume camera recording.
pub(crate) fn destack_device_camera_stream_resume_recording(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_camera_stream_resume_recording(binding, handle) }
}

/// Stop camera recording.
pub(crate) fn destack_device_camera_stream_stop_recording(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    timeoutns: u64,
) -> RuntimeResult<CameraRecordingVm> {
    vm_call_out(context, |out: *mut CameraRecording| unsafe {
        device_host::destack_device_camera_stream_stop_recording(binding, out, handle, timeoutns)
    })
}

/// Read camera photo capabilities.
pub(crate) fn destack_device_camera_stream_photo_capabilities(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraPhotoCapabilitiesVm> {
    vm_call_out(context, |out: *mut CameraPhotoCapabilities| unsafe {
        device_host::destack_device_camera_stream_photo_capabilities(binding, out, handle)
    })
}

/// Read camera photo state.
pub(crate) fn destack_device_camera_stream_photo_state(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraPhotoStateVm> {
    vm_call_out(context, |out: *mut CameraPhotoState| unsafe {
        device_host::destack_device_camera_stream_photo_state(binding, out, handle)
    })
}

/// Capture one still photo.
pub(crate) fn destack_device_camera_stream_take_photo(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    settings: CameraPhotoSettingsVm,
    timeoutns: u64,
) -> RuntimeResult<CameraPhotoVm> {
    let settings: CameraPhotoSettings = native_value_from_vm(binding, context, settings)?;

    vm_call_out(context, |out: *mut CameraPhoto| unsafe {
        device_host::destack_device_camera_stream_take_photo(
            binding, out, handle, settings, timeoutns,
        )
    })
}

/// Apply camera control updates.
pub(crate) fn destack_device_camera_stream_configure_controls(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    controls: CameraControlPatchVm,
) -> RuntimeResult<()> {
    let controls: CameraControlPatch = native_value_from_vm(binding, context, controls)?;
    let controls = unsafe { <CameraControlPatch as NativeAbiCodec>::into_value(controls)? };

    // mode-like controls
    if let Some(value) = controls.exposure_mode {
        destack_device_camera_stream_set_exposure_mode(binding, context, handle, value)?;
    }

    if let Some(value) = controls.white_balance_mode {
        destack_device_camera_stream_set_white_balance_mode(binding, context, handle, value)?;
    }

    if let Some(value) = controls.focus_mode {
        destack_device_camera_stream_set_focus_mode(binding, context, handle, value)?;
    }

    if let Some(value) = controls.stabilization_mode {
        destack_device_camera_stream_set_stabilization_mode(binding, context, handle, value)?;
    }

    if let Some(value) = controls.torch_mode {
        destack_device_camera_stream_set_torch_mode(binding, context, handle, value)?;
    }

    // numeric controls
    if let Some(value) = controls.exposure_compensation_ev {
        destack_device_camera_stream_set_exposure_compensation(binding, context, handle, value)?;
    }

    if let Some(value) = controls.exposure_time_ns {
        destack_device_camera_stream_set_exposure_time_ns(binding, context, handle, value)?;
    }

    if let Some(value) = controls.sensor_iso {
        destack_device_camera_stream_set_sensor_iso(binding, context, handle, value)?;
    }

    if let Some(value) = controls.white_balance_kelvin {
        destack_device_camera_stream_set_white_balance_kelvin(binding, context, handle, value)?;
    }

    if let Some(value) = controls.focus_distance_diopters {
        destack_device_camera_stream_set_focus_distance_diopters(binding, context, handle, value)?;
    }

    if let Some(value) = controls.brightness {
        destack_device_camera_stream_set_brightness(binding, context, handle, value)?;
    }

    if let Some(value) = controls.contrast {
        destack_device_camera_stream_set_contrast(binding, context, handle, value)?;
    }

    if let Some(value) = controls.saturation {
        destack_device_camera_stream_set_saturation(binding, context, handle, value)?;
    }

    if let Some(value) = controls.sharpness {
        destack_device_camera_stream_set_sharpness(binding, context, handle, value)?;
    }

    if let Some(value) = controls.pan_degrees {
        destack_device_camera_stream_set_pan_degrees(binding, context, handle, value)?;
    }

    if let Some(value) = controls.tilt_degrees {
        destack_device_camera_stream_set_tilt_degrees(binding, context, handle, value)?;
    }

    if let Some(value) = controls.zoom_ratio {
        destack_device_camera_stream_set_zoom_ratio(binding, context, handle, value)?;
    }

    Ok(())
}

/// Read camera control capabilities.
pub(crate) fn destack_device_camera_stream_control_capabilities(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraControlCapabilitiesVm> {
    let capabilities: CameraControlCapabilitiesVm =
        vm_call_out(context, |out: *mut CameraControlCapabilities| unsafe {
            device_host::destack_device_camera_stream_control_capabilities(binding, out, handle)
        })?;
    let capabilities: CameraControlCapabilities =
        native_value_from_vm(binding, context, capabilities)?;
    let mut capabilities = unsafe { CameraControlCapabilities::into_value(capabilities)? };

    // range overrides
    let exposure_compensation_range =
        destack_device_camera_stream_exposure_compensation_range(binding, context, handle);
    capabilities.exposure_compensation_range = optional_vm_value::<
        CameraExposureCompensationRange,
        _,
    >(binding, context, exposure_compensation_range)?;

    let exposure_time_range =
        destack_device_camera_stream_exposure_time_range(binding, context, handle);
    capabilities.exposure_time_range =
        optional_vm_value::<CameraExposureTimeRange, _>(binding, context, exposure_time_range)?;

    let sensor_iso_range = destack_device_camera_stream_sensor_iso_range(binding, context, handle);
    capabilities.sensor_iso_range =
        optional_vm_value::<CameraSensorIsoRange, _>(binding, context, sensor_iso_range)?;

    let white_balance_range =
        destack_device_camera_stream_white_balance_range(binding, context, handle);
    capabilities.white_balance_range =
        optional_vm_value::<CameraWhiteBalanceRange, _>(binding, context, white_balance_range)?;

    let focus_distance_range =
        destack_device_camera_stream_focus_distance_range(binding, context, handle);
    capabilities.focus_distance_range =
        optional_vm_value::<CameraFocusDistanceRange, _>(binding, context, focus_distance_range)?;

    let brightness_range = destack_device_camera_stream_brightness_range(binding, context, handle);
    capabilities.brightness_range =
        optional_vm_value::<CameraFloatControlRange, _>(binding, context, brightness_range)?;

    let contrast_range = destack_device_camera_stream_contrast_range(binding, context, handle);
    capabilities.contrast_range =
        optional_vm_value::<CameraFloatControlRange, _>(binding, context, contrast_range)?;

    let saturation_range = destack_device_camera_stream_saturation_range(binding, context, handle);
    capabilities.saturation_range =
        optional_vm_value::<CameraFloatControlRange, _>(binding, context, saturation_range)?;

    let sharpness_range = destack_device_camera_stream_sharpness_range(binding, context, handle);
    capabilities.sharpness_range =
        optional_vm_value::<CameraFloatControlRange, _>(binding, context, sharpness_range)?;

    let pan_range = destack_device_camera_stream_pan_range(binding, context, handle);
    capabilities.pan_range =
        optional_vm_value::<CameraPanAngleRange, _>(binding, context, pan_range)?;

    let tilt_range = destack_device_camera_stream_tilt_range(binding, context, handle);
    capabilities.tilt_range =
        optional_vm_value::<CameraTiltAngleRange, _>(binding, context, tilt_range)?;

    let zoom_ratio_range = destack_device_camera_stream_zoom_ratio_range(binding, context, handle);
    capabilities.zoom_ratio_range =
        optional_vm_value::<CameraZoomRatioRange, _>(binding, context, zoom_ratio_range)?;

    vm_value_from_native(
        context,
        CameraControlCapabilities::from_value(binding, capabilities),
    )
}

/// Read camera control state.
pub(crate) fn destack_device_camera_stream_control_state(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraControlStateVm> {
    let state = <CameraControlState as NativeAbiCodec>::from_value(
        binding,
        CameraControlStateValue {
            exposure_mode: optional_native_value(|| {
                destack_device_camera_stream_exposure_mode(binding, context, handle)
            })?,
            exposure_compensation_ev: optional_native_value(|| {
                destack_device_camera_stream_exposure_compensation(binding, context, handle)
            })?,
            exposure_time_ns: optional_native_value(|| {
                destack_device_camera_stream_exposure_time_ns(binding, context, handle)
            })?,
            sensor_iso: optional_native_value(|| {
                destack_device_camera_stream_sensor_iso(binding, context, handle)
            })?,
            white_balance_mode: optional_native_value(|| {
                destack_device_camera_stream_white_balance_mode(binding, context, handle)
            })?,
            white_balance_kelvin: optional_native_value(|| {
                destack_device_camera_stream_white_balance_kelvin(binding, context, handle)
            })?,
            focus_mode: optional_native_value(|| {
                destack_device_camera_stream_focus_mode(binding, context, handle)
            })?,
            focus_distance_diopters: optional_native_value(|| {
                destack_device_camera_stream_focus_distance_diopters(binding, context, handle)
            })?,
            brightness: optional_native_value(|| {
                destack_device_camera_stream_brightness(binding, context, handle)
            })?,
            contrast: optional_native_value(|| {
                destack_device_camera_stream_contrast(binding, context, handle)
            })?,
            saturation: optional_native_value(|| {
                destack_device_camera_stream_saturation(binding, context, handle)
            })?,
            sharpness: optional_native_value(|| {
                destack_device_camera_stream_sharpness(binding, context, handle)
            })?,
            pan_degrees: optional_native_value(|| {
                destack_device_camera_stream_pan_degrees(binding, context, handle)
            })?,
            tilt_degrees: optional_native_value(|| {
                destack_device_camera_stream_tilt_degrees(binding, context, handle)
            })?,
            zoom_ratio: optional_native_value(|| {
                destack_device_camera_stream_zoom_ratio(binding, context, handle)
            })?,
            stabilization_mode: optional_native_value(|| {
                destack_device_camera_stream_stabilization_mode(binding, context, handle)
            })?,
            torch_mode: optional_native_value(|| {
                destack_device_camera_stream_torch_mode(binding, context, handle)
            })?,
        },
    );

    vm_value_from_native(context, state)
}

/// Read camera exposure compensation.
pub(crate) fn destack_device_camera_stream_exposure_compensation(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<f64> {
    call_out(|out| unsafe {
        device_host::destack_device_camera_stream_exposure_compensation(binding, out, handle)
    })
}

/// Read exposure-compensation range.
pub(crate) fn destack_device_camera_stream_exposure_compensation_range(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraExposureCompensationRangeVm> {
    vm_call_out(
        context,
        |out: *mut CameraExposureCompensationRange| unsafe {
            device_host::destack_device_camera_stream_exposure_compensation_range(
                binding, out, handle,
            )
        },
    )
}

/// Read camera exposure mode.
pub(crate) fn destack_device_camera_stream_exposure_mode(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraExposureMode> {
    call_out(|out| unsafe {
        device_host::destack_device_camera_stream_exposure_mode(binding, out, handle)
    })
}

/// Read camera exposure time.
pub(crate) fn destack_device_camera_stream_exposure_time_ns(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe {
        device_host::destack_device_camera_stream_exposure_time_ns(binding, out, handle)
    })
}

/// Read exposure-time range.
pub(crate) fn destack_device_camera_stream_exposure_time_range(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraExposureTimeRangeVm> {
    vm_call_out(context, |out: *mut CameraExposureTimeRange| unsafe {
        device_host::destack_device_camera_stream_exposure_time_range(binding, out, handle)
    })
}

/// Read camera focus distance.
pub(crate) fn destack_device_camera_stream_focus_distance_diopters(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<f64> {
    call_out(|out| unsafe {
        device_host::destack_device_camera_stream_focus_distance_diopters(binding, out, handle)
    })
}

/// Read focus-distance range.
pub(crate) fn destack_device_camera_stream_focus_distance_range(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraFocusDistanceRangeVm> {
    vm_call_out(context, |out: *mut CameraFocusDistanceRange| unsafe {
        device_host::destack_device_camera_stream_focus_distance_range(binding, out, handle)
    })
}

/// Read camera focus mode.
pub(crate) fn destack_device_camera_stream_focus_mode(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraFocusMode> {
    call_out(|out| unsafe {
        device_host::destack_device_camera_stream_focus_mode(binding, out, handle)
    })
}

/// Read camera brightness.
pub(crate) fn destack_device_camera_stream_brightness(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<f64> {
    call_out(|out| unsafe {
        device_host::destack_device_camera_stream_brightness(binding, out, handle)
    })
}

/// Read brightness range.
pub(crate) fn destack_device_camera_stream_brightness_range(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraFloatControlRangeVm> {
    vm_call_out(context, |out: *mut CameraFloatControlRange| unsafe {
        device_host::destack_device_camera_stream_brightness_range(binding, out, handle)
    })
}

/// Read camera contrast.
pub(crate) fn destack_device_camera_stream_contrast(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<f64> {
    call_out(|out| unsafe {
        device_host::destack_device_camera_stream_contrast(binding, out, handle)
    })
}

/// Read contrast range.
pub(crate) fn destack_device_camera_stream_contrast_range(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraFloatControlRangeVm> {
    vm_call_out(context, |out: *mut CameraFloatControlRange| unsafe {
        device_host::destack_device_camera_stream_contrast_range(binding, out, handle)
    })
}

/// Read camera pan angle.
pub(crate) fn destack_device_camera_stream_pan_degrees(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<f64> {
    call_out(|out| unsafe {
        device_host::destack_device_camera_stream_pan_degrees(binding, out, handle)
    })
}

/// Read pan-angle range.
pub(crate) fn destack_device_camera_stream_pan_range(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraPanAngleRangeVm> {
    vm_call_out(context, |out: *mut CameraPanAngleRange| unsafe {
        device_host::destack_device_camera_stream_pan_range(binding, out, handle)
    })
}

/// Open camera stream.
pub(crate) fn destack_device_camera_stream_open(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    device: resource::CameraDeviceHandle,
    config: CameraStreamConfigVm,
) -> RuntimeResult<resource::CameraStreamHandle> {
    let config = native_value_from_vm(binding, context, config)?;

    call_out(|out| unsafe {
        device_host::destack_device_camera_stream_open(binding, out, device, config)
    })
}

/// Read camera frame.
pub(crate) fn destack_device_camera_stream_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    timeoutns: u64,
) -> RuntimeResult<CameraFrameVm> {
    vm_call_out(context, |out: *mut CameraFrame| unsafe {
        device_host::destack_device_camera_stream_read(binding, out, handle, timeoutns)
    })
}

/// Set camera exposure compensation.
pub(crate) fn destack_device_camera_stream_set_exposure_compensation(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    valueev: f64,
) -> RuntimeResult<()> {
    unsafe {
        device_host::destack_device_camera_stream_set_exposure_compensation(
            binding, handle, valueev,
        )
    }
}

/// Set camera exposure mode.
pub(crate) fn destack_device_camera_stream_set_exposure_mode(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    mode: CameraExposureMode,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_camera_stream_set_exposure_mode(binding, handle, mode) }
}

/// Set camera exposure time.
pub(crate) fn destack_device_camera_stream_set_exposure_time_ns(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    valuens: u64,
) -> RuntimeResult<()> {
    unsafe {
        device_host::destack_device_camera_stream_set_exposure_time_ns(binding, handle, valuens)
    }
}

/// Set camera focus distance.
pub(crate) fn destack_device_camera_stream_set_focus_distance_diopters(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    diopters: f64,
) -> RuntimeResult<()> {
    unsafe {
        device_host::destack_device_camera_stream_set_focus_distance_diopters(
            binding, handle, diopters,
        )
    }
}

/// Set camera focus mode.
pub(crate) fn destack_device_camera_stream_set_focus_mode(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    mode: CameraFocusMode,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_camera_stream_set_focus_mode(binding, handle, mode) }
}

/// Set camera brightness.
pub(crate) fn destack_device_camera_stream_set_brightness(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    argument_value: f64,
) -> RuntimeResult<()> {
    unsafe {
        device_host::destack_device_camera_stream_set_brightness(binding, handle, argument_value)
    }
}

/// Set camera contrast.
pub(crate) fn destack_device_camera_stream_set_contrast(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    argument_value: f64,
) -> RuntimeResult<()> {
    unsafe {
        device_host::destack_device_camera_stream_set_contrast(binding, handle, argument_value)
    }
}

/// Set camera pan angle.
pub(crate) fn destack_device_camera_stream_set_pan_degrees(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    degrees: f64,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_camera_stream_set_pan_degrees(binding, handle, degrees) }
}

/// Set camera saturation.
pub(crate) fn destack_device_camera_stream_set_saturation(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    argument_value: f64,
) -> RuntimeResult<()> {
    unsafe {
        device_host::destack_device_camera_stream_set_saturation(binding, handle, argument_value)
    }
}

/// Set camera sensor ISO.
pub(crate) fn destack_device_camera_stream_set_sensor_iso(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    iso: u32,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_camera_stream_set_sensor_iso(binding, handle, iso) }
}

/// Set camera sharpness.
pub(crate) fn destack_device_camera_stream_set_sharpness(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    argument_value: f64,
) -> RuntimeResult<()> {
    unsafe {
        device_host::destack_device_camera_stream_set_sharpness(binding, handle, argument_value)
    }
}

/// Set camera tilt angle.
pub(crate) fn destack_device_camera_stream_set_tilt_degrees(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    degrees: f64,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_camera_stream_set_tilt_degrees(binding, handle, degrees) }
}

/// Set camera stabilization mode.
pub(crate) fn destack_device_camera_stream_set_stabilization_mode(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    mode: CameraStabilizationMode,
) -> RuntimeResult<()> {
    unsafe {
        device_host::destack_device_camera_stream_set_stabilization_mode(binding, handle, mode)
    }
}

/// Set camera torch mode.
pub(crate) fn destack_device_camera_stream_set_torch_mode(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    mode: CameraTorchMode,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_camera_stream_set_torch_mode(binding, handle, mode) }
}

/// Set camera white balance.
pub(crate) fn destack_device_camera_stream_set_white_balance_kelvin(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    kelvin: u32,
) -> RuntimeResult<()> {
    unsafe {
        device_host::destack_device_camera_stream_set_white_balance_kelvin(binding, handle, kelvin)
    }
}

/// Set camera white-balance mode.
pub(crate) fn destack_device_camera_stream_set_white_balance_mode(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    mode: CameraWhiteBalanceMode,
) -> RuntimeResult<()> {
    unsafe {
        device_host::destack_device_camera_stream_set_white_balance_mode(binding, handle, mode)
    }
}

/// Set camera zoom ratio.
pub(crate) fn destack_device_camera_stream_set_zoom_ratio(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
    ratio: f64,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_camera_stream_set_zoom_ratio(binding, handle, ratio) }
}

/// Read camera stabilization mode.
pub(crate) fn destack_device_camera_stream_stabilization_mode(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraStabilizationMode> {
    call_out(|out| unsafe {
        device_host::destack_device_camera_stream_stabilization_mode(binding, out, handle)
    })
}

/// Start camera stream.
pub(crate) fn destack_device_camera_stream_start(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_camera_stream_start(binding, handle) }
}

/// Stop camera stream.
pub(crate) fn destack_device_camera_stream_stop(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_camera_stream_stop(binding, handle) }
}

/// Read camera torch mode.
pub(crate) fn destack_device_camera_stream_torch_mode(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraTorchMode> {
    call_out(|out| unsafe {
        device_host::destack_device_camera_stream_torch_mode(binding, out, handle)
    })
}

/// Poll camera frame without blocking.
pub(crate) fn destack_device_camera_stream_try_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraFrameVm> {
    vm_call_out(context, |out: *mut CameraFrame| unsafe {
        device_host::destack_device_camera_stream_try_read(binding, out, handle)
    })
}

/// Read camera saturation.
pub(crate) fn destack_device_camera_stream_saturation(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<f64> {
    call_out(|out| unsafe {
        device_host::destack_device_camera_stream_saturation(binding, out, handle)
    })
}

/// Read saturation range.
pub(crate) fn destack_device_camera_stream_saturation_range(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraFloatControlRangeVm> {
    vm_call_out(context, |out: *mut CameraFloatControlRange| unsafe {
        device_host::destack_device_camera_stream_saturation_range(binding, out, handle)
    })
}

/// Read camera sensor ISO.
pub(crate) fn destack_device_camera_stream_sensor_iso(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe {
        device_host::destack_device_camera_stream_sensor_iso(binding, out, handle)
    })
}

/// Read sensor-ISO range.
pub(crate) fn destack_device_camera_stream_sensor_iso_range(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraSensorIsoRangeVm> {
    vm_call_out(context, |out: *mut CameraSensorIsoRange| unsafe {
        device_host::destack_device_camera_stream_sensor_iso_range(binding, out, handle)
    })
}

/// Read camera sharpness.
pub(crate) fn destack_device_camera_stream_sharpness(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<f64> {
    call_out(|out| unsafe {
        device_host::destack_device_camera_stream_sharpness(binding, out, handle)
    })
}

/// Read sharpness range.
pub(crate) fn destack_device_camera_stream_sharpness_range(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraFloatControlRangeVm> {
    vm_call_out(context, |out: *mut CameraFloatControlRange| unsafe {
        device_host::destack_device_camera_stream_sharpness_range(binding, out, handle)
    })
}

/// Read camera tilt angle.
pub(crate) fn destack_device_camera_stream_tilt_degrees(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<f64> {
    call_out(|out| unsafe {
        device_host::destack_device_camera_stream_tilt_degrees(binding, out, handle)
    })
}

/// Read tilt-angle range.
pub(crate) fn destack_device_camera_stream_tilt_range(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraTiltAngleRangeVm> {
    vm_call_out(context, |out: *mut CameraTiltAngleRange| unsafe {
        device_host::destack_device_camera_stream_tilt_range(binding, out, handle)
    })
}

/// Read camera white balance.
pub(crate) fn destack_device_camera_stream_white_balance_kelvin(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe {
        device_host::destack_device_camera_stream_white_balance_kelvin(binding, out, handle)
    })
}

/// Read white-balance range.
pub(crate) fn destack_device_camera_stream_white_balance_range(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraWhiteBalanceRangeVm> {
    vm_call_out(context, |out: *mut CameraWhiteBalanceRange| unsafe {
        device_host::destack_device_camera_stream_white_balance_range(binding, out, handle)
    })
}

/// Read camera white-balance mode.
pub(crate) fn destack_device_camera_stream_white_balance_mode(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraWhiteBalanceMode> {
    call_out(|out| unsafe {
        device_host::destack_device_camera_stream_white_balance_mode(binding, out, handle)
    })
}

/// Read camera zoom ratio.
pub(crate) fn destack_device_camera_stream_zoom_ratio(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<f64> {
    call_out(|out| unsafe {
        device_host::destack_device_camera_stream_zoom_ratio(binding, out, handle)
    })
}

/// Read zoom-ratio range.
pub(crate) fn destack_device_camera_stream_zoom_ratio_range(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<CameraZoomRatioRangeVm> {
    vm_call_out(context, |out: *mut CameraZoomRatioRange| unsafe {
        device_host::destack_device_camera_stream_zoom_ratio_range(binding, out, handle)
    })
}

/// Close serial endpoint.
pub(crate) fn destack_device_serial_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_serial_close(binding, handle) }
}

/// Read serial endpoint configuration.
pub(crate) fn destack_device_serial_config(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<SerialPortConfigVm> {
    vm_call_out(context, |out: *mut SerialPortConfig| unsafe {
        device_host::destack_device_serial_config(binding, out, handle)
    })
}

/// Reconfigure serial endpoint.
pub(crate) fn destack_device_serial_configure(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::SerialPortHandle,
    config: SerialPortConfigVm,
) -> RuntimeResult<()> {
    let config: SerialPortConfig = native_value_from_vm(binding, context, config)?;

    unsafe { device_host::destack_device_serial_configure(binding, handle, config) }
}

/// Read serial endpoint descriptor.
pub(crate) fn destack_device_serial_descriptor(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<SerialPortDescriptorVm> {
    vm_call_out(context, |out: *mut SerialPortDescriptor| unsafe {
        device_host::destack_device_serial_descriptor(binding, out, handle)
    })
}

/// Discard queued inbound serial bytes.
pub(crate) fn destack_device_serial_discard_input(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_serial_discard_input(binding, handle) }
}

/// Discard queued outbound serial bytes.
pub(crate) fn destack_device_serial_discard_output(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_serial_discard_output(binding, handle) }
}

/// Drain serial output.
pub(crate) fn destack_device_serial_drain(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_serial_drain(binding, handle) }
}

/// Read serial input signal state.
pub(crate) fn destack_device_serial_get_signals(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<SerialInputSignalsVm> {
    vm_call_out(context, |out: *mut SerialInputSignals| unsafe {
        device_host::destack_device_serial_get_signals(binding, out, handle)
    })
}

/// List serial endpoints.
pub(crate) fn destack_device_serial_list(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<SerialPortDescriptorVm>> {
    vm_call_out(
        context,
        |out: *mut NativeSlice<SerialPortDescriptor>| unsafe {
            device_host::destack_device_serial_list(binding, out)
        },
    )
}

/// Close a serial topology watch stream.
pub(crate) fn destack_device_serial_watch_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SerialWatchHandle,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_serial_watch_close(binding, handle) }
}

/// Open a serial topology watch stream.
pub(crate) fn destack_device_serial_watch_open(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::SerialWatchHandle> {
    call_out(|out| unsafe { device_host::destack_device_serial_watch_open(binding, out) })
}

/// Read one serial topology event.
pub(crate) fn destack_device_serial_watch_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::SerialWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<SerialWatchEventVm> {
    vm_call_out(context, |out: *mut SerialWatchEvent| unsafe {
        device_host::destack_device_serial_watch_read(binding, out, handle, timeoutns)
    })
}

/// Poll one serial topology event without blocking.
pub(crate) fn destack_device_serial_watch_try_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::SerialWatchHandle,
) -> RuntimeResult<SerialWatchEventVm> {
    vm_call_out(context, |out: *mut SerialWatchEvent| unsafe {
        device_host::destack_device_serial_watch_try_read(binding, out, handle)
    })
}

/// Open serial endpoint.
pub(crate) fn destack_device_serial_open(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    id: vm::StringHandle,
    options: SerialPortOpenOptionsVm,
) -> RuntimeResult<resource::SerialPortHandle> {
    let id = native_value_from_vm(binding, context, id)?;
    let options: SerialPortOpenOptions = native_value_from_vm(binding, context, options)?;

    call_out(|out| unsafe { device_host::destack_device_serial_open(binding, out, id, options) })
}

/// Wait for one serial event.
pub(crate) fn destack_device_serial_read_event(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::SerialPortHandle,
    timeoutns: u64,
) -> RuntimeResult<SerialEventVm> {
    vm_call_out(context, |out: *mut SerialEvent| unsafe {
        device_host::destack_device_serial_read_event(binding, out, handle, timeoutns)
    })
}

/// Read serial bytes.
pub(crate) fn destack_device_serial_read_into(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::SerialPortHandle,
    buffer: VmSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<u64> {
    // copy the VM buffer into host memory for the read call
    let mut bytes = buffer.read_bytes(&context.read())?;

    // invoke the host read and then copy the results back into VM memory
    let read = call_out(|out| {
        let buffer = native_bytes_from_vec(&mut bytes);
        unsafe {
            device_host::destack_device_serial_read_into(binding, out, handle, buffer, timeoutns)
        }
    })?;
    buffer.write_bytes(&mut context.write(), &bytes)?;

    Ok(read)
}

/// Update serial output signal state.
pub(crate) fn destack_device_serial_set_signals(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::SerialPortHandle,
    signals: SerialOutputSignalsVm,
) -> RuntimeResult<()> {
    let signals: SerialOutputSignals = native_value_from_vm(binding, context, signals)?;

    unsafe { device_host::destack_device_serial_set_signals(binding, handle, signals) }
}

/// Poll one serial event without blocking.
pub(crate) fn destack_device_serial_try_read_event(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::SerialPortHandle,
) -> RuntimeResult<SerialEventVm> {
    vm_call_out(context, |out: *mut SerialEvent| unsafe {
        device_host::destack_device_serial_try_read_event(binding, out, handle)
    })
}

/// Poll serial bytes without blocking.
pub(crate) fn destack_device_serial_try_read_into(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::SerialPortHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    // copy the VM buffer into host memory for the poll call
    let mut bytes = buffer.read_bytes(&context.read())?;

    // invoke the host read and then copy the results back into VM memory
    let read = call_out(|out| {
        let buffer = native_bytes_from_vec(&mut bytes);
        unsafe { device_host::destack_device_serial_try_read_into(binding, out, handle, buffer) }
    })?;
    buffer.write_bytes(&mut context.write(), &bytes)?;

    Ok(read)
}

/// Write serial bytes.
pub(crate) fn destack_device_serial_write(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::SerialPortHandle,
    data: VmSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<u64> {
    let data: NativeSlice<u8> = native_value_from_vm(binding, context, data)?;

    call_out(|out| unsafe {
        device_host::destack_device_serial_write(binding, out, handle, data, timeoutns)
    })
}

/// List USB BOS capabilities.
pub(crate) fn destack_device_usb_bos_capability_list(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<VmSlice<UsbBosCapabilityDescriptorVm>> {
    vm_call_out(
        context,
        |out: *mut NativeSlice<UsbBosCapabilityDescriptor>| unsafe {
            device_host::destack_device_usb_bos_capability_list(binding, out, handle)
        },
    )
}

/// Read bulk endpoint bytes.
pub(crate) fn destack_device_usb_bulk_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
    endpoint: UsbEndpointSelectorVm,
    maxbytes: u32,
    timeoutns: u64,
) -> RuntimeResult<UsbInTransferResultVm> {
    let endpoint: UsbEndpointSelector = native_value_from_vm(binding, context, endpoint)?;

    vm_call_out(context, |out: *mut UsbInTransferResult| unsafe {
        device_host::destack_device_usb_bulk_read(
            binding, out, handle, endpoint, maxbytes, timeoutns,
        )
    })
}

/// Write bulk endpoint bytes.
pub(crate) fn destack_device_usb_bulk_write(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
    endpoint: UsbEndpointSelectorVm,
    argument_bytes: VmSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<UsbOutTransferResultVm> {
    let endpoint: UsbEndpointSelector = native_value_from_vm(binding, context, endpoint)?;
    let argument_bytes: NativeSlice<u8> = native_value_from_vm(binding, context, argument_bytes)?;

    vm_call_out(context, |out: *mut UsbOutTransferResult| unsafe {
        device_host::destack_device_usb_bulk_write(
            binding,
            out,
            handle,
            endpoint,
            argument_bytes,
            timeoutns,
        )
    })
}

/// Claim USB interface.
pub(crate) fn destack_device_usb_claim_interface(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
    interfacenumber: u8,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_usb_claim_interface(binding, handle, interfacenumber) }
}

/// Clear halt condition on one endpoint.
pub(crate) fn destack_device_usb_clear_halt(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
    endpoint: UsbEndpointSelectorVm,
) -> RuntimeResult<()> {
    let endpoint: UsbEndpointSelector = native_value_from_vm(binding, context, endpoint)?;

    unsafe { device_host::destack_device_usb_clear_halt(binding, handle, endpoint) }
}

/// Close USB device.
pub(crate) fn destack_device_usb_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_usb_close(binding, handle) }
}

/// Read active USB configuration value.
pub(crate) fn destack_device_usb_configuration_get(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<u8> {
    call_out(|out| unsafe {
        device_host::destack_device_usb_configuration_get(binding, out, handle)
    })
}

/// List USB configurations.
pub(crate) fn destack_device_usb_configuration_list(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<VmSlice<UsbConfigurationDescriptorVm>> {
    vm_call_out(
        context,
        |out: *mut NativeSlice<UsbConfigurationDescriptor>| unsafe {
            device_host::destack_device_usb_configuration_list(binding, out, handle)
        },
    )
}

/// Set active USB configuration value.
pub(crate) fn destack_device_usb_configuration_set(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
    configurationvalue: u8,
) -> RuntimeResult<()> {
    unsafe {
        device_host::destack_device_usb_configuration_set(binding, handle, configurationvalue)
    }
}

/// Read control-transfer response bytes.
pub(crate) fn destack_device_usb_control_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
    setup: UsbControlSetupVm,
    timeoutns: u64,
) -> RuntimeResult<UsbInTransferResultVm> {
    let setup: UsbControlSetup = native_value_from_vm(binding, context, setup)?;

    vm_call_out(context, |out: *mut UsbInTransferResult| unsafe {
        device_host::destack_device_usb_control_read(binding, out, handle, setup, timeoutns)
    })
}

/// Write control-transfer request bytes.
pub(crate) fn destack_device_usb_control_write(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
    setup: UsbControlSetupVm,
    argument_bytes: VmSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<UsbOutTransferResultVm> {
    let setup: UsbControlSetup = native_value_from_vm(binding, context, setup)?;
    let argument_bytes: NativeSlice<u8> = native_value_from_vm(binding, context, argument_bytes)?;

    vm_call_out(context, |out: *mut UsbOutTransferResult| unsafe {
        device_host::destack_device_usb_control_write(
            binding,
            out,
            handle,
            setup,
            argument_bytes,
            timeoutns,
        )
    })
}

/// Read active USB device descriptor.
pub(crate) fn destack_device_usb_descriptor(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<UsbDeviceDescriptorVm> {
    vm_call_out(context, |out: *mut UsbDeviceDescriptor| unsafe {
        device_host::destack_device_usb_descriptor(binding, out, handle)
    })
}

/// Read interrupt endpoint bytes.
pub(crate) fn destack_device_usb_interrupt_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
    endpoint: UsbEndpointSelectorVm,
    maxbytes: u32,
    timeoutns: u64,
) -> RuntimeResult<UsbInTransferResultVm> {
    let endpoint: UsbEndpointSelector = native_value_from_vm(binding, context, endpoint)?;

    vm_call_out(context, |out: *mut UsbInTransferResult| unsafe {
        device_host::destack_device_usb_interrupt_read(
            binding, out, handle, endpoint, maxbytes, timeoutns,
        )
    })
}

/// Write interrupt endpoint bytes.
pub(crate) fn destack_device_usb_interrupt_write(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
    endpoint: UsbEndpointSelectorVm,
    argument_bytes: VmSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<UsbOutTransferResultVm> {
    let endpoint: UsbEndpointSelector = native_value_from_vm(binding, context, endpoint)?;
    let argument_bytes: NativeSlice<u8> = native_value_from_vm(binding, context, argument_bytes)?;

    vm_call_out(context, |out: *mut UsbOutTransferResult| unsafe {
        device_host::destack_device_usb_interrupt_write(
            binding,
            out,
            handle,
            endpoint,
            argument_bytes,
            timeoutns,
        )
    })
}

/// Read one isochronous transfer.
pub(crate) fn destack_device_usb_isochronous_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
    endpoint: UsbEndpointSelectorVm,
    packetsizes: VmSlice<u32>,
    timeoutns: u64,
) -> RuntimeResult<UsbIsochronousTransferResultVm> {
    let endpoint: UsbEndpointSelector = native_value_from_vm(binding, context, endpoint)?;
    let packetsizes: NativeSlice<u32> = native_value_from_vm(binding, context, packetsizes)?;

    vm_call_out(context, |out: *mut UsbIsochronousTransferResult| unsafe {
        device_host::destack_device_usb_isochronous_read(
            binding,
            out,
            handle,
            endpoint,
            packetsizes,
            timeoutns,
        )
    })
}

/// Write one isochronous transfer.
pub(crate) fn destack_device_usb_isochronous_write(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
    endpoint: UsbEndpointSelectorVm,
    argument_bytes: VmSlice<u8>,
    packetsizes: VmSlice<u32>,
    timeoutns: u64,
) -> RuntimeResult<UsbIsochronousTransferResultVm> {
    let endpoint: UsbEndpointSelector = native_value_from_vm(binding, context, endpoint)?;
    let argument_bytes: NativeSlice<u8> = native_value_from_vm(binding, context, argument_bytes)?;
    let packetsizes: NativeSlice<u32> = native_value_from_vm(binding, context, packetsizes)?;

    vm_call_out(context, |out: *mut UsbIsochronousTransferResult| unsafe {
        device_host::destack_device_usb_isochronous_write(
            binding,
            out,
            handle,
            endpoint,
            argument_bytes,
            packetsizes,
            timeoutns,
        )
    })
}

/// List USB devices.
pub(crate) fn destack_device_usb_list(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<UsbDeviceDescriptorVm>> {
    vm_call_out(
        context,
        |out: *mut NativeSlice<UsbDeviceDescriptor>| unsafe {
            device_host::destack_device_usb_list(binding, out)
        },
    )
}

/// Open USB device.
pub(crate) fn destack_device_usb_open(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<resource::UsbDeviceHandle> {
    let id = native_value_from_vm(binding, context, id)?;

    call_out(|out| unsafe { device_host::destack_device_usb_open(binding, out, id) })
}

/// Release USB interface.
pub(crate) fn destack_device_usb_release_interface(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
    interfacenumber: u8,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_usb_release_interface(binding, handle, interfacenumber) }
}

/// Reset one USB device.
pub(crate) fn destack_device_usb_reset(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_usb_reset(binding, handle) }
}

/// Set USB interface alternate setting.
pub(crate) fn destack_device_usb_set_interface_alternate_setting(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
    interfacenumber: u8,
    alternatesetting: u8,
) -> RuntimeResult<()> {
    unsafe {
        device_host::destack_device_usb_set_interface_alternate_setting(
            binding,
            handle,
            interfacenumber,
            alternatesetting,
        )
    }
}

/// Read USB string descriptors.
pub(crate) fn destack_device_usb_string_descriptor(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
    languageid: u16,
) -> RuntimeResult<UsbStringDescriptorVm> {
    vm_call_out(context, |out: *mut UsbStringDescriptor| unsafe {
        device_host::destack_device_usb_string_descriptor(binding, out, handle, languageid)
    })
}

/// List USB string-descriptor language identifiers.
pub(crate) fn destack_device_usb_string_language_list(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<VmSlice<u16>> {
    vm_call_out(context, |out: *mut NativeSlice<u16>| unsafe {
        device_host::destack_device_usb_string_language_list(binding, out, handle)
    })
}

/// Cancel pending transfers on one endpoint.
pub(crate) fn destack_device_usb_transfer_cancel(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
    endpoint: UsbEndpointSelectorVm,
) -> RuntimeResult<()> {
    let endpoint: UsbEndpointSelector = native_value_from_vm(binding, context, endpoint)?;

    unsafe { device_host::destack_device_usb_transfer_cancel(binding, handle, endpoint) }
}

/// Cancel all pending transfers on one opened USB device.
pub(crate) fn destack_device_usb_transfer_cancel_all(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::UsbDeviceHandle,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_usb_transfer_cancel_all(binding, handle) }
}

/// Close USB hotplug watch stream.
pub(crate) fn destack_device_usb_watch_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::UsbWatchHandle,
) -> RuntimeResult<()> {
    unsafe { device_host::destack_device_usb_watch_close(binding, handle) }
}

/// Open USB hotplug watch stream.
pub(crate) fn destack_device_usb_watch_open(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::UsbWatchHandle> {
    call_out(|out| unsafe { device_host::destack_device_usb_watch_open(binding, out) })
}

/// Wait for one USB hotplug event.
pub(crate) fn destack_device_usb_watch_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::UsbWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<UsbHotplugEventVm> {
    vm_call_out(context, |out: *mut UsbHotplugEvent| unsafe {
        device_host::destack_device_usb_watch_read(binding, out, handle, timeoutns)
    })
}

/// Poll one USB hotplug event without blocking.
pub(crate) fn destack_device_usb_watch_try_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::UsbWatchHandle,
) -> RuntimeResult<UsbHotplugEventVm> {
    vm_call_out(context, |out: *mut UsbHotplugEvent| unsafe {
        device_host::destack_device_usb_watch_try_read(binding, out, handle)
    })
}
