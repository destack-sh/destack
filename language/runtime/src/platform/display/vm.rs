use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::VmAbi;
use crate::platform::display::{
    DisplayBackendDescriptor, DisplayBackendDescriptorVm, DisplayDescriptor, DisplayDescriptorVm,
    DisplayGammaRamp, DisplayGammaRampVm, DisplayMode, DisplayModeVm, DisplayMonitorEvent,
    DisplayMonitorEventFilter, DisplayMonitorEventFilterVm, DisplayMonitorEventOpenOptions,
    DisplayMonitorEventOpenOptionsVm, DisplayMonitorEventVm, DisplayMonitorListRequestVm,
    DisplayMonitorOpenOptionsVm, WindowDescriptor, WindowDescriptorVm, WindowDropFilePayload,
    WindowDropFilePayloadVm, WindowDropHoverLeavePayload, WindowDropHoverLeavePayloadVm,
    WindowDropHoverPayload, WindowDropHoverPayloadVm, WindowDropTextPayload,
    WindowDropTextPayloadVm, WindowEvent, WindowEventOpenOptionsVm, WindowEventVm, WindowIconImage,
    WindowIconImageVm, WindowIconSet, WindowIconSetVm, WindowModeOptions, WindowModeOptionsVm,
    WindowModePayload, WindowModePayloadVm, WindowOptions, WindowOptionsVm, host as host_display,
};
use crate::platform::fs::{
    OsPath, OsPathBytesVm, OsPathUtf16Vm, OsPathVm, PathBytes, PathBytesAbi, PathBytesVm,
    PathUtf16, PathUtf16Abi, PathUtf16Vm,
};
use crate::platform::{NativeArray, VmArray, VmSlice, resource};
use crate::runtime::{BindingCallContext, NativeSlice, NativeStringRef};

/// Invoke one host call that writes through an output pointer.
fn call_out<T>(call: impl FnOnce(*mut T) -> RuntimeResult<()>) -> RuntimeResult<T> {
    // allocate one uninitialized output slot
    let mut out = std::mem::MaybeUninit::<T>::uninit();

    // execute call and assume initialization on success
    call(out.as_mut_ptr())?;
    Ok(unsafe { out.assume_init() })
}

/// Convert one VM string handle into one runtime-owned native string.
fn string_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<NativeStringRef> {
    let value = context
        .string_ref(value)
        .map_err(|error| RuntimeError::from(error).boxed())?;
    Ok(binding.store_string(value.as_str()))
}

/// Convert one native string into one VM string handle.
fn string_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeStringRef,
) -> RuntimeResult<vm::StringHandle> {
    let value = unsafe { value.as_str()? };
    Ok(vm::StringHandle::new(context.intern_string(value)))
}

/// Convert one optional native string into one optional VM string handle.
fn optional_string_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: Option<NativeStringRef>,
) -> RuntimeResult<Option<vm::StringHandle>> {
    value.map(|value| string_to_vm(context, value)).transpose()
}

/// Convert one native path-byte payload to VM.
fn path_bytes_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    path: PathBytes,
) -> RuntimeResult<PathBytesVm> {
    let bytes = unsafe { path.0.as_slice()? };
    let array = VmArray::from_bytes(context, bytes);
    Ok(PathBytesAbi::<VmAbi>(array))
}

/// Convert one native path-utf16 payload to VM.
fn path_utf16_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    path: PathUtf16,
) -> RuntimeResult<PathUtf16Vm> {
    let units = unsafe { path.0.as_slice()? };
    let array = VmArray::from_values(context, units)?;
    Ok(PathUtf16Abi::<VmAbi>(array))
}

/// Convert one native path payload to VM.
fn path_to_vm(context: &mut vm::ExternalCallContext<'_>, path: OsPath) -> RuntimeResult<OsPathVm> {
    match path {
        OsPath::OsPathBytes(path_bytes) => {
            let bytes = path_bytes_to_vm(context, path_bytes.bytes)?;
            Ok(OsPathVm::OsPathBytes(OsPathBytesVm {
                kind: vm::StringHandle::new(context.intern_string("bytes")),
                bytes,
            }))
        }
        OsPath::OsPathUtf16(path_utf16) => {
            let utf16 = path_utf16_to_vm(context, path_utf16.utf16)?;
            Ok(OsPathVm::OsPathUtf16(OsPathUtf16Vm {
                kind: vm::StringHandle::new(context.intern_string("utf16")),
                utf16,
            }))
        }
    }
}

/// Convert one native window-mode payload to VM.
fn window_mode_options_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    mode: WindowModeOptions,
) -> RuntimeResult<WindowModeOptionsVm> {
    let mode = match mode {
        WindowModeOptions::WindowBorderlessModeOptions(mode) => {
            WindowModeOptionsVm::WindowBorderlessModeOptions(
                crate::platform::display::WindowBorderlessModeOptionsVm {
                    kind: string_to_vm(context, mode.kind)?,
                    display: mode.display,
                },
            )
        }
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(mode) => {
            WindowModeOptionsVm::WindowExclusiveFullscreenModeOptions(
                crate::platform::display::WindowExclusiveFullscreenModeOptionsVm {
                    kind: string_to_vm(context, mode.kind)?,
                    display: mode.display,
                    display_mode: mode.display_mode,
                },
            )
        }
        WindowModeOptions::WindowWindowedModeOptions(mode) => {
            WindowModeOptionsVm::WindowWindowedModeOptions(
                crate::platform::display::WindowWindowedModeOptionsVm {
                    kind: string_to_vm(context, mode.kind)?,
                },
            )
        }
    };

    Ok(mode)
}

