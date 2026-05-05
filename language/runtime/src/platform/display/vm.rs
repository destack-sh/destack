use destack_vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform;
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::core::{
    call_out, intern_string_to_vm as string_to_vm, map_native_array_to_vm, map_native_slice_to_vm,
    optional_intern_string_to_vm as optional_string_to_vm, store_string_from_vm as string_from_vm,
};
use crate::platform::display::{
    DisplayBackendCapabilityFlags, DisplayBackendDescriptor, DisplayBackendDescriptorVm,
    DisplayBeginFrameEvent, DisplayBeginFrameEventVm, DisplayBeginFrameOpenOptionsVm,
    DisplayDescriptor, DisplayDescriptorVm, DisplayDragBeginOptions, DisplayDragBeginOptionsVm,
    DisplayDragOperation, DisplayGammaRamp, DisplayGammaRampVm, DisplayModeVm, DisplayMonitorEvent,
    DisplayMonitorEventFilter, DisplayMonitorEventFilterVm, DisplayMonitorEventOpenOptions,
    DisplayMonitorEventOpenOptionsVm, DisplayMonitorEventVm, DisplayMonitorListRequestVm,
    DisplayMonitorOpenOptionsVm, WindowDescriptor, WindowDescriptorVm, WindowEvent,
    WindowEventOpenOptionsVm, WindowEventVm, WindowIconImage, WindowIconImageVm, WindowIconSet,
    WindowIconSetVm, WindowLogicalRectVm, WindowModeOptions, WindowModeOptionsVm,
    WindowModePayload, WindowModePayloadVm, WindowOptions, WindowOptionsVm, WindowPhysicalSizeVm,
    WindowRenderState, native as host_display,
};
use crate::platform::fs::OsPathVm;
use crate::platform::{NativeArray, PlatformError, VmArray, VmSlice, resource};
use crate::runtime::BindingCallContext;

/// Encode one native binding payload into one VM binding payload.
fn vm_value_from_native<Native, Vm>(
    context: &mut destack_vm::BindingContext<'_>,
    value: Native,
) -> RuntimeResult<Vm>
where
    Native: platform::NativeAbiCodec,
    Vm: platform::VmAbiCodec<Value = Native::Value>,
{
    let value = unsafe { value.into_value()? };

    Vm::from_value(&mut context.write(), value)
}

/// Convert one native runtime string into one VM string handle.
fn native_string_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    value: NativeStringRef,
) -> RuntimeResult<destack_vm::StringHandle> {
    let value = unsafe { value.as_str()? };
    context
        .string_handle(value)
        .map_err(Box::<RuntimeError>::from)
}

/// Decode one VM binding payload into one native binding payload.
fn native_value_from_vm<Native, Vm>(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    value: Vm,
) -> RuntimeResult<Native>
where
    Native: platform::NativeAbiCodec<Value = Vm::Value>,
    Vm: platform::VmAbiCodec,
{
    let value = value.into_value(&context.read())?;

    Ok(Native::from_value(binding, value))
}

/// Begin one drag session.
pub(crate) fn destack_display_drag_begin(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    options: DisplayDragBeginOptionsVm,
) -> RuntimeResult<DisplayDragOperation> {
    let options: DisplayDragBeginOptions = native_value_from_vm(binding, context, options)?;

    call_out(|out| unsafe { host_display::destack_display_drag_begin(binding, out, options) })
}

/// Set one drag session operation.
pub(crate) fn destack_display_drag_session_set_operation(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    session: resource::DisplayDragSessionHandle,
    operation: DisplayDragOperation,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_drag_session_set_operation(binding, session, operation) }
}

/// Close one drag session.
pub(crate) fn destack_display_drag_session_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    session: resource::DisplayDragSessionHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_drag_session_close(binding, session) }
}

/// Read one drag session item as bytes.
pub(crate) fn destack_display_drag_session_read_bytes(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    session: resource::DisplayDragSessionHandle,
    itemindex: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let value = call_out(|out| unsafe {
        host_display::destack_display_drag_session_read_bytes(binding, out, session, itemindex)
    })?;

    VmSlice::from_bytes(&mut context.write(), unsafe { value.as_slice()? })
}

