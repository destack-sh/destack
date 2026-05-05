use destack_vm as vm;
#[cfg(any(windows, target_os = "macos"))]
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::{PlatformError, VmSlice, display, resource};
use crate::runtime::BindingCallContext;
pub(crate) use crate::tests::execution::run_execution_case_or_return;
pub(crate) use crate::tests::platform::{
    error_code_from_runtime_error as error_code, is_not_supported_code,
    result_or_skip_not_supported,
};
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
    pub(crate) fn with_context<F, R>(&mut self, callback: F) -> R
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
                        let vm_context = vm_context as *mut vm::BindingContext<'_> as *mut ();
                        callback(DisplayHarnessContext {
                            call_context,
                            vm_context: Some(vm_context),
                        })
                    })
            }
        }
    }

    /// Run one callback that returns a runtime result.
    pub(crate) fn run<F>(&mut self, callback: F)
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
    F: FnMut(&mut DisplayHarnessHandle),
{
    let mut native = DisplayHarnessHandle::Native(NativeDisplayHarness::new());
    callback(&mut native);
    let mut vm = DisplayHarnessHandle::Vm(VmDisplayHarness::new());
    callback(&mut vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(DisplayHarnessContext<'call>) -> RuntimeResult<()>,
{
    #[cfg(any(windows, target_os = "macos"))]
    let _guard = {
        let global_lock = display_test_lock();
        global_lock
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    };

    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}

#[cfg(any(windows, target_os = "macos"))]
/// Return one process-global serialization lock for display tests.
fn display_test_lock() -> &'static Mutex<()> {
    static DISPLAY_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    DISPLAY_TEST_LOCK.get_or_init(|| Mutex::new(()))
}

/// Build one harness string payload for native and VM binding calls.
pub(crate) fn harness_string(
    context: &mut DisplayHarnessContext<'_>,
    value: &str,
) -> RuntimeResult<HarnessValue<NativeStringRef, vm::StringHandle>> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut vm::BindingContext<'_>) };
            Ok(HarnessValue::Vm(vm::StringHandle::new(
                vm_context
                    .intern_string(value)
                    .expect("vm test string should intern"),
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
            let vm_context = unsafe { &mut *(vm_context as *mut vm::BindingContext<'_>) };
            let values = values.read_values(&vm_context.read())?;
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
            let vm_context = unsafe { &mut *(vm_context as *mut vm::BindingContext<'_>) };
            Ok(values.read_values(&vm_context.read())?)
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
            let vm_context = unsafe { &mut *(vm_context as *mut vm::BindingContext<'_>) };
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
) -> RuntimeResult<(
    display::DisplayOrientation,
    display::DisplaySupportStatus,
    display::DisplaySupportStatus,
    display::DisplaySupportStatus,
    u32,
    u32,
)> {
    match value {
        HarnessValue::Native(value) => Ok((
            value.orientation,
            value.builtin_panel,
            value.variable_refresh_support,
            value.hdr_support,
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
            let _vm_context = unsafe { &mut *(vm_context as *mut vm::BindingContext<'_>) };
            Ok((
                value.orientation,
                value.builtin_panel,
                value.variable_refresh_support,
                value.hdr_support,
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
            let vm_context = unsafe { &mut *(vm_context as *mut vm::BindingContext<'_>) };
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
            let vm_context = unsafe { &mut *(vm_context as *mut vm::BindingContext<'_>) };
            HarnessValue::Vm(display::WindowOptionsVm {
                backend: display::DisplayBackend::Auto,
                backend_policy: display::DisplayBackendSelectionPolicy::AllowFallback,
                title: vm::StringHandle::new(
                    vm_context
                        .intern_string(title)
                        .expect("vm test string should intern"),
                ),
                role: display::WindowRole::Toplevel,
                size_logical: display::WindowLogicalSizeVm {
                    width: 1280.0,
                    height: 720.0,
                },
                position: Some(display::WindowPositionVm { x: 40, y: 50 }),
                constraints: None,
                display: None,
                mode: display::WindowModeOptionsVm::WindowWindowedModeOptions(
                    display::WindowWindowedModeOptionsVm {
                        kind: vm::StringHandle::new(
                            vm_context
                                .intern_string("windowed")
                                .expect("vm test string should intern"),
                        ),
                    },
                ),
                visibility: display::WindowVisibility::Visible,
                resizable: true,
                decorated: true,
                transparent: false,
                chrome: display::WindowChromeKind::Standard,
                taskbar_visible: true,
                opacity: None,
                focus_on_show: true,
                always_on_top: false,
                parent: None,
                transient_for: None,
                modal: None,
                mouse_passthrough: None,
                aspect_ratio: None,
            })
        }
        None => HarnessValue::Native(display::WindowOptions {
            backend: display::DisplayBackend::Auto,
            backend_policy: display::DisplayBackendSelectionPolicy::AllowFallback,
            title: context.call_context.store_string(title),
            role: display::WindowRole::Toplevel,
            size_logical: display::WindowLogicalSize {
                width: 1280.0,
                height: 720.0,
            },
            position: Some(display::WindowPosition { x: 40, y: 50 }),
            constraints: None,
            display: None,
            mode: display::WindowModeOptions::WindowWindowedModeOptions(
                display::WindowWindowedModeOptions {
                    kind: context.call_context.store_string("windowed"),
                },
            ),
            visibility: display::WindowVisibility::Visible,
            resizable: true,
            decorated: true,
            transparent: false,
            chrome: display::WindowChromeKind::Standard,
            taskbar_visible: true,
            opacity: None,
            focus_on_show: true,
            always_on_top: false,
            parent: None,
            transient_for: None,
            modal: None,
            mouse_passthrough: None,
            aspect_ratio: None,
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
    if context.vm_context.is_some() {
        HarnessValue::Vm(display::DisplayMonitorEventOpenOptionsVm {
            backend: display::DisplayBackend::Auto,
            backend_policy: display::DisplayBackendSelectionPolicy::AllowFallback,
            queue: display::DisplayEventQueueOptions {
                queue_capacity: 256,
                overflow_policy: display::DisplayEventOverflowPolicy::DropOldest,
            },
            filter: None,
        })
    } else {
        HarnessValue::Native(display::DisplayMonitorEventOpenOptions {
            backend: display::DisplayBackend::Auto,
            backend_policy: display::DisplayBackendSelectionPolicy::AllowFallback,
            queue: display::DisplayEventQueueOptions {
                queue_capacity: 256,
                overflow_policy: display::DisplayEventOverflowPolicy::DropOldest,
            },
            filter: None,
        })
    }
}

/// Build one monitor-event open options payload for custom queue behavior.
pub(crate) fn monitor_event_open_options(
    context: &DisplayHarnessContext<'_>,
    queue_capacity: u32,
    overflow_policy: display::DisplayEventOverflowPolicy,
) -> HarnessValue<display::DisplayMonitorEventOpenOptions, display::DisplayMonitorEventOpenOptionsVm>
{
    if context.vm_context.is_some() {
        HarnessValue::Vm(display::DisplayMonitorEventOpenOptionsVm {
            backend: display::DisplayBackend::Auto,
            backend_policy: display::DisplayBackendSelectionPolicy::AllowFallback,
            queue: display::DisplayEventQueueOptions {
                queue_capacity,
                overflow_policy,
            },
            filter: None,
        })
    } else {
        HarnessValue::Native(display::DisplayMonitorEventOpenOptions {
            backend: display::DisplayBackend::Auto,
            backend_policy: display::DisplayBackendSelectionPolicy::AllowFallback,
            queue: display::DisplayEventQueueOptions {
                queue_capacity,
                overflow_policy,
            },
            filter: None,
        })
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
        filter: None,
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
        filter: None,
    };

    if context.vm_context.is_some() {
        HarnessValue::Vm(options)
    } else {
        HarnessValue::Native(options)
    }
}

/// Build one monitor-event open options payload with one explicit kind-mask filter.
pub(crate) fn monitor_event_open_options_with_kind_mask(
    context: &DisplayHarnessContext<'_>,
    queue_capacity: u32,
    overflow_policy: display::DisplayEventOverflowPolicy,
    kind_mask: u32,
) -> HarnessValue<display::DisplayMonitorEventOpenOptions, display::DisplayMonitorEventOpenOptionsVm>
{
    if context.vm_context.is_some() {
        HarnessValue::Vm(display::DisplayMonitorEventOpenOptionsVm {
            backend: display::DisplayBackend::Auto,
            backend_policy: display::DisplayBackendSelectionPolicy::AllowFallback,
            queue: display::DisplayEventQueueOptions {
                queue_capacity,
                overflow_policy,
            },
            filter: Some(display::DisplayMonitorEventFilterVm {
                display_id: None,
                kind_mask: Some(display::DisplayMonitorEventKindMask(kind_mask)),
            }),
        })
    } else {
        HarnessValue::Native(display::DisplayMonitorEventOpenOptions {
            backend: display::DisplayBackend::Auto,
            backend_policy: display::DisplayBackendSelectionPolicy::AllowFallback,
            queue: display::DisplayEventQueueOptions {
                queue_capacity,
                overflow_policy,
            },
            filter: Some(display::DisplayMonitorEventFilter {
                display_id: None,
                kind_mask: Some(display::DisplayMonitorEventKindMask(kind_mask)),
            }),
        })
    }
}

/// Build one window-event open options payload with explicit filter restrictions.
pub(crate) fn window_event_open_options_with_filter(
    context: &DisplayHarnessContext<'_>,
    queue_capacity: u32,
    overflow_policy: display::DisplayEventOverflowPolicy,
    window: Option<resource::WindowHandle>,
    kind_mask: Option<u64>,
) -> HarnessValue<display::WindowEventOpenOptions, display::WindowEventOpenOptionsVm> {
    let options = display::WindowEventOpenOptions {
        backend: display::DisplayBackend::Auto,
        backend_policy: display::DisplayBackendSelectionPolicy::AllowFallback,
        queue: display::DisplayEventQueueOptions {
            queue_capacity,
            overflow_policy,
        },
        filter: Some(display::WindowEventFilter {
            window,
            kind_mask: kind_mask.map(display::WindowEventKindMask),
        }),
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
        format: display::DisplayPixelFormat(0),
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

/// Build one window position payload for native and VM binding calls.
pub(crate) fn harness_window_position(
    context: &DisplayHarnessContext<'_>,
    x: i32,
    y: i32,
) -> HarnessValue<display::WindowPosition, display::WindowPositionVm> {
    let position = display::WindowPosition { x, y };
    if context.vm_context.is_some() {
        HarnessValue::Vm(position)
    } else {
        HarnessValue::Native(position)
    }
}

/// Build one window size-constraints payload for native and VM binding calls.
pub(crate) fn harness_window_size_constraints(
    context: &DisplayHarnessContext<'_>,
    min_width: f64,
    min_height: f64,
    max_width: f64,
    max_height: f64,
) -> HarnessValue<Option<display::WindowSizeConstraints>, Option<display::WindowSizeConstraintsVm>>
{
    let constraints = Some(display::WindowSizeConstraints {
        min: Some(display::WindowLogicalSize {
            width: min_width,
            height: min_height,
        }),
        max: Some(display::WindowLogicalSize {
            width: max_width,
            height: max_height,
        }),
    });

    if context.vm_context.is_some() {
        HarnessValue::Vm(constraints)
    } else {
        HarnessValue::Native(constraints)
    }
}

/// Build one cleared window size-constraints payload for native and VM binding calls.
pub(crate) fn harness_window_size_constraints_none(
    context: &DisplayHarnessContext<'_>,
) -> HarnessValue<Option<display::WindowSizeConstraints>, Option<display::WindowSizeConstraintsVm>>
{
    if context.vm_context.is_some() {
        HarnessValue::Vm(None)
    } else {
        HarnessValue::Native(None)
    }
}

/// Build one window icon-set payload for native and VM binding calls.
pub(crate) fn harness_window_icon_set(
    context: &mut DisplayHarnessContext<'_>,
) -> RuntimeResult<HarnessValue<Option<display::WindowIconSet>, Option<display::WindowIconSetVm>>> {
    let pixels = vec![
        0xFF, 0x00, 0x00, 0xFF, //
        0x00, 0xFF, 0x00, 0xFF, //
        0x00, 0x00, 0xFF, 0xFF, //
        0xFF, 0xFF, 0xFF, 0xFF, //
    ];

    if let Some(vm_context) = context.vm_context {
        let vm_context = unsafe { &mut *(vm_context as *mut vm::BindingContext<'_>) };
        let pixels_vm = VmSlice::from_bytes(&mut vm_context.write(), &pixels)
            .expect("vm test byte slice should allocate");
        let image_vm = display::WindowIconImageVm {
            width: 2,
            height: 2,
            pixel_format: display::WindowIconPixelFormat::Rgba8,
            pixels: pixels_vm,
        };
        let images_vm = VmSlice::from_values(&mut vm_context.write(), &[image_vm])?;
        let icon_set_vm = display::WindowIconSetVm { images: images_vm };

        return Ok(HarnessValue::Vm(Some(icon_set_vm)));
    }

    let pixels_native = context.call_context.store_slice(pixels);
    let image_native = display::WindowIconImage {
        width: 2,
        height: 2,
        pixel_format: display::WindowIconPixelFormat::Rgba8,
        pixels: pixels_native,
    };
    let images_native = context.call_context.store_slice(vec![image_native]);
    let icon_set_native = display::WindowIconSet {
        images: images_native,
    };
    Ok(HarnessValue::Native(Some(icon_set_native)))
}

/// Build one empty icon-set payload for native and VM binding calls.
pub(crate) fn harness_window_icon_set_none(
    context: &DisplayHarnessContext<'_>,
) -> HarnessValue<Option<display::WindowIconSet>, Option<display::WindowIconSetVm>> {
    if context.vm_context.is_some() {
        HarnessValue::Vm(None)
    } else {
        HarnessValue::Native(None)
    }
}

/// Window mode helper for harness payload construction.
pub(crate) enum HarnessWindowMode {
    /// Windowed mode.
    Windowed,
    /// Borderless mode.
    #[allow(dead_code)]
    Borderless,
    /// Exclusive fullscreen mode.
    ExclusiveFullscreen {
        /// Target display handle.
        display: resource::DisplayHandle,
        /// Optional preferred display mode.
        display_mode: Option<display::DisplayMode>,
    },
}

/// Build one window mode options payload for native and VM binding calls.
pub(crate) fn harness_window_mode_options(
    context: &DisplayHarnessContext<'_>,
    mode: HarnessWindowMode,
) -> HarnessValue<display::WindowModeOptions, display::WindowModeOptionsVm> {
    if let Some(vm_context) = context.vm_context {
        let vm_context = unsafe { &mut *(vm_context as *mut vm::BindingContext<'_>) };
        match mode {
            HarnessWindowMode::Windowed => {
                HarnessValue::Vm(display::WindowModeOptionsVm::WindowWindowedModeOptions(
                    display::WindowWindowedModeOptionsVm {
                        kind: vm::StringHandle::new(
                            vm_context
                                .intern_string("windowed")
                                .expect("vm test string should intern"),
                        ),
                    },
                ))
            }
            HarnessWindowMode::Borderless => {
                HarnessValue::Vm(display::WindowModeOptionsVm::WindowBorderlessModeOptions(
                    display::WindowBorderlessModeOptionsVm {
                        kind: vm::StringHandle::new(
                            vm_context
                                .intern_string("borderless")
                                .expect("vm test string should intern"),
                        ),
                        display: None,
                    },
                ))
            }
            HarnessWindowMode::ExclusiveFullscreen {
                display,
                display_mode,
            } => HarnessValue::Vm(
                display::WindowModeOptionsVm::WindowExclusiveFullscreenModeOptions(
                    display::WindowExclusiveFullscreenModeOptionsVm {
                        kind: vm::StringHandle::new(
                            vm_context
                                .intern_string("exclusiveFullscreen")
                                .expect("vm test string should intern"),
                        ),
                        display,
                        display_mode,
                    },
                ),
            ),
        }
    } else {
        match mode {
            HarnessWindowMode::Windowed => {
                HarnessValue::Native(display::WindowModeOptions::WindowWindowedModeOptions(
                    display::WindowWindowedModeOptions {
                        kind: context.call_context.store_string("windowed"),
                    },
                ))
            }
            HarnessWindowMode::Borderless => {
                HarnessValue::Native(display::WindowModeOptions::WindowBorderlessModeOptions(
                    display::WindowBorderlessModeOptions {
                        kind: context.call_context.store_string("borderless"),
                        display: None,
                    },
                ))
            }
            HarnessWindowMode::ExclusiveFullscreen {
                display,
                display_mode,
            } => HarnessValue::Native(
                display::WindowModeOptions::WindowExclusiveFullscreenModeOptions(
                    display::WindowExclusiveFullscreenModeOptions {
                        kind: context.call_context.store_string("exclusiveFullscreen"),
                        display,
                        display_mode,
                    },
                ),
            ),
        }
    }
}

/// Open one window and map not-supported errors to none.
pub(crate) fn open_window_or_skip_not_supported(
    context: &mut DisplayHarnessContext<'_>,
    options: HarnessValue<display::WindowOptions, display::WindowOptionsVm>,
) -> RuntimeResult<Option<resource::WindowHandle>> {
    result_or_skip_not_supported(context.destack_display_window_open(options))
}

/// Decode one harness value where native and vm payloads share one ABI shape.
pub(crate) fn decode_harness_value<T>(value: HarnessValue<T, T>) -> T {
    match value {
        HarnessValue::Native(value) => value,
        HarnessValue::Vm(value) => value,
    }
}

/// Wait until one window state confirms the requested visibility lane.
pub(crate) fn wait_window_visibility(
    context: &mut DisplayHarnessContext<'_>,
    window: resource::WindowHandle,
    expected_visibility: display::WindowVisibility,
) -> RuntimeResult<bool> {
    const VISIBILITY_POLL_ATTEMPTS: usize = 32;
    const VISIBILITY_POLL_INTERVAL_MS: u64 = 10;

    // poll the current snapshot until the backend confirms the requested lane
    for _ in 0..VISIBILITY_POLL_ATTEMPTS {
        let state = context.destack_display_window_state(window)?;
        let state = decode_harness_value(state);
        if state.visibility == expected_visibility {
            return Ok(true);
        }

        thread::sleep(Duration::from_millis(VISIBILITY_POLL_INTERVAL_MS));
    }

    Ok(false)
}