/// Convert one VM window-mode payload to native.
fn window_mode_options_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    mode: WindowModeOptionsVm,
) -> RuntimeResult<WindowModeOptions> {
    let mode = match mode {
        WindowModeOptionsVm::WindowBorderlessModeOptions(mode) => {
            WindowModeOptions::WindowBorderlessModeOptions(
                crate::platform::display::WindowBorderlessModeOptions {
                    kind: string_from_vm(binding, context, mode.kind)?,
                    display: mode.display,
                },
            )
        }
        WindowModeOptionsVm::WindowExclusiveFullscreenModeOptions(mode) => {
            WindowModeOptions::WindowExclusiveFullscreenModeOptions(
                crate::platform::display::WindowExclusiveFullscreenModeOptions {
                    kind: string_from_vm(binding, context, mode.kind)?,
                    display: mode.display,
                    display_mode: mode.display_mode,
                },
            )
        }
        WindowModeOptionsVm::WindowWindowedModeOptions(mode) => {
            WindowModeOptions::WindowWindowedModeOptions(
                crate::platform::display::WindowWindowedModeOptions {
                    kind: string_from_vm(binding, context, mode.kind)?,
                },
            )
        }
    };

    Ok(mode)
}

/// Convert one native display descriptor into its VM representation.
fn display_descriptor_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: DisplayDescriptor,
) -> RuntimeResult<DisplayDescriptorVm> {
    let id = unsafe { value.id.as_str()? };
    let name = unsafe { value.name.as_str()? };

    Ok(DisplayDescriptorVm {
        backend: value.backend,
        id: vm::StringHandle::new(context.intern_string(id)),
        name: vm::StringHandle::new(context.intern_string(name)),
        primary: value.primary,
        x: value.x,
        y: value.y,
        width_px: value.width_px,
        height_px: value.height_px,
        work_area_x: value.work_area_x,
        work_area_y: value.work_area_y,
        work_area_width_px: value.work_area_width_px,
        work_area_height_px: value.work_area_height_px,
        width_mm: value.width_mm,
        height_mm: value.height_mm,
        scale_factor_milli: value.scale_factor_milli,
        orientation: value.orientation,
        builtin_panel: value.builtin_panel,
        variable_refresh_support: value.variable_refresh_support,
        hdr_support: value.hdr_support,
    })
}

/// Convert one native display backend descriptor into its VM representation.
fn display_backend_descriptor_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: DisplayBackendDescriptor,
) -> RuntimeResult<DisplayBackendDescriptorVm> {
    let name = unsafe { value.name.as_str()? };

    Ok(DisplayBackendDescriptorVm {
        backend: value.backend,
        name: vm::StringHandle::new(context.intern_string(name)),
        available: value.available,
        priority: value.priority,
        capability_flags: value.capability_flags,
    })
}

/// Convert one native display event into its VM representation.
fn display_event_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: DisplayMonitorEvent,
) -> RuntimeResult<DisplayMonitorEventVm> {
    match value {
        DisplayMonitorEvent::DisplayAddedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            let metadata = display_event_metadata_to_vm(context, event.metadata)?;
            let descriptor = display_descriptor_to_vm(context, event.payload.descriptor)?;
            Ok(DisplayMonitorEventVm::DisplayAddedEvent(
                crate::platform::display::DisplayAddedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata,
                    payload: crate::platform::display::DisplayAddedPayloadVm { descriptor },
                },
            ))
        }
        DisplayMonitorEvent::DisplayDescriptorChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            let metadata = display_event_metadata_to_vm(context, event.metadata)?;
            let previous = event
                .payload
                .previous
                .map(|value| display_descriptor_to_vm(context, value))
                .transpose()?;
            let current = display_descriptor_to_vm(context, event.payload.current)?;
            Ok(DisplayMonitorEventVm::DisplayDescriptorChangedEvent(
                crate::platform::display::DisplayDescriptorChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata,
                    payload: crate::platform::display::DisplayDescriptorChangedPayloadVm {
                        previous,
                        current,
                        changed_mask: event.payload.changed_mask,
                    },
                },
            ))
        }
        DisplayMonitorEvent::DisplayModeChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            let metadata = display_event_metadata_to_vm(context, event.metadata)?;
            Ok(DisplayMonitorEventVm::DisplayModeChangedEvent(
                crate::platform::display::DisplayModeChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata,
                    payload: event.payload,
                },
            ))
        }
        DisplayMonitorEvent::DisplayPrimaryChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            let metadata = display_event_metadata_to_vm(context, event.metadata)?;
            Ok(DisplayMonitorEventVm::DisplayPrimaryChangedEvent(
                crate::platform::display::DisplayPrimaryChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata,
                    payload: crate::platform::display::DisplayPrimaryPayloadVm {
                        previous_id: optional_string_to_vm(context, event.payload.previous_id)?,
                        current_id: optional_string_to_vm(context, event.payload.current_id)?,
                    },
                },
            ))
        }
        DisplayMonitorEvent::DisplayRemovedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            let metadata = display_event_metadata_to_vm(context, event.metadata)?;
            let payload_id = string_to_vm(context, event.payload.id)?;
            let descriptor = event
                .payload
                .descriptor
                .map(|value| display_descriptor_to_vm(context, value))
                .transpose()?;
            Ok(DisplayMonitorEventVm::DisplayRemovedEvent(
                crate::platform::display::DisplayRemovedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata,
                    payload: crate::platform::display::DisplayRemovedPayloadVm {
                        id: payload_id,
                        descriptor,
                    },
                },
            ))
        }
    }
}

