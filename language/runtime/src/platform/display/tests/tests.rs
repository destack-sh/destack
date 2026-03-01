#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{NativeSlice, NativeStringRef, PlatformError, VmSlice, display};
use crate::runtime::BindingCallContext;
use crate::tests::runtime::TestRuntime;

#[path = "harness.generated.rs"]
mod harness;
pub(crate) use harness::HarnessValue;

/// Test harness context used by tests.
pub(crate) struct DisplayHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native display harness.
pub(crate) struct NativeDisplayHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeDisplayHarness {
    /// Create a new native display harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM display harness.
pub(crate) struct VmDisplayHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmDisplayHarness {
    /// Create a new VM display harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum DisplayHarnessHandle {
    /// Native display harness.
    Native(NativeDisplayHarness),
    /// VM display harness.
    Vm(VmDisplayHarness),
}

impl DisplayHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(DisplayHarnessContext<'call>) -> R,
    {
        match self {
            DisplayHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(DisplayHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            DisplayHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(DisplayHarnessContext {
                            call_context,
                            vm_context: Some(vm_context),
                        })
                    })
            }
        }
    }

    /// Run one callback that returns a runtime result.
    pub(crate) fn run<F>(&self, callback: F)
    where
        F: for<'call> FnOnce(DisplayHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("display harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&DisplayHarnessHandle),
{
    let native = DisplayHarnessHandle::Native(NativeDisplayHarness::new());
    callback(&native);
    let vm = DisplayHarnessHandle::Vm(VmDisplayHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(DisplayHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}

/// Build one harness string payload for native and VM binding calls.
pub(crate) fn harness_string(
    context: &mut DisplayHarnessContext<'_>,
    value: &str,
) -> RuntimeResult<HarnessValue<NativeStringRef, vm::StringHandle>> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
            Ok(HarnessValue::Vm(vm::StringHandle::new(
                vm_context.intern_string(value),
            )))
        }
        None => Ok(HarnessValue::Native(
            context.call_context.store_string(value),
        )),
    }
}

/// Decode one monitor list payload into plain Rust records.
pub(crate) fn decode_monitor_list(
    context: &mut DisplayHarnessContext<'_>,
    value: HarnessValue<
        NativeSlice<display::DisplayDescriptor>,
        VmSlice<display::DisplayDescriptorVm>,
    >,
) -> RuntimeResult<Vec<(String, String, bool)>> {
    match value {
        HarnessValue::Native(values) => {
            let values = unsafe { values.as_slice()? };
            let mut decoded = Vec::with_capacity(values.len());

            for value in values {
                let id = unsafe { value.id.as_str()? }.to_string();
                let name = unsafe { value.name.as_str()? }.to_string();
                decoded.push((id, name, value.primary));
            }

            Ok(decoded)
        }
        HarnessValue::Vm(values) => {
            let Some(vm_context) = context.vm_context else {
                return Err(RuntimeError::from(PlatformError::invalid_argument(
                    "missing vm context",
                ))
                .boxed());
            };
            let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
            let values = values.read_values(vm_context)?;
            let mut decoded = Vec::with_capacity(values.len());

            for value in values {
                let id = vm_context
                    .string_ref(value.id)
                    .map_err(|error| RuntimeError::from(error).boxed())?
                    .as_str()
                    .to_string();
                let name = vm_context
                    .string_ref(value.name)
                    .map_err(|error| RuntimeError::from(error).boxed())?
                    .as_str()
                    .to_string();
                decoded.push((id, name, value.primary));
            }

            Ok(decoded)
        }
    }
}

/// Decode one monitor modes payload.
pub(crate) fn decode_monitor_modes(
    context: &mut DisplayHarnessContext<'_>,
    value: HarnessValue<NativeSlice<display::DisplayMode>, VmSlice<display::DisplayModeVm>>,
) -> RuntimeResult<Vec<display::DisplayMode>> {
    match value {
        HarnessValue::Native(values) => {
            let values = unsafe { values.as_slice()? };
            Ok(values.to_vec())
        }
        HarnessValue::Vm(values) => {
            let Some(vm_context) = context.vm_context else {
                return Err(RuntimeError::from(PlatformError::invalid_argument(
                    "missing vm context",
                ))
                .boxed());
            };
            let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
            Ok(values.read_values(vm_context)?)
        }
    }
}

