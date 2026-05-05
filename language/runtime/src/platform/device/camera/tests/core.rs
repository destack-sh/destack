use std::collections::BTreeSet;
use std::sync::{Mutex, OnceLock};

use crate::diagnostic::RuntimeResult;
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::device::tests::{
    DeviceHarnessContext, HarnessValue, assert_ok_or_expected_error, vm_context_mut,
    with_harness_context, with_native_context,
};
use crate::platform::device::{
    CameraDeviceDescriptor, CameraDeviceDescriptorValue, CameraDeviceDescriptorVm,
    CameraFacingMode, CameraStreamCapabilityValue, CameraWatchEvent, CameraWatchEventValue,
    CameraWatchEventVm, native as device_native, vm as device_vm,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{CameraDeviceHandle, CameraStreamHandle};
use crate::platform::{NativeAbiCodec, VmAbiCodec, VmSlice};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// Return one process-global serialization lock for camera hardware tests.
#[cfg(any(unix, windows))]
fn camera_test_lock() -> &'static Mutex<()> {
    static CAMERA_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    CAMERA_TEST_LOCK.get_or_init(|| Mutex::new(()))
}

/// Assert one camera result is success, not-supported, or permission-gated.
pub(super) fn assert_camera_supported_not_supported_or_permission<T>(
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

/// Assert one nonblocking frame read succeeds or reports `ioWouldBlock`.
pub(super) fn assert_camera_frame_or_would_block<T>(
    result: Result<T, Box<crate::diagnostic::RuntimeError>>,
) -> crate::diagnostic::RuntimeResult<Option<T>> {
    assert_ok_or_expected_error(result, &[PlatformErrorCode::IoWouldBlock])
}

/// Decode one camera descriptor list through the active harness lane.
#[cfg(any(unix, windows))]
pub(super) fn camera_device_descriptor_list_value(
    context: &mut DeviceHarnessContext<'_>,
    listed: HarnessValue<NativeSlice<CameraDeviceDescriptor>, VmSlice<CameraDeviceDescriptorVm>>,
) -> RuntimeResult<Vec<CameraDeviceDescriptorValue>> {
    context.harness_value_into(listed)
}

/// Decode one camera watch event through the active harness lane.
#[cfg(any(unix, windows))]
pub(super) fn camera_watch_event_value(
    context: &mut DeviceHarnessContext<'_>,
    event: HarnessValue<CameraWatchEvent, CameraWatchEventVm>,
) -> RuntimeResult<CameraWatchEventValue> {
    context.harness_value_into(event)
}

/// Assert that one camera descriptor is structurally valid.
#[cfg(any(unix, windows))]
pub(super) fn assert_camera_device_descriptor_shape(descriptor: &CameraDeviceDescriptorValue) {
    assert!(!descriptor.id.is_empty());
    assert!(!descriptor.name.is_empty());
}

/// Assert that one camera descriptor is self consistent and uniquely identified.
#[cfg(any(unix, windows))]
pub(super) fn assert_camera_device_descriptor_invariants(
    descriptor: &CameraDeviceDescriptorValue,
    ids: &mut BTreeSet<String>,
) {
    assert_camera_device_descriptor_shape(descriptor);

    assert!(ids.insert(descriptor.id.clone()));
}

/// Run one native camera test while holding the process-global camera lock.
#[cfg(any(unix, windows))]
pub(super) fn with_camera_native_context<F>(callback: F)
where
    F: for<'call> FnOnce(&'call BindingCallContext) -> RuntimeResult<()>,
{
    let _lock = camera_test_lock().lock().expect("camera test lock");
    with_native_context(callback);
}

/// Run one camera harness test while holding the process-global camera lock.
#[cfg(any(unix, windows))]
pub(super) fn with_camera_harness_context<F>(callback: F)
where
    F: for<'call> FnMut(DeviceHarnessContext<'call>) -> RuntimeResult<()>,
{
    let _lock = camera_test_lock().lock().expect("camera test lock");
    with_harness_context(callback);
}

/// Run one callback against the first openable non-external native camera stream when present.
#[cfg(any(unix, windows))]
pub(super) fn with_first_openable_local_camera_stream_native<F>(
    call_context: &BindingCallContext,
    callback: F,
) -> RuntimeResult<()>
where
    F: FnMut(
        CameraDeviceHandle,
        CameraStreamHandle,
        CameraStreamCapabilityValue,
    ) -> RuntimeResult<()>,
{
    with_openable_camera_stream_native(call_context, false, callback)
}