/// Convert one native display event metadata payload into its VM representation.
fn display_event_metadata_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: crate::platform::display::DisplayMonitorEventMetadata,
) -> RuntimeResult<crate::platform::display::DisplayMonitorEventMetadataVm> {
    let display_id = if let Some(value) = value.display_id {
        let value = unsafe { value.as_str()? };
        Some(vm::StringHandle::new(context.intern_string(value)))
    } else {
        None
    };

    Ok(crate::platform::display::DisplayMonitorEventMetadataVm {
        backend: value.backend,
        display_id,
        timestamp_ns: value.timestamp_ns,
        sequence: value.sequence,
        dropped_count: value.dropped_count,
    })
}

/// Convert one native window descriptor into its VM representation.
fn window_descriptor_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: WindowDescriptor,
) -> RuntimeResult<WindowDescriptorVm> {
    let id = unsafe { value.id.as_str()? };
    let title = unsafe { value.title.as_str()? };

    Ok(WindowDescriptorVm {
        backend: value.backend,
        id: vm::StringHandle::new(context.intern_string(id)),
        title: vm::StringHandle::new(context.intern_string(title)),
        mode: window_mode_options_to_vm(context, value.mode)?,
        display: value.display,
        resizable: value.resizable,
        decorated: value.decorated,
        chrome: value.chrome,
        taskbar_visible: value.taskbar_visible,
        transparent: value.transparent,
        opacity: value.opacity,
        always_on_top: value.always_on_top,
        parent: value.parent,
        transient_for: value.transient_for,
        modal: value.modal,
        mouse_passthrough: value.mouse_passthrough,
        aspect_ratio: value.aspect_ratio,
    })
}

/// Convert one native display-descriptor slice into one VM slice.
fn descriptor_slice_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeSlice<DisplayDescriptor>,
) -> RuntimeResult<VmSlice<DisplayDescriptorVm>> {
    let value = unsafe { value.as_slice()? };
    let mut vm_values = Vec::with_capacity(value.len());

    for descriptor in value {
        vm_values.push(display_descriptor_to_vm(context, *descriptor)?);
    }

    VmSlice::from_values(context, &vm_values)
}

/// Convert one native backend descriptor slice into one VM slice.
fn backend_descriptor_slice_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeSlice<DisplayBackendDescriptor>,
) -> RuntimeResult<VmSlice<DisplayBackendDescriptorVm>> {
    let value = unsafe { value.as_slice()? };
    let mut vm_values = Vec::with_capacity(value.len());

    for descriptor in value {
        vm_values.push(display_backend_descriptor_to_vm(context, *descriptor)?);
    }

    VmSlice::from_values(context, &vm_values)
}

/// List display backends that are available for the active target.
pub(crate) fn destack_display_backend_list(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<DisplayBackendDescriptorVm>> {
    let values =
        call_out(|out| unsafe { host_display::destack_display_backend_list(binding, out) })?;
    backend_descriptor_slice_to_vm(context, values)
}

/// Convert one native display-mode slice into one VM slice.
fn mode_slice_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeSlice<DisplayMode>,
) -> RuntimeResult<VmSlice<DisplayModeVm>> {
    let value = unsafe { value.as_slice()? };
    VmSlice::from_values(context, value)
}

/// Convert one native monitor-event array into one VM array.
fn display_event_array_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeArray<DisplayMonitorEvent>,
) -> RuntimeResult<VmArray<DisplayMonitorEventVm>> {
    let value = unsafe { value.as_slice()? };
    let mut vm_values = Vec::with_capacity(value.len());

    for event in value {
        vm_values.push(display_event_to_vm(context, *event)?);
    }

    VmArray::from_values(context, &vm_values)
}

/// Convert one native window-event array into one VM array.
fn window_event_array_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeArray<crate::platform::display::WindowEvent>,
) -> RuntimeResult<VmArray<crate::platform::display::WindowEventVm>> {
    let value = unsafe { value.as_slice()? };
    let mut vm_values = Vec::with_capacity(value.len());

    for event in value {
        vm_values.push(window_event_to_vm(context, *event)?);
    }

    VmArray::from_values(context, &vm_values)
}