/// Read one drag session item as one path.
pub(crate) fn destack_display_drag_session_read_path(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    session: resource::DisplayDragSessionHandle,
    itemindex: u32,
) -> RuntimeResult<OsPathVm> {
    let value = call_out(|out| unsafe {
        host_display::destack_display_drag_session_read_path(binding, out, session, itemindex)
    })?;

    vm_value_from_native(context, value)
}

/// Read one drag session item as text.
pub(crate) fn destack_display_drag_session_read_text(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    session: resource::DisplayDragSessionHandle,
    itemindex: u32,
) -> RuntimeResult<destack_vm::StringHandle> {
    let value = call_out(|out| unsafe {
        host_display::destack_display_drag_session_read_text(binding, out, session, itemindex)
    })?;

    native_string_to_vm(context, value)
}

/// Convert one native window-mode payload to VM.
fn window_mode_options_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    mode: WindowModeOptions,
) -> RuntimeResult<WindowModeOptionsVm> {
    let mode = match mode {
        WindowModeOptions::WindowBorderlessModeOptions(mode) => {
            WindowModeOptionsVm::WindowBorderlessModeOptions(
                platform::display::WindowBorderlessModeOptionsVm {
                    kind: string_to_vm(context, mode.kind)?,
                    display: mode.display,
                },
            )
        }
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(mode) => {
            WindowModeOptionsVm::WindowExclusiveFullscreenModeOptions(
                platform::display::WindowExclusiveFullscreenModeOptionsVm {
                    kind: string_to_vm(context, mode.kind)?,
                    display: mode.display,
                    display_mode: mode.display_mode,
                },
            )
        }
        WindowModeOptions::WindowWindowedModeOptions(mode) => {
            WindowModeOptionsVm::WindowWindowedModeOptions(
                platform::display::WindowWindowedModeOptionsVm {
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
    context: &mut destack_vm::BindingContext<'_>,
    mode: WindowModeOptionsVm,
) -> RuntimeResult<WindowModeOptions> {
    let mode = match mode {
        WindowModeOptionsVm::WindowBorderlessModeOptions(mode) => {
            WindowModeOptions::WindowBorderlessModeOptions(
                platform::display::WindowBorderlessModeOptions {
                    kind: string_from_vm(binding, context, mode.kind)?,
                    display: mode.display,
                },
            )
        }
        WindowModeOptionsVm::WindowExclusiveFullscreenModeOptions(mode) => {
            WindowModeOptions::WindowExclusiveFullscreenModeOptions(
                platform::display::WindowExclusiveFullscreenModeOptions {
                    kind: string_from_vm(binding, context, mode.kind)?,
                    display: mode.display,
                    display_mode: mode.display_mode,
                },
            )
        }
        WindowModeOptionsVm::WindowWindowedModeOptions(mode) => {
            WindowModeOptions::WindowWindowedModeOptions(
                platform::display::WindowWindowedModeOptions {
                    kind: string_from_vm(binding, context, mode.kind)?,
                },
            )
        }
    };

    Ok(mode)
}

/// Convert one native display descriptor into its VM representation.
fn display_descriptor_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    value: DisplayDescriptor,
) -> RuntimeResult<DisplayDescriptorVm> {
    let id = unsafe { value.id.as_str()? };
    let name = unsafe { value.name.as_str()? };

    Ok(DisplayDescriptorVm {
        backend: value.backend,
        id: context
            .string_handle(id)
            .map_err(Box::<RuntimeError>::from)?,
        name: context
            .string_handle(name)
            .map_err(Box::<RuntimeError>::from)?,
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
        color_state: value.color_state,
    })
}

/// Convert one native display backend descriptor into its VM representation.
fn display_backend_descriptor_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    value: DisplayBackendDescriptor,
) -> RuntimeResult<DisplayBackendDescriptorVm> {
    let name = unsafe { value.name.as_str()? };

    Ok(DisplayBackendDescriptorVm {
        backend: value.backend,
        name: context
            .string_handle(name)
            .map_err(Box::<RuntimeError>::from)?,
        support: value.support,
        priority: value.priority,
        capability_flags: value.capability_flags,
    })
}

/// Convert one native display event into its VM representation.
fn display_event_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    value: DisplayMonitorEvent,
) -> RuntimeResult<DisplayMonitorEventVm> {
    match value {
        DisplayMonitorEvent::DisplayAddedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            let metadata = display_event_metadata_to_vm(context, event.metadata)?;
            let descriptor = display_descriptor_to_vm(context, event.payload.descriptor)?;
            Ok(DisplayMonitorEventVm::DisplayAddedEvent(
                platform::display::DisplayAddedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata,
                    payload: platform::display::DisplayAddedPayloadVm { descriptor },
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
                platform::display::DisplayDescriptorChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata,
                    payload: platform::display::DisplayDescriptorChangedPayloadVm {
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
                platform::display::DisplayModeChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata,
                    payload: event.payload,
                },
            ))
        }
        DisplayMonitorEvent::DisplayPrimaryChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            let metadata = display_event_metadata_to_vm(context, event.metadata)?;
            Ok(DisplayMonitorEventVm::DisplayPrimaryChangedEvent(
                platform::display::DisplayPrimaryChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata,
                    payload: platform::display::DisplayPrimaryPayloadVm {
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
                platform::display::DisplayRemovedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata,
                    payload: platform::display::DisplayRemovedPayloadVm {
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
    context: &mut destack_vm::BindingContext<'_>,
    value: platform::display::DisplayMonitorEventMetadata,
) -> RuntimeResult<platform::display::DisplayMonitorEventMetadataVm> {
    let display_id = if let Some(value) = value.display_id {
        let value = unsafe { value.as_str()? };
        Some(
            context
                .string_handle(value)
                .map_err(Box::<RuntimeError>::from)?,
        )
    } else {
        None
    };

    Ok(platform::display::DisplayMonitorEventMetadataVm {
        backend: value.backend,
        display_id,
        timestamp_ns: value.timestamp_ns,
        sequence: value.sequence,
        dropped_count: value.dropped_count,
    })
}

/// Convert one native window descriptor into its VM representation.
fn window_descriptor_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    value: WindowDescriptor,
) -> RuntimeResult<WindowDescriptorVm> {
    let id = unsafe { value.id.as_str()? };
    let title = unsafe { value.title.as_str()? };

    Ok(WindowDescriptorVm {
        backend: value.backend,
        id: context
            .string_handle(id)
            .map_err(Box::<RuntimeError>::from)?,
        title: context
            .string_handle(title)
            .map_err(Box::<RuntimeError>::from)?,
        role: value.role,
        mode: window_mode_options_to_vm(context, value.mode)?,
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
    context: &mut destack_vm::BindingContext<'_>,
    value: NativeSlice<DisplayDescriptor>,
) -> RuntimeResult<VmSlice<DisplayDescriptorVm>> {
    map_native_slice_to_vm(context, value, |context, descriptor| {
        display_descriptor_to_vm(context, *descriptor)
    })
}

/// List display backends that are available for the active target.
pub(crate) fn destack_display_backend_list(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<DisplayBackendDescriptorVm>> {
    let values =
        call_out(|out| unsafe { host_display::destack_display_backend_list(binding, out) })?;
    map_native_slice_to_vm(context, values, |context, descriptor| {
        display_backend_descriptor_to_vm(context, *descriptor)
    })
}

/// Convert one native monitor-event array into one VM array.
fn display_event_array_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    value: NativeArray<DisplayMonitorEvent>,
) -> RuntimeResult<VmArray<DisplayMonitorEventVm>> {
    map_native_array_to_vm(context, value, |context, event| {
        display_event_to_vm(context, *event)
    })
}

/// Convert one native window-event array into one VM array.
fn window_event_array_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    value: NativeArray<platform::display::WindowEvent>,
) -> RuntimeResult<VmArray<platform::display::WindowEventVm>> {
    map_native_array_to_vm(context, value, |context, event| {
        window_event_to_vm(context, *event)
    })
}

/// Convert one VM monitor-event filter payload to native.
fn monitor_event_filter_from_vm(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
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
    context: &mut destack_vm::BindingContext<'_>,
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
    context: &mut destack_vm::BindingContext<'_>,
    image: WindowIconImageVm,
) -> RuntimeResult<WindowIconImage> {
    let pixels = image.pixels.read_bytes(&context.read())?;

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
    context: &mut destack_vm::BindingContext<'_>,
    icon_set: WindowIconSetVm,
) -> RuntimeResult<WindowIconSet> {
    let images = icon_set.images.read_values(&context.read())?;
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
    context: &mut destack_vm::BindingContext<'_>,
    payload: WindowModePayload,
) -> RuntimeResult<WindowModePayloadVm> {
    Ok(WindowModePayloadVm {
        previous_mode: window_mode_options_to_vm(context, payload.previous_mode)?,
        current_mode: window_mode_options_to_vm(context, payload.current_mode)?,
    })
}

/// Convert one native window event into its VM representation.
fn window_event_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    value: WindowEvent,
) -> RuntimeResult<WindowEventVm> {
    match value {
        WindowEvent::WindowAspectRatioChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowAspectRatioChangedEvent(
                platform::display::WindowAspectRatioChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowChromeChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowChromeChangedEvent(
                platform::display::WindowChromeChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowCloseRequestedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowCloseRequestedEvent(
                platform::display::WindowCloseRequestedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                },
            ))
        }
        WindowEvent::WindowCreatedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowCreatedEvent(
                platform::display::WindowCreatedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                },
            ))
        }
        WindowEvent::WindowDestroyedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowDestroyedEvent(
                platform::display::WindowDestroyedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                },
            ))
        }
        WindowEvent::WindowDisplayChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowDisplayChangedEvent(
                platform::display::WindowDisplayChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowContentRectChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowContentRectChangedEvent(
                platform::display::WindowContentRectChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowDragEnteredEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowDragEnteredEvent(
                platform::display::WindowDragEnteredEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: vm_value_from_native(context, event.payload)?,
                },
            ))
        }
        WindowEvent::WindowDragExitedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowDragExitedEvent(
                platform::display::WindowDragExitedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: vm_value_from_native(context, event.payload)?,
                },
            ))
        }
        WindowEvent::WindowDragUpdatedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowDragUpdatedEvent(
                platform::display::WindowDragUpdatedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: vm_value_from_native(context, event.payload)?,
                },
            ))
        }
        WindowEvent::WindowDroppedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowDroppedEvent(
                platform::display::WindowDroppedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: vm_value_from_native(context, event.payload)?,
                },
            ))
        }
        WindowEvent::WindowFocusChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowFocusChangedEvent(
                platform::display::WindowFocusChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowFramebufferSizeChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowFramebufferSizeChangedEvent(
                platform::display::WindowFramebufferSizeChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowModalChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowModalChangedEvent(
                platform::display::WindowModalChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowModeChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowModeChangedEvent(
                platform::display::WindowModeChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: window_mode_payload_to_vm(context, event.payload)?,
                },
            ))
        }
        WindowEvent::WindowMousePassthroughChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowMousePassthroughChangedEvent(
                platform::display::WindowMousePassthroughChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowOcclusionChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowOcclusionChangedEvent(
                platform::display::WindowOcclusionChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowOpacityChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowOpacityChangedEvent(
                platform::display::WindowOpacityChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowParentChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowParentChangedEvent(
                platform::display::WindowParentChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowPositionChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowPositionChangedEvent(
                platform::display::WindowPositionChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowSafeAreaChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowSafeAreaChangedEvent(
                platform::display::WindowSafeAreaChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowRenderStateChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowRenderStateChangedEvent(
                platform::display::WindowRenderStateChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowScaleFactorChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowScaleFactorChangedEvent(
                platform::display::WindowScaleFactorChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowSizeChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowSizeChangedEvent(
                platform::display::WindowSizeChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowTaskbarVisibilityChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowTaskbarVisibilityChangedEvent(
                platform::display::WindowTaskbarVisibilityChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowThemeChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowThemeChangedEvent(
                platform::display::WindowThemeChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowTransientChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowTransientChangedEvent(
                platform::display::WindowTransientChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
                    metadata: event.metadata,
                    payload: event.payload,
                },
            ))
        }
        WindowEvent::WindowVisibilityChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowVisibilityChangedEvent(
                platform::display::WindowVisibilityChangedEventVm {
                    kind: context
                        .string_handle(kind)
                        .map_err(Box::<RuntimeError>::from)?,
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
    context: &mut destack_vm::BindingContext<'_>,
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
        role: options.role,
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

/// Convert one native begin-frame event to VM.
fn begin_frame_event_to_vm(
    _context: &mut destack_vm::BindingContext<'_>,
    value: DisplayBeginFrameEvent,
) -> RuntimeResult<DisplayBeginFrameEventVm> {
    Ok(value)
}

/// Close one display endpoint.
pub(crate) fn destack_display_monitor_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_monitor_close(binding, handle) }
}

/// Resolve one requested mode to the closest supported mode.
pub(crate) fn destack_display_monitor_closest_mode(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
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
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<DisplayModeVm> {
    call_out(|out| unsafe {
        host_display::destack_display_monitor_current_mode(binding, out, handle)
    })
}

/// Read descriptor metadata for one display.
pub(crate) fn destack_display_monitor_descriptor(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
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
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<DisplayModeVm> {
    call_out(|out| unsafe {
        host_display::destack_display_monitor_desktop_mode(binding, out, handle)
    })
}

/// Close one monitor-event stream.
pub(crate) fn destack_display_monitor_event_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_monitor_event_close(binding, handle) }
}

/// Open one monitor-event stream.
pub(crate) fn destack_display_monitor_event_open(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
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
    context: &mut destack_vm::BindingContext<'_>,
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
    context: &mut destack_vm::BindingContext<'_>,
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
    context: &mut destack_vm::BindingContext<'_>,
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
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::DisplayEventHandle,
    maxevents: u32,
) -> RuntimeResult<VmArray<DisplayMonitorEventVm>> {
    let events = call_out(|out| unsafe {
        host_display::destack_display_monitor_event_try_read_batch(binding, out, handle, maxevents)
    })?;

    display_event_array_to_vm(context, events)
}

/// Open one begin-frame stream.
pub(crate) fn destack_display_begin_frame_open(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    options: DisplayBeginFrameOpenOptionsVm,
) -> RuntimeResult<resource::DisplayBeginFrameHandle> {
    let _ = options;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.frame.beginOpen",
    ))
    .boxed())
}

/// Close one begin-frame stream.
pub(crate) fn destack_display_begin_frame_close(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::DisplayBeginFrameHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.frame.beginClose",
    ))
    .boxed())
}

/// Wait for one begin-frame event.
pub(crate) fn destack_display_begin_frame_read(
    _binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::DisplayBeginFrameHandle,
    timeoutns: u64,
) -> RuntimeResult<DisplayBeginFrameEventVm> {
    let _ = (handle, timeoutns);

    begin_frame_event_to_vm(
        context,
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.display.frame.beginRead",
        ))
        .boxed())?,
    )
}

