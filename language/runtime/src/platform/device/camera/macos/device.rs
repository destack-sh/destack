#![allow(unsafe_op_in_unsafe_fn)]

use super::core::*;
use super::metadata::*;

/// List the macOS camera devices exposed to the runtime.
pub(crate) unsafe fn destack_device_camera_device_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CameraDeviceDescriptor>,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // enumerate visible capture devices
    let mut descriptors = Vec::new();
    for device in camera_devices("destack.device.camera.device.list")?.iter() {
        let descriptor = camera_descriptor_from_device(&device);
        descriptors.push(camera_descriptor_from_value(binding, descriptor));
    }

    // store the descriptor slice
    unsafe {
        out.write(binding.store_slice(descriptors));
    }

    Ok(())
}

/// Open one macOS camera device session.
pub(crate) unsafe fn destack_device_camera_device_open(
    binding: &BindingCallContext,
    out: *mut resource::CameraDeviceHandle,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected device
    let id = unsafe { id.as_str()? };
    let device = camera_device_by_id(id, "destack.device.camera.device.open")?;
    let info = camera_descriptor_info_from_device(&device);

    // store the opened device resource
    let entry = ResourceEntry::new(ResourceKind::CameraDevice)
        .with_label(CAMERA_DEVICE_RESOURCE_LABEL)
        .with_payload(Arc::new(MacosCameraDeviceResource { info }))
        .with_finalizer(
            binding
                .worker()
                .platform_state
                .device
                .retain_runtime_activity(),
        );
    let handle = binding
        .worker()
        .resources
        .insert(&binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::CameraDeviceHandle(handle));
    }

    Ok(())
}

/// Close one macOS camera device session.
pub(crate) unsafe fn destack_device_camera_device_close(
    binding: &BindingCallContext,
    handle: resource::CameraDeviceHandle,
) -> RuntimeResult<()> {
    close_camera_device_resource(binding, handle, "destack.device.camera.device.close")
}

/// List one macOS camera device's stream capabilities.
pub(crate) unsafe fn destack_device_camera_device_stream_capability_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CameraStreamCapability>,
    handle: resource::CameraDeviceHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the opened device snapshot
    let resource = camera_device_resource::<MacosCameraDeviceResource>(
        binding,
        handle,
        "destack.device.camera.device.streamCapabilityList",
    )?;

    // store the cached capability list
    unsafe {
        out.write(store_camera_capabilities(
            binding,
            &resource.info.capabilities,
        ));
    }

    Ok(())
}