/// Convert one native display gamma-ramp payload to VM.
fn display_gamma_ramp_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    ramp: DisplayGammaRamp,
) -> RuntimeResult<DisplayGammaRampVm> {
    let red = unsafe { ramp.red.as_slice()? };
    let green = unsafe { ramp.green.as_slice()? };
    let blue = unsafe { ramp.blue.as_slice()? };

    Ok(DisplayGammaRampVm {
        red: VmSlice::from_values(context, red)?,
        green: VmSlice::from_values(context, green)?,
        blue: VmSlice::from_values(context, blue)?,
    })
}

/// Convert one VM display gamma-ramp payload to native.
fn display_gamma_ramp_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    ramp: DisplayGammaRampVm,
) -> RuntimeResult<DisplayGammaRamp> {
    let red = binding.store_slice(ramp.red.read_values(context)?);
    let green = binding.store_slice(ramp.green.read_values(context)?);
    let blue = binding.store_slice(ramp.blue.read_values(context)?);

    Ok(DisplayGammaRamp { red, green, blue })
}

/// Convert one VM monitor-event filter payload to native.
fn monitor_event_filter_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    filter: DisplayMonitorEventFilterVm,
) -> RuntimeResult<DisplayMonitorEventFilter> {
    let display_id = filter
        .display_id
        .map(|value| string_from_vm(binding, context, value))
        .transpose()?;

    Ok(DisplayMonitorEventFilter {
        display_id,
        kind_mask: filter.kind_mask,
    })
}

/// Convert one VM monitor-event open-options payload to native.
fn monitor_event_open_options_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    options: DisplayMonitorEventOpenOptionsVm,
) -> RuntimeResult<DisplayMonitorEventOpenOptions> {
    let filter = options
        .filter
        .map(|filter| monitor_event_filter_from_vm(binding, context, filter))
        .transpose()?;

    Ok(DisplayMonitorEventOpenOptions {
        backend: options.backend,
        backend_policy: options.backend_policy,
        queue: options.queue,
        filter,
    })
}

/// Convert one VM icon-image payload to native.
fn window_icon_image_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    image: WindowIconImageVm,
) -> RuntimeResult<WindowIconImage> {
    let pixels = image.pixels.read_bytes(context)?;

    Ok(WindowIconImage {
        width: image.width,
        height: image.height,
        pixel_format: image.pixel_format,
        pixels: binding.store_slice(pixels),
    })
}

/// Convert one VM icon-set payload to native.
fn window_icon_set_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    icon_set: WindowIconSetVm,
) -> RuntimeResult<WindowIconSet> {
    let images = icon_set.images.read_values(context)?;
    let mut native_images = Vec::with_capacity(images.len());

    for image in images {
        native_images.push(window_icon_image_from_vm(binding, context, image)?);
    }

    Ok(WindowIconSet {
        images: binding.store_slice(native_images),
    })
}

/// Convert one native window-mode-changed payload to VM.
fn window_mode_payload_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    payload: WindowModePayload,
) -> RuntimeResult<WindowModePayloadVm> {
    Ok(WindowModePayloadVm {
        previous_mode: window_mode_options_to_vm(context, payload.previous_mode)?,
        current_mode: window_mode_options_to_vm(context, payload.current_mode)?,
    })
}

/// Convert one native file-drop payload to VM.
fn window_drop_file_payload_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    payload: WindowDropFilePayload,
) -> RuntimeResult<WindowDropFilePayloadVm> {
    let path = payload
        .path
        .map(|path| path_to_vm(context, path))
        .transpose()?;
    Ok(WindowDropFilePayloadVm {
        path,
        position: payload.position,
    })
}

/// Convert one native file-hover payload to VM.
fn window_drop_hover_payload_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    payload: WindowDropHoverPayload,
) -> RuntimeResult<WindowDropHoverPayloadVm> {
    let path = payload
        .path
        .map(|path| path_to_vm(context, path))
        .transpose()?;
    Ok(WindowDropHoverPayloadVm {
        path,
        position: payload.position,
    })
}

/// Convert one native file-hover-leave payload to VM.
fn window_drop_hover_leave_payload_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    payload: WindowDropHoverLeavePayload,
) -> RuntimeResult<WindowDropHoverLeavePayloadVm> {
    let previous_path = payload
        .previous_path
        .map(|path| path_to_vm(context, path))
        .transpose()?;
    Ok(WindowDropHoverLeavePayloadVm {
        previous_path,
        position: payload.position,
    })
}

/// Convert one native text-drop payload to VM.
fn window_drop_text_payload_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    payload: WindowDropTextPayload,
) -> RuntimeResult<WindowDropTextPayloadVm> {
    Ok(WindowDropTextPayloadVm {
        text: string_to_vm(context, payload.text)?,
        position: payload.position,
    })
}