/// Decode one display descriptor payload into plain Rust values.
pub(crate) fn decode_display_descriptor(
    context: &mut DisplayHarnessContext<'_>,
    value: HarnessValue<display::DisplayDescriptor, display::DisplayDescriptorVm>,
) -> RuntimeResult<(String, String, bool)> {
    match value {
        HarnessValue::Native(value) => {
            let id = unsafe { value.id.as_str()? }.to_string();
            let name = unsafe { value.name.as_str()? }.to_string();
            Ok((id, name, value.primary))
        }
        HarnessValue::Vm(value) => {
            let Some(vm_context) = context.vm_context else {
                return Err(RuntimeError::from(PlatformError::invalid_argument(
                    "missing vm context",
                ))
                .boxed());
            };
            let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
            let id = vm_context
                .string_ref(value.id)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string();
            let name = vm_context
                .string_ref(value.name)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string();
            Ok((id, name, value.primary))
        }
    }
}

/// Decode one display descriptor payload into monitor capability metrics.
#[cfg(windows)]
pub(crate) fn decode_display_descriptor_metrics(
    context: &mut DisplayHarnessContext<'_>,
    value: HarnessValue<display::DisplayDescriptor, display::DisplayDescriptorVm>,
) -> RuntimeResult<(display::DisplayOrientation, bool, bool, bool, u32, u32)> {
    match value {
        HarnessValue::Native(value) => Ok((
            value.orientation,
            value.is_builtin,
            value.supports_variable_refresh,
            value.supports_hdr,
            value.width_px,
            value.height_px,
        )),
        HarnessValue::Vm(value) => {
            let Some(vm_context) = context.vm_context else {
                return Err(RuntimeError::from(PlatformError::invalid_argument(
                    "missing vm context",
                ))
                .boxed());
            };
            let _vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
            Ok((
                value.orientation,
                value.is_builtin,
                value.supports_variable_refresh,
                value.supports_hdr,
                value.width_px,
                value.height_px,
            ))
        }
    }
}

/// Decode one window descriptor payload into plain Rust values.
pub(crate) fn decode_window_descriptor(
    context: &mut DisplayHarnessContext<'_>,
    value: HarnessValue<display::WindowDescriptor, display::WindowDescriptorVm>,
) -> RuntimeResult<(String, String)> {
    match value {
        HarnessValue::Native(value) => {
            let id = unsafe { value.id.as_str()? }.to_string();
            let title = unsafe { value.title.as_str()? }.to_string();
            Ok((id, title))
        }
        HarnessValue::Vm(value) => {
            let Some(vm_context) = context.vm_context else {
                return Err(RuntimeError::from(PlatformError::invalid_argument(
                    "missing vm context",
                ))
                .boxed());
            };
            let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
            let id = vm_context
                .string_ref(value.id)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string();
            let title = vm_context
                .string_ref(value.title)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string();
            Ok((id, title))
        }
    }
}