/// Wait for one batch of begin-frame events.
pub(crate) fn destack_display_begin_frame_read_batch(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::DisplayBeginFrameHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<VmArray<DisplayBeginFrameEventVm>> {
    let _ = (handle, maxevents, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.frame.beginReadBatch",
    ))
    .boxed())
}

/// Poll one begin-frame event without blocking.
pub(crate) fn destack_display_begin_frame_try_read(
    _binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::DisplayBeginFrameHandle,
) -> RuntimeResult<DisplayBeginFrameEventVm> {
    let _ = handle;

    begin_frame_event_to_vm(
        context,
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.display.frame.beginTryRead",
        ))
        .boxed())?,
    )
}

/// Poll one batch of begin-frame events without blocking.
pub(crate) fn destack_display_begin_frame_try_read_batch(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::DisplayBeginFrameHandle,
    maxevents: u32,
) -> RuntimeResult<VmArray<DisplayBeginFrameEventVm>> {
    let _ = (handle, maxevents);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.frame.beginTryReadBatch",
    ))
    .boxed())
}

/// List available displays.
pub(crate) fn destack_display_monitor_list(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
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
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<VmSlice<DisplayModeVm>> {
    let modes = call_out(|out| unsafe {
        host_display::destack_display_monitor_modes(binding, out, handle)
    })?;
    let modes = unsafe { modes.as_slice()? };

    VmSlice::from_values(&mut context.write(), modes)
}

/// Open one display endpoint.
pub(crate) fn destack_display_monitor_open(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    id: destack_vm::StringHandle,
    options: DisplayMonitorOpenOptionsVm,
) -> RuntimeResult<resource::DisplayHandle> {
    let id = string_from_vm(binding, context, id)?;
    call_out(|out| unsafe { host_display::destack_display_monitor_open(binding, out, id, options) })
}

/// Read one primary display handle when available.
pub(crate) fn destack_display_monitor_primary(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    request: DisplayMonitorListRequestVm,
) -> RuntimeResult<Option<resource::DisplayHandle>> {
    call_out(|out| unsafe { host_display::destack_display_monitor_primary(binding, out, request) })
}

/// Apply one display mode.
pub(crate) fn destack_display_monitor_set_mode(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
    mode: DisplayModeVm,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_monitor_set_mode(binding, handle, mode) }
}