/// Convert one native window event into its VM representation.
fn window_event_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: WindowEvent,
) -> RuntimeResult<WindowEventVm> {
    match value {
        WindowEvent::WindowAspectRatioChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowAspectRatioChangedEvent(
                crate::platform::display::WindowAspectRatioChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowChromeChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowChromeChangedEvent(
                crate::platform::display::WindowChromeChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowCloseRequestedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowCloseRequestedEvent(
                crate::platform::display::WindowCloseRequestedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                },
            ))
        }
        WindowEvent::WindowCreatedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowCreatedEvent(
                crate::platform::display::WindowCreatedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                },
            ))
        }
        WindowEvent::WindowDestroyedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowDestroyedEvent(
                crate::platform::display::WindowDestroyedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                },
            ))
        }
        WindowEvent::WindowDisplayChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowDisplayChangedEvent(
                crate::platform::display::WindowDisplayChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowDropCancelledEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowDropCancelledEvent(
                crate::platform::display::WindowDropCancelledEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                },
            ))
        }
        WindowEvent::WindowDropCompletedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowDropCompletedEvent(
                crate::platform::display::WindowDropCompletedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                },
            ))
        }
        WindowEvent::WindowDropStartedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowDropStartedEvent(
                crate::platform::display::WindowDropStartedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                },
            ))
        }
        WindowEvent::WindowFileDroppedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowFileDroppedEvent(
                crate::platform::display::WindowFileDroppedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: window_drop_file_payload_to_vm(context, event.payload)?,
                },
            ))
        }
        WindowEvent::WindowFileHoverLeftEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowFileHoverLeftEvent(
                crate::platform::display::WindowFileHoverLeftEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: window_drop_hover_leave_payload_to_vm(context, event.payload)?,
                },
            ))
        }
        WindowEvent::WindowFileHoveredEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowFileHoveredEvent(
                crate::platform::display::WindowFileHoveredEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: window_drop_hover_payload_to_vm(context, event.payload)?,
                },
            ))
        }
        WindowEvent::WindowFocusChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowFocusChangedEvent(
                crate::platform::display::WindowFocusChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowModalChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowModalChangedEvent(
                crate::platform::display::WindowModalChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowModeChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowModeChangedEvent(
                crate::platform::display::WindowModeChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: window_mode_payload_to_vm(context, event.payload)?,
                },
            ))
        }
        WindowEvent::WindowMousePassthroughChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowMousePassthroughChangedEvent(
                crate::platform::display::WindowMousePassthroughChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowOcclusionChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowOcclusionChangedEvent(
                crate::platform::display::WindowOcclusionChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowOpacityChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowOpacityChangedEvent(
                crate::platform::display::WindowOpacityChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowParentChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowParentChangedEvent(
                crate::platform::display::WindowParentChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowPositionChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowPositionChangedEvent(
                crate::platform::display::WindowPositionChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowRefreshRequestedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowRefreshRequestedEvent(
                crate::platform::display::WindowRefreshRequestedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                },
            ))
        }
        WindowEvent::WindowSafeAreaChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowSafeAreaChangedEvent(
                crate::platform::display::WindowSafeAreaChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowScaleFactorChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowScaleFactorChangedEvent(
                crate::platform::display::WindowScaleFactorChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowSizeChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowSizeChangedEvent(
                crate::platform::display::WindowSizeChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowTaskbarVisibilityChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowTaskbarVisibilityChangedEvent(
                crate::platform::display::WindowTaskbarVisibilityChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowTextDroppedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowTextDroppedEvent(
                crate::platform::display::WindowTextDroppedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: window_drop_text_payload_to_vm(context, event.payload)?,
                },
            ))
        }
        WindowEvent::WindowThemeChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowThemeChangedEvent(
                crate::platform::display::WindowThemeChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowTransientChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowTransientChangedEvent(
                crate::platform::display::WindowTransientChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowVisibilityChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowVisibilityChangedEvent(
                crate::platform::display::WindowVisibilityChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
    }
}

/// Convert one VM window options payload into one native payload.
fn window_options_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    options: WindowOptionsVm,
) -> RuntimeResult<WindowOptions> {
    let title = {
        let title = context
            .string_ref(options.title)
            .map_err(|error| RuntimeError::from(error).boxed())?;
        binding.store_string(title.as_str())
    };

    Ok(WindowOptions {
        backend: options.backend,
        backend_policy: options.backend_policy,
        title,
        size_logical: options.size_logical,
        position: options.position,
        constraints: options.constraints,
        display: options.display,
        mode: window_mode_options_from_vm(binding, context, options.mode)?,
        visibility: options.visibility,
        resizable: options.resizable,
        decorated: options.decorated,
        transparent: options.transparent,
        chrome: options.chrome,
        taskbar_visible: options.taskbar_visible,
        opacity: options.opacity,
        focus_on_show: options.focus_on_show,
        always_on_top: options.always_on_top,
        parent: options.parent,
        transient_for: options.transient_for,
        modal: options.modal,
        mouse_passthrough: options.mouse_passthrough,
        aspect_ratio: options.aspect_ratio,
    })
}

/// Close one display endpoint.
pub(crate) fn destack_display_monitor_close(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_monitor_close(binding, handle) }
}

/// Resolve one requested mode to the closest supported mode.
pub(crate) fn destack_display_monitor_closest_mode(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
    requested: DisplayModeVm,
) -> RuntimeResult<DisplayModeVm> {
    call_out(|out| unsafe {
        host_display::destack_display_monitor_closest_mode(binding, out, handle, requested)
    })
}

/// Read one point-in-time current mode for one display.
pub(crate) fn destack_display_monitor_current_mode(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<DisplayModeVm> {
    call_out(|out| unsafe {
        host_display::destack_display_monitor_current_mode(binding, out, handle)
    })
}

/// Read descriptor metadata for one display.
pub(crate) fn destack_display_monitor_descriptor(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<DisplayDescriptorVm> {
    let descriptor = call_out(|out| unsafe {
        host_display::destack_display_monitor_descriptor(binding, out, handle)
    })?;

    display_descriptor_to_vm(context, descriptor)
}

/// Read one point-in-time desktop mode for one display.
pub(crate) fn destack_display_monitor_desktop_mode(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<DisplayModeVm> {
    call_out(|out| unsafe {
        host_display::destack_display_monitor_desktop_mode(binding, out, handle)
    })
}

/// Close one monitor-event stream.
pub(crate) fn destack_display_monitor_event_close(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_monitor_event_close(binding, handle) }
}

/// Open one monitor-event stream.
pub(crate) fn destack_display_monitor_event_open(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    options: DisplayMonitorEventOpenOptionsVm,
) -> RuntimeResult<resource::DisplayEventHandle> {
    let options = monitor_event_open_options_from_vm(binding, context, options)?;
    call_out(|out| unsafe {
        host_display::destack_display_monitor_event_open(binding, out, options)
    })
}

/// Wait for one monitor event.
pub(crate) fn destack_display_monitor_event_read(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayEventHandle,
    timeoutns: u64,
) -> RuntimeResult<DisplayMonitorEventVm> {
    let event = call_out(|out| unsafe {
        host_display::destack_display_monitor_event_read(binding, out, handle, timeoutns)
    })?;

    display_event_to_vm(context, event)
}

/// Wait for one batch of monitor events.
pub(crate) fn destack_display_monitor_event_read_batch(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<VmArray<DisplayMonitorEventVm>> {
    let events = call_out(|out| unsafe {
        host_display::destack_display_monitor_event_read_batch(
            binding, out, handle, maxevents, timeoutns,
        )
    })?;

    display_event_array_to_vm(context, events)
}

/// Poll one monitor event without blocking.
pub(crate) fn destack_display_monitor_event_try_read(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<DisplayMonitorEventVm> {
    let event = call_out(|out| unsafe {
        host_display::destack_display_monitor_event_try_read(binding, out, handle)
    })?;

    display_event_to_vm(context, event)
}

/// Poll one batch of monitor events without blocking.
pub(crate) fn destack_display_monitor_event_try_read_batch(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayEventHandle,
    maxevents: u32,
) -> RuntimeResult<VmArray<DisplayMonitorEventVm>> {
    let events = call_out(|out| unsafe {
        host_display::destack_display_monitor_event_try_read_batch(binding, out, handle, maxevents)
    })?;

    display_event_array_to_vm(context, events)
}

/// List available displays.
pub(crate) fn destack_display_monitor_list(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    request: DisplayMonitorListRequestVm,
) -> RuntimeResult<VmSlice<DisplayDescriptorVm>> {
    let descriptors = call_out(|out| unsafe {
        host_display::destack_display_monitor_list(binding, out, request)
    })?;
    descriptor_slice_to_vm(context, descriptors)
}

/// Read available display modes.
pub(crate) fn destack_display_monitor_modes(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<VmSlice<DisplayModeVm>> {
    let modes = call_out(|out| unsafe {
        host_display::destack_display_monitor_modes(binding, out, handle)
    })?;
    mode_slice_to_vm(context, modes)
}

/// Open one display endpoint.
pub(crate) fn destack_display_monitor_open(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
    options: DisplayMonitorOpenOptionsVm,
) -> RuntimeResult<resource::DisplayHandle> {
    let id = string_from_vm(binding, context, id)?;
    call_out(|out| unsafe { host_display::destack_display_monitor_open(binding, out, id, options) })
}

/// Read one primary display handle when available.
pub(crate) fn destack_display_monitor_primary(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    request: DisplayMonitorListRequestVm,
) -> RuntimeResult<Option<resource::DisplayHandle>> {
    call_out(|out| unsafe { host_display::destack_display_monitor_primary(binding, out, request) })
}

/// Apply one display mode.
pub(crate) fn destack_display_monitor_set_mode(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
    mode: DisplayModeVm,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_monitor_set_mode(binding, handle, mode) }
}

/// Close one window.
pub(crate) fn destack_display_window_close(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_close(binding, window) }
}

/// Read descriptor metadata for one window.
pub(crate) fn destack_display_window_descriptor(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<WindowDescriptorVm> {
    let descriptor = call_out(|out| unsafe {
        host_display::destack_display_window_descriptor(binding, out, window)
    })?;

    window_descriptor_to_vm(context, descriptor)
}

/// Close one window-event stream.
pub(crate) fn destack_display_window_event_close(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_event_close(binding, handle) }
}

/// Open one window-event stream.
pub(crate) fn destack_display_window_event_open(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    options: WindowEventOpenOptionsVm,
) -> RuntimeResult<resource::WindowEventHandle> {
    call_out(|out| unsafe {
        host_display::destack_display_window_event_open(binding, out, options)
    })
}

/// Wait for one window event.
pub(crate) fn destack_display_window_event_read(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::WindowEventHandle,
    timeoutns: u64,
) -> RuntimeResult<WindowEventVm> {
    let event = call_out(|out| unsafe {
        host_display::destack_display_window_event_read(binding, out, handle, timeoutns)
    })?;

    window_event_to_vm(context, event)
}

/// Wait for one batch of window events.
pub(crate) fn destack_display_window_event_read_batch(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<VmArray<WindowEventVm>> {
    let events = call_out(|out| unsafe {
        host_display::destack_display_window_event_read_batch(
            binding, out, handle, maxevents, timeoutns,
        )
    })?;

    window_event_array_to_vm(context, events)
}

/// Poll one window event without blocking.
pub(crate) fn destack_display_window_event_try_read(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<WindowEventVm> {
    let event = call_out(|out| unsafe {
        host_display::destack_display_window_event_try_read(binding, out, handle)
    })?;

    window_event_to_vm(context, event)
}

/// Poll one batch of window events without blocking.
pub(crate) fn destack_display_window_event_try_read_batch(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
) -> RuntimeResult<VmArray<WindowEventVm>> {
    let events = call_out(|out| unsafe {
        host_display::destack_display_window_event_try_read_batch(binding, out, handle, maxevents)
    })?;

    window_event_array_to_vm(context, events)
}

/// Open one window.
pub(crate) fn destack_display_window_open(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    options: WindowOptionsVm,
) -> RuntimeResult<resource::WindowHandle> {
    let options = window_options_from_vm(binding, context, options)?;
    call_out(|out| unsafe { host_display::destack_display_window_open(binding, out, options) })
}

/// Request user attention for one window.
pub(crate) fn destack_display_window_request_attention(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    level: crate::platform::display::WindowAttentionLevel,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_request_attention(binding, window, level) }
}

/// Request one redraw for one window.
pub(crate) fn destack_display_window_request_refresh(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_request_refresh(binding, window) }
}

/// Set always-on-top state.
pub(crate) fn destack_display_window_set_always_on_top(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    alwaysontop: bool,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_always_on_top(binding, window, alwaysontop) }
}

/// Set cursor icon for one window.
pub(crate) fn destack_display_window_set_cursor_icon(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    icon: crate::platform::display::WindowCursorIcon,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_cursor_icon(binding, window, icon) }
}

/// Set cursor interaction mode for one window.
pub(crate) fn destack_display_window_set_cursor_mode(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    mode: crate::platform::display::WindowCursorMode,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_cursor_mode(binding, window, mode) }
}

/// Set cursor position for one window.
pub(crate) fn destack_display_window_set_cursor_position(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    position: crate::platform::display::WindowPositionVm,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_cursor_position(binding, window, position) }
}

/// Set cursor visibility for one window.
pub(crate) fn destack_display_window_set_cursor_visible(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_cursor_visible(binding, window, visible) }
}