/// Build one default window-options payload for harness calls.
pub(crate) fn default_window_options(
    context: &mut DisplayHarnessContext<'_>,
    title: &str,
) -> RuntimeResult<HarnessValue<display::WindowOptions, display::WindowOptionsVm>> {
    let options = match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
            HarnessValue::Vm(display::WindowOptionsVm {
                backend: display::DisplayBackend::Auto,
                backend_policy: display::DisplayBackendSelectionPolicy::AllowFallback,
                title: vm::StringHandle::new(vm_context.intern_string(title)),
                size_logical: display::WindowLogicalSizeVm {
                    width: 1280.0,
                    height: 720.0,
                },
                position: Some(display::WindowPositionVm { x: 40, y: 50 }),
                constraints: None,
                display: None,
                mode: display::WindowModeOptionsVm {
                    mode: display::WindowMode::Windowed,
                    display: None,
                    display_mode: None,
                },
                visibility: display::WindowVisibility::Visible,
                resizable: true,
                decorated: true,
                transparent: false,
                focus_on_show: true,
                always_on_top: false,
            })
        }
        None => HarnessValue::Native(display::WindowOptions {
            backend: display::DisplayBackend::Auto,
            backend_policy: display::DisplayBackendSelectionPolicy::AllowFallback,
            title: context.call_context.store_string(title),
            size_logical: display::WindowLogicalSize {
                width: 1280.0,
                height: 720.0,
            },
            position: Some(display::WindowPosition { x: 40, y: 50 }),
            constraints: None,
            display: None,
            mode: display::WindowModeOptions {
                mode: display::WindowMode::Windowed,
                display: None,
                display_mode: None,
            },
            visibility: display::WindowVisibility::Visible,
            resizable: true,
            decorated: true,
            transparent: false,
            focus_on_show: true,
            always_on_top: false,
        }),
    };

    Ok(options)
}

/// Build one default monitor-list request payload for harness calls.
pub(crate) fn default_monitor_list_request(
    context: &DisplayHarnessContext<'_>,
) -> HarnessValue<display::DisplayMonitorListRequest, display::DisplayMonitorListRequestVm> {
    let request = display::DisplayMonitorListRequest {
        backend: display::DisplayBackend::Auto,
        backend_policy: display::DisplayBackendSelectionPolicy::AllowFallback,
    };

    if context.vm_context.is_some() {
        HarnessValue::Vm(request)
    } else {
        HarnessValue::Native(request)
    }
}

/// Build one default monitor-open options payload for harness calls.
pub(crate) fn default_monitor_open_options(
    context: &DisplayHarnessContext<'_>,
) -> HarnessValue<display::DisplayMonitorOpenOptions, display::DisplayMonitorOpenOptionsVm> {
    let options = display::DisplayMonitorOpenOptions {
        backend: display::DisplayBackend::Auto,
        backend_policy: display::DisplayBackendSelectionPolicy::AllowFallback,
    };

    if context.vm_context.is_some() {
        HarnessValue::Vm(options)
    } else {
        HarnessValue::Native(options)
    }
}

/// Build one default monitor-event open options payload for harness calls.
pub(crate) fn default_monitor_event_open_options(
    context: &DisplayHarnessContext<'_>,
) -> HarnessValue<display::DisplayMonitorEventOpenOptions, display::DisplayMonitorEventOpenOptionsVm>
{
    let options = display::DisplayMonitorEventOpenOptions {
        backend: display::DisplayBackend::Auto,
        backend_policy: display::DisplayBackendSelectionPolicy::AllowFallback,
        queue: display::DisplayEventQueueOptions {
            queue_capacity: 256,
            overflow_policy: display::DisplayEventOverflowPolicy::DropOldest,
        },
    };

    if context.vm_context.is_some() {
        HarnessValue::Vm(options)
    } else {
        HarnessValue::Native(options)
    }
}

/// Build one monitor-event open options payload for custom queue behavior.
pub(crate) fn monitor_event_open_options(
    context: &DisplayHarnessContext<'_>,
    queue_capacity: u32,
    overflow_policy: display::DisplayEventOverflowPolicy,
) -> HarnessValue<display::DisplayMonitorEventOpenOptions, display::DisplayMonitorEventOpenOptionsVm>
{
    let options = display::DisplayMonitorEventOpenOptions {
        backend: display::DisplayBackend::Auto,
        backend_policy: display::DisplayBackendSelectionPolicy::AllowFallback,
        queue: display::DisplayEventQueueOptions {
            queue_capacity,
            overflow_policy,
        },
    };

    if context.vm_context.is_some() {
        HarnessValue::Vm(options)
    } else {
        HarnessValue::Native(options)
    }
}