/// Close one window.
pub(crate) fn destack_display_window_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_close(binding, window) }
}

/// Read descriptor metadata for one window.
pub(crate) fn destack_display_window_descriptor(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<WindowDescriptorVm> {
    let descriptor = call_out(|out| unsafe {
        host_display::destack_display_window_descriptor(binding, out, window)
    })?;

    window_descriptor_to_vm(context, descriptor)
}

/// Read one logical content rectangle.
pub(crate) fn destack_display_window_content_rect(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<WindowLogicalRectVm> {
    let _ = window;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.contentRect",
    ))
    .boxed())
}

/// Read one drawable framebuffer size.
pub(crate) fn destack_display_window_framebuffer_size(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<WindowPhysicalSizeVm> {
    let _ = window;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.framebufferSize",
    ))
    .boxed())
}

/// Read one capability mask for one opened window backend.
pub(crate) fn destack_display_window_capabilities(
    runtime: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<DisplayBackendCapabilityFlags> {
    call_out(|out| unsafe {
        host_display::destack_display_window_capabilities(runtime, out, window)
    })
}

/// Close one window-event stream.
pub(crate) fn destack_display_window_event_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_event_close(binding, handle) }
}

/// Open one window-event stream.
pub(crate) fn destack_display_window_event_open(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    options: WindowEventOpenOptionsVm,
) -> RuntimeResult<resource::WindowEventHandle> {
    call_out(|out| unsafe {
        host_display::destack_display_window_event_open(binding, out, options)
    })
}