/// Set window decoration state.
pub(crate) fn destack_display_window_set_decorated(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_decorated(binding, window, decorated) }
}

/// Set one window mode.
pub(crate) fn destack_display_window_set_mode(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    mode: crate::platform::display::WindowModeOptionsVm,
) -> RuntimeResult<()> {
    let mode = window_mode_options_from_vm(binding, context, mode)?;
    unsafe { host_display::destack_display_window_set_mode(binding, window, mode) }
}

/// Set one window position.
pub(crate) fn destack_display_window_set_position(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    position: crate::platform::display::WindowPositionVm,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_position(binding, window, position) }
}

/// Set window resizable state.
pub(crate) fn destack_display_window_set_resizable(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_resizable(binding, window, resizable) }
}

/// Set one logical window size.
pub(crate) fn destack_display_window_set_size_logical(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    size: crate::platform::display::WindowLogicalSizeVm,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_size_logical(binding, window, size) }
}

/// Set one logical size-constraint payload.
pub(crate) fn destack_display_window_set_size_constraints(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    constraints: Option<crate::platform::display::WindowSizeConstraintsVm>,
) -> RuntimeResult<()> {
    unsafe {
        host_display::destack_display_window_set_size_constraints(binding, window, constraints)
    }
}

/// Set one physical window size.
pub(crate) fn destack_display_window_set_size_physical(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    size: crate::platform::display::WindowPhysicalSizeVm,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_size_physical(binding, window, size) }
}

