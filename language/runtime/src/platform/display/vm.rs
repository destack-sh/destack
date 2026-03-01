use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::display::{
    DisplayBackendDescriptor, DisplayBackendDescriptorVm, DisplayDescriptor, DisplayDescriptorVm,
    DisplayEvent, DisplayEventVm, DisplayMode, DisplayModeVm, DisplayMonitorEventOpenOptionsVm,
    DisplayMonitorListRequestVm, DisplayMonitorOpenOptionsVm, WindowDescriptor, WindowDescriptorVm,
    WindowEvent, WindowEventOpenOptionsVm, WindowEventVm, WindowOptions, WindowOptionsVm,
    host as host_display,
};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, VmArray, VmSlice, resource};
use crate::runtime::BindingCallContext;

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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<NativeStringRef> {
    let value = context
        .string_ref(value)
        .map_err(|error| RuntimeError::from(error).boxed())?;
    Ok(runtime.store_string(value.as_str()))
}

/// Convert one native display descriptor into its VM representation.
fn display_descriptor_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: DisplayDescriptor,
) -> RuntimeResult<DisplayDescriptorVm> {
    let id = unsafe { value.id.as_str()? };
    let name = unsafe { value.name.as_str()? };

    Ok(DisplayDescriptorVm {
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
        is_builtin: value.is_builtin,
        supports_variable_refresh: value.supports_variable_refresh,
        supports_hdr: value.supports_hdr,
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
    value: DisplayEvent,
) -> RuntimeResult<DisplayEventVm> {
    match value {
        DisplayEvent::DisplayAddedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            let metadata = display_event_metadata_to_vm(context, event.metadata)?;
            let descriptor = display_descriptor_to_vm(context, event.payload.descriptor)?;
            Ok(DisplayEventVm::DisplayAddedEvent(
                crate::platform::display::DisplayAddedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata,
                    payload: crate::platform::display::DisplayAddedPayloadVm { descriptor },
                },
            ))
        }
        DisplayEvent::DisplayDescriptorChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            let metadata = display_event_metadata_to_vm(context, event.metadata)?;
            let descriptor = display_descriptor_to_vm(context, event.payload.descriptor)?;
            Ok(DisplayEventVm::DisplayDescriptorChangedEvent(
                crate::platform::display::DisplayDescriptorChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata,
                    payload: crate::platform::display::DisplayDescriptorChangedPayloadVm {
                        descriptor,
                        changed_mask: event.payload.changed_mask,
                    },
                },
            ))
        }
        DisplayEvent::DisplayModeChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            let metadata = display_event_metadata_to_vm(context, event.metadata)?;
            Ok(DisplayEventVm::DisplayModeChangedEvent(
                crate::platform::display::DisplayModeChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata,
                    payload: event.payload,
                },
            ))
        }
        DisplayEvent::DisplayPrimaryChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            let metadata = display_event_metadata_to_vm(context, event.metadata)?;
            let payload_id = if let Some(value) = event.payload.id {
                let value = unsafe { value.as_str()? };
                Some(vm::StringHandle::new(context.intern_string(value)))
            } else {
                None
            };
            Ok(DisplayEventVm::DisplayPrimaryChangedEvent(
                crate::platform::display::DisplayPrimaryChangedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata,
                    payload: crate::platform::display::DisplayPrimaryPayloadVm { id: payload_id },
                },
            ))
        }
        DisplayEvent::DisplayRemovedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            let metadata = display_event_metadata_to_vm(context, event.metadata)?;
            let payload_id = unsafe { event.payload.id.as_str()? };
            Ok(DisplayEventVm::DisplayRemovedEvent(
                crate::platform::display::DisplayRemovedEventVm {
                    kind: vm::StringHandle::new(context.intern_string(kind)),
                    metadata,
                    payload: crate::platform::display::DisplayRemovedPayloadVm {
                        id: vm::StringHandle::new(context.intern_string(payload_id)),
                    },
                },
            ))
        }
    }
}