/// Wait for one window event.
pub(crate) fn destack_display_window_event_read(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
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
    context: &mut destack_vm::BindingContext<'_>,
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
    context: &mut destack_vm::BindingContext<'_>,
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
    context: &mut destack_vm::BindingContext<'_>,
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
    context: &mut destack_vm::BindingContext<'_>,
    options: WindowOptionsVm,
) -> RuntimeResult<resource::WindowHandle> {
    let options = window_options_from_vm(binding, context, options)?;
    call_out(|out| unsafe { host_display::destack_display_window_open(binding, out, options) })
}

/// Request user attention for one window.
pub(crate) fn destack_display_window_request_attention(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    level: platform::display::WindowAttentionLevel,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_request_attention(binding, window, level) }
}

/// Request one redraw for one window.
pub(crate) fn destack_display_window_request_refresh(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_request_refresh(binding, window) }
}

/// Invalidate one window and schedule another frame.
pub(crate) fn destack_display_window_invalidate(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    destack_display_window_request_refresh(binding, context, window)
}

/// Set always-on-top state.
pub(crate) fn destack_display_window_set_always_on_top(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    alwaysontop: bool,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_always_on_top(binding, window, alwaysontop) }
}

/// Set cursor icon for one window.
pub(crate) fn destack_display_window_set_cursor_icon(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    icon: platform::display::WindowCursorIcon,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_cursor_icon(binding, window, icon) }
}