/// Set one window title string.
pub(crate) fn destack_display_window_set_title(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    title: vm::StringHandle,
) -> RuntimeResult<()> {
    let title = string_from_vm(binding, context, title)?;
    unsafe { host_display::destack_display_window_set_title(binding, window, title) }
}

/// Set one window visibility state.
pub(crate) fn destack_display_window_set_visibility(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    visibility: crate::platform::display::WindowVisibility,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_visibility(binding, window, visibility) }
}

/// Read one window state snapshot.
pub(crate) fn destack_display_window_state(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<crate::platform::display::WindowStateVm> {
    call_out(|out| unsafe { host_display::destack_display_window_state(binding, out, window) })
}

/// Read display color state.
pub(crate) fn destack_display_monitor_color_state(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<crate::platform::display::DisplayColorState> {
    call_out(|out| unsafe {
        host_display::destack_display_monitor_color_state(binding, out, handle)
    })
}

/// Read display gamma ramp.
pub(crate) fn destack_display_monitor_gamma_ramp(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<DisplayGammaRampVm> {
    let ramp = call_out(|out| unsafe {
        host_display::destack_display_monitor_gamma_ramp(binding, out, handle)
    })?;
    display_gamma_ramp_to_vm(context, ramp)
}

/// Read display HDR mode.
pub(crate) fn destack_display_monitor_hdr_mode(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<crate::platform::display::DisplayHdrMode> {
    call_out(|out| unsafe { host_display::destack_display_monitor_hdr_mode(binding, out, handle) })
}

/// Set display gamma ramp.
pub(crate) fn destack_display_monitor_set_gamma_ramp(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
    ramp: DisplayGammaRampVm,
) -> RuntimeResult<()> {
    let ramp = display_gamma_ramp_from_vm(binding, context, ramp)?;
    unsafe { host_display::destack_display_monitor_set_gamma_ramp(binding, handle, ramp) }
}

/// Set display HDR mode.
pub(crate) fn destack_display_monitor_set_hdr_mode(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
    mode: crate::platform::display::DisplayHdrMode,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_monitor_set_hdr_mode(binding, handle, mode) }
}

/// Begin one native move-drag interaction.
pub(crate) fn destack_display_window_begin_move_drag(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_begin_move_drag(binding, window) }
}

/// Begin one native resize-drag interaction.
pub(crate) fn destack_display_window_begin_resize_drag(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    edge: crate::platform::display::WindowResizeEdge,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_begin_resize_drag(binding, window, edge) }
}

/// Focus one window.
pub(crate) fn destack_display_window_focus(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_focus(binding, window) }
}

