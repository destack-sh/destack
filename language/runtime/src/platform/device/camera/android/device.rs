use super::codec::*;

/// List visible Android camera devices.
pub(crate) unsafe fn destack_device_camera_device_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CameraDeviceDescriptor>,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // host descriptors
    let descriptors = read_camera_device_descriptors(binding, "destack.device.camera.deviceList")?;
    let descriptors = descriptors
        .into_iter()
        .map(|descriptor| camera_descriptor_from_value(binding, descriptor))
        .collect::<Vec<_>>();

    // binding store
    unsafe {
        out.write(binding.store_slice(descriptors));
    }

    Ok(())
}

/// Open one Android camera device session.
pub(crate) unsafe fn destack_device_camera_device_open(
    binding: &BindingCallContext,
    out: *mut resource::CameraDeviceHandle,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // host open
    let runtime_id = host_session_id(binding, "destack.device.camera.device.open")?;
    let id = unsafe { id.as_str()? };
    let mut session_id = 0u64;
    let status = unsafe {
        destack_host_android_camera_device_open(
            runtime_id,
            NativeStringRef::from(id),
            &mut session_id,
        )
    };
    host_status_result(
        status,
        "destack.device.camera.device.open",
        "camera device open",
    )?;

    // cache metadata
    let configs = read_camera_stream_configs(
        binding,
        session_id,
        "destack.device.camera.device.streamConfigList",
    )?;
    let capabilities = read_camera_stream_capabilities(
        binding,
        session_id,
        "destack.device.camera.device.streamCapabilityList",
    )?;

    // resource store
    let resource = Arc::new(AndroidCameraDeviceResource {
        session_id,
        configs,
        capabilities,
    });
    let entry = ResourceEntry::new(ResourceKind::CameraDevice)
        .with_label(CAMERA_DEVICE_RESOURCE_LABEL)
        .with_payload(resource)
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            AndroidCameraDeviceFinalizer {
                runtime_id,
                session_id,
            },
        ));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::CameraDeviceHandle(resource_id));
    }

    Ok(())
}

/// Close one Android camera device session.
pub(crate) unsafe fn destack_device_camera_device_close(
    binding: &BindingCallContext,
    handle: resource::CameraDeviceHandle,
) -> RuntimeResult<()> {
    close_camera_device_resource(binding, handle, "destack.device.camera.device.close")
}

/// List supported stream capabilities for one opened Android camera device.
pub(crate) unsafe fn destack_device_camera_device_stream_capability_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CameraStreamCapability>,
    handle: resource::CameraDeviceHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // cached resource
    let resource = camera_device_resource::<AndroidCameraDeviceResource>(
        binding,
        handle,
        "destack.device.camera.device.streamCapabilityList",
    )?;

    // binding store
    unsafe {
        out.write(store_camera_capabilities(binding, &resource.capabilities));
    }

    Ok(())
}