/// Set cursor interaction mode for one window.
pub(crate) fn destack_display_window_set_cursor_mode(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    mode: platform::display::WindowCursorMode,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_cursor_mode(binding, window, mode) }
}

/// Set cursor position for one window.
pub(crate) fn destack_display_window_set_cursor_position(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    position: platform::display::WindowPositionVm,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_cursor_position(binding, window, position) }
}

/// Set cursor visibility for one window.
pub(crate) fn destack_display_window_set_cursor_visible(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_cursor_visible(binding, window, visible) }
}

/// Set window decoration state.
pub(crate) fn destack_display_window_set_decorated(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_decorated(binding, window, decorated) }
}

/// Set one window mode.
pub(crate) fn destack_display_window_set_mode(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    mode: platform::display::WindowModeOptionsVm,
) -> RuntimeResult<()> {
    let mode = window_mode_options_from_vm(binding, context, mode)?;
    unsafe { host_display::destack_display_window_set_mode(binding, window, mode) }
}

/// Set one window position.
pub(crate) fn destack_display_window_set_position(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    position: platform::display::WindowPositionVm,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_position(binding, window, position) }
}

/// Set window resizable state.
pub(crate) fn destack_display_window_set_resizable(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_resizable(binding, window, resizable) }
}

/// Set one logical window size.
pub(crate) fn destack_display_window_set_size_logical(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    size: platform::display::WindowLogicalSizeVm,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_size_logical(binding, window, size) }
}