/// Run one callback against the first openable native camera stream with one external-device policy.
#[cfg(any(unix, windows))]
fn with_openable_camera_stream_native<F>(
    call_context: &BindingCallContext,
    is_external_allowed: bool,
    mut callback: F,
) -> RuntimeResult<()>
where
    F: FnMut(
        CameraDeviceHandle,
        CameraStreamHandle,
        CameraStreamCapabilityValue,
    ) -> RuntimeResult<()>,
{
    let mut listed_out = std::mem::MaybeUninit::uninit();
    let result = unsafe {
        device_native::destack_device_camera_device_list(call_context, listed_out.as_mut_ptr())
    };
    let listed = match assert_camera_supported_not_supported_or_permission(result)? {
        Some(()) => unsafe { listed_out.assume_init() },
        None => return Ok(()),
    };
    let listed = unsafe { listed.as_slice()? };

    // prefer built-in or host-local cameras before external continuity paths
    for is_external in [false, true] {
        if is_external && !is_external_allowed {
            continue;
        }

        for device in listed {
            if (device.facing_mode == CameraFacingMode::External) != is_external {
                continue;
            }

            let device_id = unsafe { device.id.as_str()? };
            let mut device_out = std::mem::MaybeUninit::uninit();
            let opened = unsafe {
                device_native::destack_device_camera_device_open(
                    call_context,
                    device_out.as_mut_ptr(),
                    NativeStringRef::from(device_id),
                )
            };
            if opened.is_err() {
                continue;
            }

            let device_handle = unsafe { device_out.assume_init() };
            let mut capabilities_out = std::mem::MaybeUninit::uninit();
            let listed_capabilities = unsafe {
                device_native::destack_device_camera_device_stream_capability_list(
                    call_context,
                    capabilities_out.as_mut_ptr(),
                    device_handle,
                )
            };
            let listed_capabilities = match listed_capabilities {
                Ok(()) => unsafe { capabilities_out.assume_init() },
                Err(_) => {
                    close_camera_device(call_context, device_handle);
                    continue;
                }
            };
            let listed_capabilities = unsafe { listed_capabilities.as_slice()? };

            for capability in listed_capabilities.iter().copied() {
                let capability = unsafe { capability.into_value()? };
                let mut stream_out = std::mem::MaybeUninit::uninit();
                let opened = unsafe {
                    device_native::destack_device_camera_stream_open(
                        call_context,
                        stream_out.as_mut_ptr(),
                        device_handle,
                        capability.config,
                    )
                };
                if opened.is_err() {
                    continue;
                }

                let stream_handle = unsafe { stream_out.assume_init() };
                let result = callback(device_handle, stream_handle, capability);
                close_camera_stream(call_context, stream_handle);
                close_camera_device(call_context, device_handle);
                return result;
            }

            close_camera_device(call_context, device_handle);
        }
    }

    Ok(())
}

/// Run one callback against the first openable non-external VM camera stream when present.
#[cfg(any(unix, windows))]
pub(super) fn with_first_openable_local_camera_stream_vm<F>(
    context: &DeviceHarnessContext<'_>,
    callback: F,
) -> RuntimeResult<()>
where
    F: FnMut(
        &mut vm::BindingContext<'_>,
        CameraDeviceHandle,
        CameraStreamHandle,
        CameraStreamCapabilityValue,
    ) -> RuntimeResult<()>,
{
    with_openable_camera_stream_vm(context, false, callback)
}

/// Run one callback against the first openable VM camera stream with one external-device policy.
#[cfg(any(unix, windows))]
fn with_openable_camera_stream_vm<F>(
    context: &DeviceHarnessContext<'_>,
    is_external_allowed: bool,
    mut callback: F,
) -> RuntimeResult<()>
where
    F: FnMut(
        &mut vm::BindingContext<'_>,
        CameraDeviceHandle,
        CameraStreamHandle,
        CameraStreamCapabilityValue,
    ) -> RuntimeResult<()>,
{
    let Some(vm_context) = vm_context_mut(context) else {
        return Ok(());
    };

    let listed = match assert_camera_supported_not_supported_or_permission(
        device_vm::destack_device_camera_device_list(context.call_context, vm_context),
    )? {
        Some(listed) => listed,
        None => return Ok(()),
    };
    let listed = listed.read_values(&vm_context.read())?;

    // prefer built-in or host-local cameras before external continuity paths
    for is_external in [false, true] {
        if is_external && !is_external_allowed {
            continue;
        }

        for device in &listed {
            if (device.facing_mode == CameraFacingMode::External) != is_external {
                continue;
            }

            let device_id = vm_context.string_ref(device.id)?.as_str().to_string();
            let device_id = vm::StringHandle::new(vm_context.intern_string(&device_id)?);
            let opened = device_vm::destack_device_camera_device_open(
                context.call_context,
                vm_context,
                device_id,
            );
            let device_handle = match opened {
                Ok(handle) => handle,
                Err(_) => continue,
            };

            let listed_capabilities =
                device_vm::destack_device_camera_device_stream_capability_list(
                    context.call_context,
                    vm_context,
                    device_handle,
                );
            let listed_capabilities = match listed_capabilities {
                Ok(capabilities) => capabilities.read_values(&vm_context.read())?,
                Err(_) => {
                    let _ = device_vm::destack_device_camera_device_close(
                        context.call_context,
                        vm_context,
                        device_handle,
                    );
                    continue;
                }
            };

            for capability in listed_capabilities {
                let capability = capability.into_value(&vm_context.read())?;
                let opened = device_vm::destack_device_camera_stream_open(
                    context.call_context,
                    vm_context,
                    device_handle,
                    capability.config,
                );
                let stream_handle = match opened {
                    Ok(handle) => handle,
                    Err(_) => continue,
                };

                let result = callback(vm_context, device_handle, stream_handle, capability);
                let _ = device_vm::destack_device_camera_stream_close(
                    context.call_context,
                    vm_context,
                    stream_handle,
                );
                let _ = device_vm::destack_device_camera_device_close(
                    context.call_context,
                    vm_context,
                    device_handle,
                );
                return result;
            }

            let _ = device_vm::destack_device_camera_device_close(
                context.call_context,
                vm_context,
                device_handle,
            );
        }
    }

    Ok(())
}

/// Close one opened camera device during test cleanup.
#[cfg(any(unix, windows))]
pub(super) fn close_camera_device(call_context: &BindingCallContext, handle: CameraDeviceHandle) {
    unsafe {
        drop(device_native::destack_device_camera_device_close(
            call_context,
            handle,
        ));
    }
}

/// Close one opened camera stream during test cleanup.
#[cfg(any(unix, windows))]
pub(super) fn close_camera_stream(call_context: &BindingCallContext, handle: CameraStreamHandle) {
    unsafe {
        drop(device_native::destack_device_camera_stream_close(
            call_context,
            handle,
        ));
    }
}