/// Maximize one window.
pub(crate) fn destack_display_window_maximize(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_maximize(binding, window) }
}

/// Minimize one window.
pub(crate) fn destack_display_window_minimize(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_minimize(binding, window) }
}

/// Read one window opacity.
pub(crate) fn destack_display_window_opacity(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<f64> {
    call_out(|out| unsafe { host_display::destack_display_window_opacity(binding, out, window) })
}

/// Raise one window.
pub(crate) fn destack_display_window_raise(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_raise(binding, window) }
}

/// Restore one window.
pub(crate) fn destack_display_window_restore(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_restore(binding, window) }
}

/// Set one window aspect-ratio lock.
pub(crate) fn destack_display_window_set_aspect_ratio(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    aspectratio: Option<crate::platform::display::WindowAspectRatio>,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_aspect_ratio(binding, window, aspectratio) }
}

/// Set one window chrome style.
pub(crate) fn destack_display_window_set_chrome(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    chrome: crate::platform::display::WindowChromeKind,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_chrome(binding, window, chrome) }
}

/// Set one window icon set.
pub(crate) fn destack_display_window_set_icons(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    icons: Option<WindowIconSetVm>,
) -> RuntimeResult<()> {
    let icons = icons
        .map(|icons| window_icon_set_from_vm(binding, context, icons))
        .transpose()?;
    unsafe { host_display::destack_display_window_set_icons(binding, window, icons) }
}

/// Set one window modal state.
pub(crate) fn destack_display_window_set_modal(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    modal: bool,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_modal(binding, window, modal) }
}

/// Set one window mouse-passthrough state.
pub(crate) fn destack_display_window_set_mouse_passthrough(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    passthrough: bool,
) -> RuntimeResult<()> {
    unsafe {
        host_display::destack_display_window_set_mouse_passthrough(binding, window, passthrough)
    }
}

/// Set one window opacity.
pub(crate) fn destack_display_window_set_opacity(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    opacity: f64,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_opacity(binding, window, opacity) }
}

/// Set one window parent relationship.
pub(crate) fn destack_display_window_set_parent(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    parent: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_parent(binding, window, parent) }
}

/// Set one window taskbar visibility state.
pub(crate) fn destack_display_window_set_taskbar_visible(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_taskbar_visible(binding, window, visible) }
}

/// Set one window transient-owner relationship.
pub(crate) fn destack_display_window_set_transient_for(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    transientfor: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_transient_for(binding, window, transientfor) }
}