/// Set one logical size-constraint payload.
pub(crate) fn destack_display_window_set_size_constraints(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    constraints: Option<platform::display::WindowSizeConstraintsVm>,
) -> RuntimeResult<()> {
    unsafe {
        host_display::destack_display_window_set_size_constraints(binding, window, constraints)
    }
}

/// Set one physical window size.
pub(crate) fn destack_display_window_set_size_physical(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    size: platform::display::WindowPhysicalSizeVm,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_size_physical(binding, window, size) }
}

/// Set one window title string.
pub(crate) fn destack_display_window_set_title(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    title: destack_vm::StringHandle,
) -> RuntimeResult<()> {
    let title = string_from_vm(binding, context, title)?;
    unsafe { host_display::destack_display_window_set_title(binding, window, title) }
}

/// Set one window visibility state.
pub(crate) fn destack_display_window_set_visibility(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    visibility: platform::display::WindowVisibility,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_visibility(binding, window, visibility) }
}

/// Read one window state snapshot.
pub(crate) fn destack_display_window_state(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<platform::display::WindowStateVm> {
    call_out(|out| unsafe { host_display::destack_display_window_state(binding, out, window) })
}

/// Read one current render state.
pub(crate) fn destack_display_window_render_state(
    _binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<WindowRenderState> {
    let _ = window;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.renderState",
    ))
    .boxed())
}