/// Convert one native display event metadata payload into its VM representation.
fn display_event_metadata_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: crate::platform::display::DisplayEventMetadata,
) -> RuntimeResult<crate::platform::display::DisplayEventMetadataVm> {
    let display_id = if let Some(value) = value.display_id {
        let value = unsafe { value.as_str()? };
        Some(vm::StringHandle::new(context.intern_string(value)))
    } else {
        None
    };

    Ok(crate::platform::display::DisplayEventMetadataVm {
        display_id,
        timestamp_ns: value.timestamp_ns,
        sequence: value.sequence,
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
        id: vm::StringHandle::new(context.intern_string(id)),
        title: vm::StringHandle::new(context.intern_string(title)),
        mode: value.mode,
        display: value.display,
        resizable: value.resizable,
        decorated: value.decorated,
        transparent: value.transparent,
        always_on_top: value.always_on_top,
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<DisplayBackendDescriptorVm>> {
    let values =
        call_out(|out| unsafe { host_display::destack_display_backend_list(runtime, out) })?;
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
    value: NativeArray<DisplayEvent>,
) -> RuntimeResult<VmArray<DisplayEventVm>> {
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

/// Convert one native window event into its VM representation.
fn window_event_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: WindowEvent,
) -> RuntimeResult<WindowEventVm> {
    match value {
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
        WindowEvent::WindowModeChangedEvent(event) => {
            let kind = unsafe { event.kind.as_str()? };
            Ok(WindowEventVm::WindowModeChangedEvent(
                crate::platform::display::WindowModeChangedEventVm {
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    options: WindowOptionsVm,
) -> RuntimeResult<WindowOptions> {
    let title = context
        .string_ref(options.title)
        .map_err(|error| RuntimeError::from(error).boxed())?;
    let title = runtime.store_string(title.as_str());

    Ok(WindowOptions {
        backend: options.backend,
        backend_policy: options.backend_policy,
        title,
        size_logical: options.size_logical,
        position: options.position,
        constraints: options.constraints,
        display: options.display,
        mode: options.mode,
        visibility: options.visibility,
        resizable: options.resizable,
        decorated: options.decorated,
        transparent: options.transparent,
        focus_on_show: options.focus_on_show,
        always_on_top: options.always_on_top,
    })
}

/// Close one display endpoint.
pub(crate) fn destack_display_monitor_close(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_monitor_close(runtime, handle) }
}

/// Resolve one requested mode to the closest supported mode.
pub(crate) fn destack_display_monitor_closest_mode(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
    requested: DisplayModeVm,
) -> RuntimeResult<DisplayModeVm> {
    call_out(|out| unsafe {
        host_display::destack_display_monitor_closest_mode(runtime, out, handle, requested)
    })
}

/// Read one point-in-time current mode for one display.
pub(crate) fn destack_display_monitor_current_mode(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<DisplayModeVm> {
    call_out(|out| unsafe {
        host_display::destack_display_monitor_current_mode(runtime, out, handle)
    })
}

/// Read descriptor metadata for one display.
pub(crate) fn destack_display_monitor_descriptor(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<DisplayDescriptorVm> {
    let descriptor = call_out(|out| unsafe {
        host_display::destack_display_monitor_descriptor(runtime, out, handle)
    })?;

    display_descriptor_to_vm(context, descriptor)
}

/// Read one point-in-time desktop mode for one display.
pub(crate) fn destack_display_monitor_desktop_mode(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<DisplayModeVm> {
    call_out(|out| unsafe {
        host_display::destack_display_monitor_desktop_mode(runtime, out, handle)
    })
}

/// Close one monitor-event stream.
pub(crate) fn destack_display_monitor_event_close(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_monitor_event_close(runtime, handle) }
}

/// Open one monitor-event stream.
pub(crate) fn destack_display_monitor_event_open(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    options: DisplayMonitorEventOpenOptionsVm,
) -> RuntimeResult<resource::DisplayEventHandle> {
    call_out(|out| unsafe {
        host_display::destack_display_monitor_event_open(runtime, out, options)
    })
}

/// Wait for one monitor event.
pub(crate) fn destack_display_monitor_event_read(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayEventHandle,
    timeoutns: u64,
) -> RuntimeResult<DisplayEventVm> {
    let event = call_out(|out| unsafe {
        host_display::destack_display_monitor_event_read(runtime, out, handle, timeoutns)
    })?;

    display_event_to_vm(context, event)
}

/// Wait for one batch of monitor events.
pub(crate) fn destack_display_monitor_event_read_batch(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<VmArray<DisplayEventVm>> {
    let events = call_out(|out| unsafe {
        host_display::destack_display_monitor_event_read_batch(
            runtime, out, handle, maxevents, timeoutns,
        )
    })?;

    display_event_array_to_vm(context, events)
}

/// Poll one monitor event without blocking.
pub(crate) fn destack_display_monitor_event_try_read(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<DisplayEventVm> {
    let event = call_out(|out| unsafe {
        host_display::destack_display_monitor_event_try_read(runtime, out, handle)
    })?;

    display_event_to_vm(context, event)
}

/// Poll one batch of monitor events without blocking.
pub(crate) fn destack_display_monitor_event_try_read_batch(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayEventHandle,
    maxevents: u32,
) -> RuntimeResult<VmArray<DisplayEventVm>> {
    let events = call_out(|out| unsafe {
        host_display::destack_display_monitor_event_try_read_batch(runtime, out, handle, maxevents)
    })?;

    display_event_array_to_vm(context, events)
}

/// List available displays.
pub(crate) fn destack_display_monitor_list(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    request: DisplayMonitorListRequestVm,
) -> RuntimeResult<VmSlice<DisplayDescriptorVm>> {
    let descriptors = call_out(|out| unsafe {
        host_display::destack_display_monitor_list(runtime, out, request)
    })?;
    descriptor_slice_to_vm(context, descriptors)
}

/// Read available display modes.
pub(crate) fn destack_display_monitor_modes(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<VmSlice<DisplayModeVm>> {
    let modes = call_out(|out| unsafe {
        host_display::destack_display_monitor_modes(runtime, out, handle)
    })?;
    mode_slice_to_vm(context, modes)
}

/// Open one display endpoint.
pub(crate) fn destack_display_monitor_open(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
    options: DisplayMonitorOpenOptionsVm,
) -> RuntimeResult<resource::DisplayHandle> {
    let id = string_from_vm(runtime, context, id)?;
    call_out(|out| unsafe { host_display::destack_display_monitor_open(runtime, out, id, options) })
}

/// Read one primary display handle when available.
pub(crate) fn destack_display_monitor_primary(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    request: DisplayMonitorListRequestVm,
) -> RuntimeResult<Option<resource::DisplayHandle>> {
    call_out(|out| unsafe { host_display::destack_display_monitor_primary(runtime, out, request) })
}

/// Apply one display mode.
pub(crate) fn destack_display_monitor_set_mode(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
    mode: DisplayModeVm,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_monitor_set_mode(runtime, handle, mode) }
}

/// Close one window.
pub(crate) fn destack_display_window_close(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_close(runtime, window) }
}

/// Read descriptor metadata for one window.
pub(crate) fn destack_display_window_descriptor(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<WindowDescriptorVm> {
    let descriptor = call_out(|out| unsafe {
        host_display::destack_display_window_descriptor(runtime, out, window)
    })?;

    window_descriptor_to_vm(context, descriptor)
}

/// Close one window-event stream.
pub(crate) fn destack_display_window_event_close(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_event_close(runtime, handle) }
}

/// Open one window-event stream.
pub(crate) fn destack_display_window_event_open(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    options: WindowEventOpenOptionsVm,
) -> RuntimeResult<resource::WindowEventHandle> {
    call_out(|out| unsafe {
        host_display::destack_display_window_event_open(runtime, out, options)
    })
}

/// Wait for one window event.
pub(crate) fn destack_display_window_event_read(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::WindowEventHandle,
    timeoutns: u64,
) -> RuntimeResult<WindowEventVm> {
    let event = call_out(|out| unsafe {
        host_display::destack_display_window_event_read(runtime, out, handle, timeoutns)
    })?;

    window_event_to_vm(context, event)
}

/// Wait for one batch of window events.
pub(crate) fn destack_display_window_event_read_batch(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<VmArray<WindowEventVm>> {
    let events = call_out(|out| unsafe {
        host_display::destack_display_window_event_read_batch(
            runtime, out, handle, maxevents, timeoutns,
        )
    })?;

    window_event_array_to_vm(context, events)
}

/// Poll one window event without blocking.
pub(crate) fn destack_display_window_event_try_read(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<WindowEventVm> {
    let event = call_out(|out| unsafe {
        host_display::destack_display_window_event_try_read(runtime, out, handle)
    })?;

    window_event_to_vm(context, event)
}

/// Poll one batch of window events without blocking.
pub(crate) fn destack_display_window_event_try_read_batch(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
) -> RuntimeResult<VmArray<WindowEventVm>> {
    let events = call_out(|out| unsafe {
        host_display::destack_display_window_event_try_read_batch(runtime, out, handle, maxevents)
    })?;

    window_event_array_to_vm(context, events)
}

/// Open one window.
pub(crate) fn destack_display_window_open(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    options: WindowOptionsVm,
) -> RuntimeResult<resource::WindowHandle> {
    let options = window_options_from_vm(runtime, context, options)?;
    call_out(|out| unsafe { host_display::destack_display_window_open(runtime, out, options) })
}

/// Request user attention for one window.
pub(crate) fn destack_display_window_request_attention(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    level: crate::platform::display::WindowAttentionLevel,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_request_attention(runtime, window, level) }
}

/// Request one redraw for one window.
pub(crate) fn destack_display_window_request_refresh(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_request_refresh(runtime, window) }
}

/// Set always-on-top state.
pub(crate) fn destack_display_window_set_always_on_top(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    alwaysontop: bool,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_always_on_top(runtime, window, alwaysontop) }
}

/// Set cursor icon for one window.
pub(crate) fn destack_display_window_set_cursor_icon(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    icon: crate::platform::display::WindowCursorIcon,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_cursor_icon(runtime, window, icon) }
}

/// Set cursor interaction mode for one window.
pub(crate) fn destack_display_window_set_cursor_mode(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    mode: crate::platform::display::WindowCursorMode,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_cursor_mode(runtime, window, mode) }
}

/// Set cursor position for one window.
pub(crate) fn destack_display_window_set_cursor_position(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    position: crate::platform::display::WindowPositionVm,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_cursor_position(runtime, window, position) }
}

/// Set cursor visibility for one window.
pub(crate) fn destack_display_window_set_cursor_visible(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_cursor_visible(runtime, window, visible) }
}

/// Set window decoration state.
pub(crate) fn destack_display_window_set_decorated(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_decorated(runtime, window, decorated) }
}

/// Set one window mode.
pub(crate) fn destack_display_window_set_mode(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    mode: crate::platform::display::WindowModeOptionsVm,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_mode(runtime, window, mode) }
}

/// Set one window position.
pub(crate) fn destack_display_window_set_position(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    position: crate::platform::display::WindowPositionVm,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_position(runtime, window, position) }
}

/// Set window resizable state.
pub(crate) fn destack_display_window_set_resizable(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_resizable(runtime, window, resizable) }
}

/// Set one logical window size.
pub(crate) fn destack_display_window_set_size_logical(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    size: crate::platform::display::WindowLogicalSizeVm,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_size_logical(runtime, window, size) }
}

