use super::core::*;
use super::metadata::*;

/// List visible Windows camera devices.
pub(crate) unsafe fn destack_device_camera_device_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CameraDeviceDescriptor>,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // enumerate visible source groups
    let groups = MediaFrameSourceGroup::FindAllAsync()
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.device.list",
                "MediaFrameSourceGroup::FindAllAsync",
                &error,
            )
        })?
        .get()
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.device.list",
                "IAsyncOperation::get",
                &error,
            )
        })?;

    // collect one descriptor per visible color source group
    let mut descriptors = Vec::new();
    for group in &groups {
        let source_infos = group.SourceInfos().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.device.list",
                "MediaFrameSourceGroup::SourceInfos",
                &error,
            )
        })?;

        for source_info in &source_infos {
            if !is_color_camera_source(&source_info) {
                continue;
            }

            let descriptor = descriptor_from_source_group(&group, &source_info)?;
            descriptors.push(camera_descriptor_from_value(binding, descriptor));
            break;
        }
    }

    // store the descriptor slice
    unsafe {
        out.write(binding.store_slice(descriptors));
    }

    Ok(())
}

/// Open one Windows camera device.
pub(crate) unsafe fn destack_device_camera_device_open(
    binding: &BindingCallContext,
    out: *mut resource::CameraDeviceHandle,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected source group
    let id = unsafe { id.as_str()? };
    let source_group_id = parse_camera_id(id)?;
    let source_group = MediaFrameSourceGroup::FromIdAsync(&HSTRING::from(source_group_id))
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.device.open",
                "MediaFrameSourceGroup::FromIdAsync",
                &error,
            )
        })?
        .get()
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.device.open",
                "IAsyncOperation::get",
                &error,
            )
        })?;

    // initialize the shared capture session
    let capture = open_media_capture(&source_group, "destack.device.camera.device.open")?;
    let info = descriptor_info_from_capture(&capture, &source_group)?;
    let capture_reference = AgileReference::new(&capture).map_err(|error| {
        windows_camera_error(
            "destack.device.camera.device.open",
            "AgileReference::new",
            &error,
        )
    })?;

    // store the opened device snapshot
    let entry = ResourceEntry::new(ResourceKind::CameraDevice)
        .with_label(CAMERA_DEVICE_RESOURCE_LABEL)
        .with_payload(Arc::new(WindowsCameraDeviceResource {
            capture: capture_reference.clone(),
            info,
        }))
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            WindowsCameraDeviceFinalizer {
                capture: capture_reference,
            },
        ));
    let handle = binding
        .worker()
        .resources
        .insert(&binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::CameraDeviceHandle(handle));
    }

    Ok(())
}

/// Close one Windows camera device.
pub(crate) unsafe fn destack_device_camera_device_close(
    binding: &BindingCallContext,
    handle: resource::CameraDeviceHandle,
) -> RuntimeResult<()> {
    close_camera_device_resource(binding, handle, "destack.device.camera.device.close")
}

/// List supported stream capabilities for one opened Windows camera device.
pub(crate) unsafe fn destack_device_camera_device_stream_capability_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CameraStreamCapability>,
    handle: resource::CameraDeviceHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the opened device snapshot
    let resource = camera_device_resource::<WindowsCameraDeviceResource>(
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