/// Read display color state.
pub(crate) fn destack_display_monitor_color_state(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<platform::display::DisplayColorState> {
    call_out(|out| unsafe {
        host_display::destack_display_monitor_color_state(binding, out, handle)
    })
}

/// Read display gamma ramp.
pub(crate) fn destack_display_monitor_gamma_ramp(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<DisplayGammaRampVm> {
    let ramp = call_out(|out| unsafe {
        host_display::destack_display_monitor_gamma_ramp(binding, out, handle)
    })?;
    let red = unsafe { ramp.red.as_slice()? };
    let green = unsafe { ramp.green.as_slice()? };
    let blue = unsafe { ramp.blue.as_slice()? };

    Ok(DisplayGammaRampVm {
        red: VmSlice::from_values(&mut context.write(), red)?,
        green: VmSlice::from_values(&mut context.write(), green)?,
        blue: VmSlice::from_values(&mut context.write(), blue)?,
    })
}

/// Read display HDR mode.
pub(crate) fn destack_display_monitor_hdr_mode(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<platform::display::DisplayHdrMode> {
    call_out(|out| unsafe { host_display::destack_display_monitor_hdr_mode(binding, out, handle) })
}

/// Set display gamma ramp.
pub(crate) fn destack_display_monitor_set_gamma_ramp(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
    ramp: DisplayGammaRampVm,
) -> RuntimeResult<()> {
    let red = binding.store_slice(ramp.red.read_values(&context.read())?);
    let green = binding.store_slice(ramp.green.read_values(&context.read())?);
    let blue = binding.store_slice(ramp.blue.read_values(&context.read())?);
    let ramp = DisplayGammaRamp { red, green, blue };

    unsafe { host_display::destack_display_monitor_set_gamma_ramp(binding, handle, ramp) }
}

/// Set display HDR mode.
pub(crate) fn destack_display_monitor_set_hdr_mode(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
    mode: platform::display::DisplayHdrMode,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_monitor_set_hdr_mode(binding, handle, mode) }
}

/// Begin one native move-drag interaction.
pub(crate) fn destack_display_window_begin_move_drag(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_begin_move_drag(binding, window) }
}

/// Begin one native resize-drag interaction.
pub(crate) fn destack_display_window_begin_resize_drag(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    edge: platform::display::WindowResizeEdge,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_begin_resize_drag(binding, window, edge) }
}

/// Focus one window.
pub(crate) fn destack_display_window_focus(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_focus(binding, window) }
}

/// Maximize one window.
pub(crate) fn destack_display_window_maximize(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_maximize(binding, window) }
}

/// Minimize one window.
pub(crate) fn destack_display_window_minimize(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_minimize(binding, window) }
}

/// Read one window opacity.
pub(crate) fn destack_display_window_opacity(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<f64> {
    call_out(|out| unsafe { host_display::destack_display_window_opacity(binding, out, window) })
}

/// Raise one window.
pub(crate) fn destack_display_window_raise(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_raise(binding, window) }
}

/// Restore one window.
pub(crate) fn destack_display_window_restore(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_restore(binding, window) }
}

/// Set one window aspect-ratio lock.
pub(crate) fn destack_display_window_set_aspect_ratio(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    aspectratio: Option<platform::display::WindowAspectRatio>,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_aspect_ratio(binding, window, aspectratio) }
}

/// Set one window chrome style.
pub(crate) fn destack_display_window_set_chrome(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    chrome: platform::display::WindowChromeKind,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_chrome(binding, window, chrome) }
}

/// Set one window icon set.
pub(crate) fn destack_display_window_set_icons(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
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
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    modal: bool,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_modal(binding, window, modal) }
}

/// Set one window mouse-passthrough state.
pub(crate) fn destack_display_window_set_mouse_passthrough(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
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
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    opacity: f64,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_opacity(binding, window, opacity) }
}

/// Set one window parent relationship.
pub(crate) fn destack_display_window_set_parent(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    parent: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_parent(binding, window, parent) }
}

/// Set one window taskbar visibility state.
pub(crate) fn destack_display_window_set_taskbar_visible(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_taskbar_visible(binding, window, visible) }
}

/// Set one window transient-owner relationship.
pub(crate) fn destack_display_window_set_transient_for(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    window: resource::WindowHandle,
    transientfor: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_transient_for(binding, window, transientfor) }
}