/// Set one logical size-constraint payload.
pub(crate) fn destack_display_window_set_size_constraints(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    constraints: Option<crate::platform::display::WindowSizeConstraintsVm>,
) -> RuntimeResult<()> {
    unsafe {
        host_display::destack_display_window_set_size_constraints(runtime, window, constraints)
    }
}

/// Set one physical window size.
pub(crate) fn destack_display_window_set_size_physical(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    size: crate::platform::display::WindowPhysicalSizeVm,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_size_physical(runtime, window, size) }
}

/// Set one window title string.
pub(crate) fn destack_display_window_set_title(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    title: vm::StringHandle,
) -> RuntimeResult<()> {
    let title = string_from_vm(runtime, context, title)?;
    unsafe { host_display::destack_display_window_set_title(runtime, window, title) }
}

/// Set one window visibility state.
pub(crate) fn destack_display_window_set_visibility(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    visibility: crate::platform::display::WindowVisibility,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_set_visibility(runtime, window, visibility) }
}

/// Read one window state snapshot.
pub(crate) fn destack_display_window_state(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<crate::platform::display::WindowStateVm> {
    call_out(|out| unsafe { host_display::destack_display_window_state(runtime, out, window) })
}

/// Wait for one present interval.
pub(crate) fn destack_display_window_vsync_wait(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { host_display::destack_display_window_vsync_wait(runtime, window, timeoutns) }
}