/// Build one default window-event open options payload for harness calls.
pub(crate) fn default_window_event_open_options(
    context: &DisplayHarnessContext<'_>,
) -> HarnessValue<display::WindowEventOpenOptions, display::WindowEventOpenOptionsVm> {
    let options = display::WindowEventOpenOptions {
        backend: display::DisplayBackend::Auto,
        backend_policy: display::DisplayBackendSelectionPolicy::AllowFallback,
        queue: display::DisplayEventQueueOptions {
            queue_capacity: 256,
            overflow_policy: display::DisplayEventOverflowPolicy::DropOldest,
        },
    };

    if context.vm_context.is_some() {
        HarnessValue::Vm(options)
    } else {
        HarnessValue::Native(options)
    }
}

/// Build one window-event open options payload for custom queue behavior.
pub(crate) fn window_event_open_options(
    context: &DisplayHarnessContext<'_>,
    queue_capacity: u32,
    overflow_policy: display::DisplayEventOverflowPolicy,
) -> HarnessValue<display::WindowEventOpenOptions, display::WindowEventOpenOptionsVm> {
    let options = display::WindowEventOpenOptions {
        backend: display::DisplayBackend::Auto,
        backend_policy: display::DisplayBackendSelectionPolicy::AllowFallback,
        queue: display::DisplayEventQueueOptions {
            queue_capacity,
            overflow_policy,
        },
    };

    if context.vm_context.is_some() {
        HarnessValue::Vm(options)
    } else {
        HarnessValue::Native(options)
    }
}

/// Decode one display mode payload into the shared native shape.
pub(crate) fn decode_display_mode(
    value: HarnessValue<display::DisplayMode, display::DisplayModeVm>,
) -> display::DisplayMode {
    match value {
        HarnessValue::Native(value) => value,
        HarnessValue::Vm(value) => value,
    }
}

/// Build one display mode payload for native and VM binding calls.
pub(crate) fn harness_display_mode(
    context: &DisplayHarnessContext<'_>,
    width: u32,
    height: u32,
    refresh_milli_hz: u32,
    bit_depth: u16,
) -> HarnessValue<display::DisplayMode, display::DisplayModeVm> {
    let mode = display::DisplayMode {
        width,
        height,
        refresh_milli_hz,
        format: 0,
        bit_depth,
    };

    if context.vm_context.is_some() {
        HarnessValue::Vm(mode)
    } else {
        HarnessValue::Native(mode)
    }
}

/// Build one window logical size payload for native and VM binding calls.
pub(crate) fn harness_window_logical_size(
    context: &DisplayHarnessContext<'_>,
    width: f64,
    height: f64,
) -> HarnessValue<display::WindowLogicalSize, display::WindowLogicalSizeVm> {
    let size = display::WindowLogicalSize { width, height };
    if context.vm_context.is_some() {
        HarnessValue::Vm(size)
    } else {
        HarnessValue::Native(size)
    }
}

/// Build one window physical size payload for native and VM binding calls.
pub(crate) fn harness_window_physical_size(
    context: &DisplayHarnessContext<'_>,
    width: u32,
    height: u32,
) -> HarnessValue<display::WindowPhysicalSize, display::WindowPhysicalSizeVm> {
    let size = display::WindowPhysicalSize { width, height };
    if context.vm_context.is_some() {
        HarnessValue::Vm(size)
    } else {
        HarnessValue::Native(size)
    }
}

/// Build one window mode options payload for native and VM binding calls.
pub(crate) fn harness_window_mode_options(
    context: &DisplayHarnessContext<'_>,
    mode: display::WindowMode,
) -> HarnessValue<display::WindowModeOptions, display::WindowModeOptionsVm> {
    let options = display::WindowModeOptions {
        mode,
        display: None,
        display_mode: None,
    };
    if context.vm_context.is_some() {
        HarnessValue::Vm(options)
    } else {
        HarnessValue::Native(options)
    }
}

/// Extract one platform error code from one runtime error.
pub(crate) fn error_code(error: &RuntimeError) -> Option<PlatformErrorCode> {
    error.platform_error().map(|platform| platform.code)
}

/// Decode one harness value where native and vm payloads share one ABI shape.
pub(crate) fn decode_harness_value<T>(value: HarnessValue<T, T>) -> T {
    match value {
        HarnessValue::Native(value) => value,
        HarnessValue::Vm(value) => value,
    }
}
