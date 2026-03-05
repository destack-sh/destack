#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::display::bindings_generated as bindings;
use crate::platform::{NativeArray, PlatformError};
use crate::runtime::{NativeSlice, NativeStringRef};

use crate::runtime::BindingCallContext;
use bindings::*;

use crate::platform::display::{
    DisplayAddedPayload, DisplayBackendCapabilityFlags, DisplayColorState, DisplayDescriptor,
    DisplayDescriptorChangedPayload, DisplayGammaRamp, DisplayHdrMode, DisplayMode,
    DisplayModeChangedPayload, DisplayMonitorEvent, DisplayMonitorEventOpenOptions,
    DisplayMonitorListRequest, DisplayMonitorOpenOptions, DisplayOrientation,
    DisplayPrimaryPayload, DisplayRemovedPayload, WindowAspectRatio, WindowAttentionLevel,
    WindowChromeKind, WindowCursorIcon, WindowCursorMode, WindowDescriptor, WindowDisplayPayload,
    WindowEvent, WindowEventOpenOptions, WindowFocusPayload, WindowIconSet, WindowLogicalSize,
    WindowModeOptions, WindowModePayload, WindowOcclusionPayload, WindowOptions,
    WindowPhysicalSize, WindowPosition, WindowPositionPayload, WindowResizeEdge,
    WindowScaleFactorPayload, WindowSizeConstraints, WindowSizePayload, WindowState, WindowTheme,
    WindowThemePayload, WindowVisibility, WindowVisibilityPayload,
};
use crate::platform::resource;

/// Close one display endpoint.
///
/// Close one opened display endpoint and release host resources.
/// Any outstanding mode-change session state is discarded.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific display close operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `display.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_monitor_close(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.close",
    ))
    .boxed())
}

/// Resolve one requested mode to the closest supported mode.
///
/// Return one backend-selected closest mode for one requested mode.
/// Mode-matching behavior follows host backend selection policy.
///
/// # Platform
/// Unix and Windows.
/// Uses backend mode-matching queries where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `display.mode`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_monitor_closest_mode(
    binding: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
    requested: DisplayMode,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, requested);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.closestMode",
    ))
    .boxed())
}

/// Read the current mode for one opened display.
///
/// Read one point-in-time active mode for one opened display endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific current-mode queries.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `display.mode`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_monitor_current_mode(
    binding: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.currentMode",
    ))
    .boxed())
}

/// Read descriptor metadata for one opened display.
///
/// Read one normalized descriptor snapshot for one opened display endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific monitor metadata queries.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `display.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_monitor_descriptor(
    binding: &BindingCallContext,
    out: *mut DisplayDescriptor,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.descriptor",
    ))
    .boxed())
}

/// Read the desktop-preferred mode for one opened display.
///
/// Read one platform desktop mode for one opened display endpoint.
/// This aligns with SDL desktop-mode and Unreal desktop-resolution semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses backend desktop-mode queries where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `display.mode`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_monitor_desktop_mode(
    binding: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.desktopMode",
    ))
    .boxed())
}

/// Close one global monitor-event stream.
///
/// Close one opened monitor-event stream and release host monitor routing resources.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific event-stream close operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_monitor_event_close(
    binding: &BindingCallContext,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.eventClose",
    ))
    .boxed())
}

/// Open one global monitor-event stream.
///
/// Open one host monitor-event stream for display hotplug and metrics-change routing.
/// Event ordering follows host event-loop delivery behavior.
/// This stream should be consumed from one runtime event-loop thread.
///
/// # Platform
/// Unix and Windows.
/// Uses host monitor callback or message subscriptions similar to GLFW monitor callbacks and SDL display events.
///
/// # Errors
/// Returns ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_monitor_event_open(
    binding: &BindingCallContext,
    out: *mut resource::DisplayEventHandle,
    options: DisplayMonitorEventOpenOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.eventOpen",
    ))
    .boxed())
}

/// Wait for one monitor event.
///
/// Wait for one event from one opened monitor-event stream.
///
/// # Platform
/// Unix and Windows.
/// Uses host event queue wait operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_monitor_event_read(
    binding: &BindingCallContext,
    out: *mut DisplayMonitorEvent,
    handle: resource::DisplayEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.eventRead",
    ))
    .boxed())
}

/// Wait for one batch of monitor events.
///
/// Wait for pending events from one opened monitor-event stream and return up to `maxEvents` events.
///
/// # Platform
/// Unix and Windows.
/// Uses host event queue batch wait operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_monitor_event_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<DisplayMonitorEvent>,
    handle: resource::DisplayEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, maxevents, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.eventReadBatch",
    ))
    .boxed())
}

/// Poll one monitor event without blocking.
///
/// Poll one pending event from one opened monitor-event stream without waiting.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking host event queue polling.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_monitor_event_try_read(
    binding: &BindingCallContext,
    out: *mut DisplayMonitorEvent,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.eventTryRead",
    ))
    .boxed())
}

/// Poll one batch of monitor events without blocking.
///
/// Poll pending events from one opened monitor-event stream and return up to `maxEvents` events.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking host event queue batch polling.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_monitor_event_try_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<DisplayMonitorEvent>,
    handle: resource::DisplayEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, maxevents);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.eventTryReadBatch",
    ))
    .boxed())
}

/// List available displays.
///
/// Enumerate host display outputs and return stable identifiers and physical metadata.
/// Output ordering and hotplug visibility follow host compositor or kernel display APIs.
///
/// # Platform
/// Unix and Windows.
/// Uses DRM or Wayland or X11 display enumeration on Unix-like hosts and DXGI display enumeration on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `display.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_monitor_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<DisplayDescriptor>,
    request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, request);

    Err(RuntimeError::from(PlatformError::not_supported("destack.display.monitor.list")).boxed())
}

/// Read available display modes.
///
/// Read all host-supported modes for one opened display endpoint.
/// Mode list ordering follows host backend reporting behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses drmModeGetConnector or compositor APIs on Unix-like hosts and DXGI mode queries on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `display.mode`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_monitor_modes(
    binding: &BindingCallContext,
    out: *mut NativeSlice<DisplayMode>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.modes",
    ))
    .boxed())
}

/// Open one display endpoint.
///
/// Open one host display endpoint by identifier for mode queries and updates.
/// Endpoint lifetime semantics follow host display management APIs.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific display open handles.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `display.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_monitor_open(
    binding: &BindingCallContext,
    out: *mut resource::DisplayHandle,
    id: NativeStringRef,
    options: DisplayMonitorOpenOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, id, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.display.monitor.open")).boxed())
}

/// Read the current primary display handle.
///
/// Return one opened display handle for the current primary display when available.
/// Returns `void` when the backend has no discoverable primary display.
///
/// # Platform
/// Unix and Windows.
/// Uses compositor and system-display primary-output selection.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `display.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_monitor_primary(
    binding: &BindingCallContext,
    out: *mut Option<resource::DisplayHandle>,
    request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, request);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.primary",
    ))
    .boxed())
}

/// Apply one display mode.
///
/// Apply one mode to an opened display endpoint.
/// Mode-set behavior and rollback semantics are host-defined.
///
/// # Platform
/// Unix and Windows.
/// Uses KMS mode setting on Unix-like hosts and display mode APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.mode`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_display_monitor_set_mode(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayMode,
) -> RuntimeResult<()> {
    let _ = (binding, handle, mode);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.setMode",
    ))
    .boxed())
}

/// Read display color state.
pub(crate) unsafe fn destack_display_monitor_color_state(
    binding: &BindingCallContext,
    out: *mut DisplayColorState,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.colorState",
    ))
    .boxed())
}

/// Read display HDR mode.
pub(crate) unsafe fn destack_display_monitor_hdr_mode(
    binding: &BindingCallContext,
    out: *mut DisplayHdrMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.hdrMode",
    ))
    .boxed())
}

/// Set display HDR mode.
pub(crate) unsafe fn destack_display_monitor_set_hdr_mode(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayHdrMode,
) -> RuntimeResult<()> {
    let _ = (binding, handle, mode);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.setHdrMode",
    ))
    .boxed())
}

/// Read display gamma ramp.
pub(crate) unsafe fn destack_display_monitor_gamma_ramp(
    binding: &BindingCallContext,
    out: *mut DisplayGammaRamp,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.gammaRamp",
    ))
    .boxed())
}

/// Set display gamma ramp.
pub(crate) unsafe fn destack_display_monitor_set_gamma_ramp(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    ramp: DisplayGammaRamp,
) -> RuntimeResult<()> {
    let _ = (binding, handle, ramp);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.setGammaRamp",
    ))
    .boxed())
}

/// Close one window.
///
/// Close one host window and release associated compositor or window-system resources.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific window close and destroy operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_close(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _ = (binding, window);

    Err(RuntimeError::from(PlatformError::not_supported("destack.display.window.close")).boxed())
}

/// Read descriptor metadata for one window.
///
/// Read one normalized descriptor snapshot for one opened host window.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific window metadata queries.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_descriptor(
    binding: &BindingCallContext,
    out: *mut WindowDescriptor,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, window);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.descriptor",
    ))
    .boxed())
}

/// Read one capability mask for one opened window backend.
pub(crate) unsafe fn destack_display_window_capabilities(
    context: &BindingCallContext,
    out: *mut DisplayBackendCapabilityFlags,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, window);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.capabilities",
    ))
    .boxed())
}

/// Close one global window-event stream.
///
/// Close one opened window-event stream and release host event routing resources.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific event-stream close operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window.events`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_event_close(
    binding: &BindingCallContext,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.eventClose",
    ))
    .boxed())
}

/// Open one global window-event stream.
///
/// Open one host window-event stream for all windows in this runtime.
/// Event ordering follows host event-loop delivery behavior.
/// This stream should be consumed from one runtime event-loop thread.
///
/// # Platform
/// Unix and Windows.
/// Uses one host event-loop stream model aligned with winit and SDL style window id routing.
///
/// # Errors
/// Returns ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window.events`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_event_open(
    binding: &BindingCallContext,
    out: *mut resource::WindowEventHandle,
    options: WindowEventOpenOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.eventOpen",
    ))
    .boxed())
}

/// Wait for one window event.
///
/// Wait for one event from one opened host window-event stream.
///
/// # Platform
/// Unix and Windows.
/// Uses host event queue wait operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window.events`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_event_read(
    binding: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.eventRead",
    ))
    .boxed())
}

/// Wait for one batch of window events.
///
/// Wait for pending events from one opened host window-event stream and return up to `maxEvents` events.
///
/// # Platform
/// Unix and Windows.
/// Uses host event queue batch wait operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window.events`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_event_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, maxevents, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.eventReadBatch",
    ))
    .boxed())
}

/// Poll one window event without blocking.
///
/// Poll one pending event from one opened host window-event stream without waiting.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking host event queue polling.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window.events`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_event_try_read(
    binding: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.eventTryRead",
    ))
    .boxed())
}

/// Poll one batch of window events without blocking.
///
/// Poll pending events from one opened host window-event stream and return up to `maxEvents` events.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking host event queue batch polling.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window.events`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_event_try_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, maxevents);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.eventTryReadBatch",
    ))
    .boxed())
}

/// Open one window.
///
/// Create one host window with one explicit window configuration payload.
/// Window lifecycle and compositor integration follow host window-system semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses winit or SDL class host backends over Wayland or X11 or Win32 windowing APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_open(
    binding: &BindingCallContext,
    out: *mut resource::WindowHandle,
    options: WindowOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.display.window.open")).boxed())
}

/// Request user attention for one window.
///
/// Request host-specific user attention signaling for one opened window.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific request-attention primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_request_attention(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    level: WindowAttentionLevel,
) -> RuntimeResult<()> {
    let _ = (binding, window, level);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.requestAttention",
    ))
    .boxed())
}

/// Request one redraw for one window.
///
/// Enqueue one host redraw request for one opened window.
///
/// # Platform
/// Unix and Windows.
/// Uses backend redraw request operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_request_refresh(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _ = (binding, window);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.requestRefresh",
    ))
    .boxed())
}

/// Set always-on-top state.
///
/// Toggle host always-on-top policy for one opened window.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific topmost-window flags.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_set_always_on_top(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    alwaysontop: bool,
) -> RuntimeResult<()> {
    let _ = (binding, window, alwaysontop);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setAlwaysOnTop",
    ))
    .boxed())
}

/// Set cursor icon for one window.
///
/// Apply one standard system cursor icon for one opened window.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific cursor-shape operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_set_cursor_icon(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    icon: WindowCursorIcon,
) -> RuntimeResult<()> {
    let _ = (binding, window, icon);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setCursorIcon",
    ))
    .boxed())
}

/// Set cursor interaction mode for one window.
///
/// Apply one cursor mode policy for one opened window.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific cursor lock and confine and hide operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_set_cursor_mode(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowCursorMode,
) -> RuntimeResult<()> {
    let _ = (binding, window, mode);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setCursorMode",
    ))
    .boxed())
}

/// Set cursor position for one window.
///
/// Warp cursor position relative to one opened window.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific cursor warp operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_set_cursor_position(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    let _ = (binding, window, position);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setCursorPosition",
    ))
    .boxed())
}

/// Set cursor visibility for one window.
///
/// Show or hide one window cursor without changing lock or confinement state.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific cursor visibility operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_set_cursor_visible(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    let _ = (binding, window, visible);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setCursorVisible",
    ))
    .boxed())
}

/// Set window decoration state.
///
/// Toggle host decorations for one opened window.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific decorated-window flags.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_set_decorated(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    let _ = (binding, window, decorated);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setDecorated",
    ))
    .boxed())
}

/// Set one window mode.
///
/// Apply one host window mode transition for one opened window.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific fullscreen and borderless and windowed mode operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_set_mode(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowModeOptions,
) -> RuntimeResult<()> {
    let _ = (binding, window, mode);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setMode",
    ))
    .boxed())
}

/// Set one window aspect ratio lock.
pub(crate) unsafe fn destack_display_window_set_aspect_ratio(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    aspectratio: Option<WindowAspectRatio>,
) -> RuntimeResult<()> {
    let _ = (binding, window, aspectratio);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setAspectRatio",
    ))
    .boxed())
}

/// Set one window chrome kind.
pub(crate) unsafe fn destack_display_window_set_chrome(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    chrome: WindowChromeKind,
) -> RuntimeResult<()> {
    let _ = (binding, window, chrome);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setChrome",
    ))
    .boxed())
}

/// Set one window position.
///
/// Apply one host window position in desktop coordinates.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific window move operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_set_position(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    let _ = (binding, window, position);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setPosition",
    ))
    .boxed())
}

/// Set window resizable state.
///
/// Toggle host resize affordances for one opened window.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific resizable-window flags.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_set_resizable(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    let _ = (binding, window, resizable);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setResizable",
    ))
    .boxed())
}

/// Set one logical window size.
///
/// Apply one host window size in logical platform points.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific window resize operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_set_size_logical(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowLogicalSize,
) -> RuntimeResult<()> {
    let _ = (binding, window, size);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setSize",
    ))
    .boxed())
}

/// Set logical size constraints.
///
/// Apply minimum and maximum logical size constraints for one window.
/// Passing `void` clears current constraints.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific size-constraint operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_set_size_constraints(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    constraints: Option<WindowSizeConstraints>,
) -> RuntimeResult<()> {
    let _ = (binding, window, constraints);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setSizeConstraints",
    ))
    .boxed())
}

/// Set one physical window size.
///
/// Apply one host window size in physical pixels.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific pixel-size resize operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_set_size_physical(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowPhysicalSize,
) -> RuntimeResult<()> {
    let _ = (binding, window, size);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setSizePhysical",
    ))
    .boxed())
}

/// Set one window title string.
///
/// Update one host window title using host window-system APIs.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific title update operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_set_title(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    title: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (binding, window, title);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setTitle",
    ))
    .boxed())
}

/// Set one window icon set.
pub(crate) unsafe fn destack_display_window_set_icons(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    icons: Option<WindowIconSet>,
) -> RuntimeResult<()> {
    let _ = (binding, window, icons);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setIcons",
    ))
    .boxed())
}

/// Set one window modal state.
pub(crate) unsafe fn destack_display_window_set_modal(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    modal: bool,
) -> RuntimeResult<()> {
    let _ = (binding, window, modal);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setModal",
    ))
    .boxed())
}

/// Set one window mouse passthrough state.
pub(crate) unsafe fn destack_display_window_set_mouse_passthrough(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    passthrough: bool,
) -> RuntimeResult<()> {
    let _ = (binding, window, passthrough);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setMousePassthrough",
    ))
    .boxed())
}

/// Set one window opacity.
pub(crate) unsafe fn destack_display_window_set_opacity(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    opacity: f64,
) -> RuntimeResult<()> {
    let _ = (binding, window, opacity);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setOpacity",
    ))
    .boxed())
}

/// Read one window opacity.
pub(crate) unsafe fn destack_display_window_opacity(
    binding: &BindingCallContext,
    out: *mut f64,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, window);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.opacity",
    ))
    .boxed())
}

/// Focus one window.
pub(crate) unsafe fn destack_display_window_focus(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _ = (binding, window);

    Err(RuntimeError::from(PlatformError::not_supported("destack.display.window.focus")).boxed())
}

/// Raise one window.
pub(crate) unsafe fn destack_display_window_raise(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _ = (binding, window);

    Err(RuntimeError::from(PlatformError::not_supported("destack.display.window.raise")).boxed())
}

/// Minimize one window.
pub(crate) unsafe fn destack_display_window_minimize(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _ = (binding, window);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.minimize",
    ))
    .boxed())
}

/// Maximize one window.
pub(crate) unsafe fn destack_display_window_maximize(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _ = (binding, window);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.maximize",
    ))
    .boxed())
}

/// Restore one window.
pub(crate) unsafe fn destack_display_window_restore(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _ = (binding, window);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.restore",
    ))
    .boxed())
}

/// Set one window parent relationship.
pub(crate) unsafe fn destack_display_window_set_parent(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    parent: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    let _ = (binding, window, parent);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setParent",
    ))
    .boxed())
}

/// Set one window transient relationship.
pub(crate) unsafe fn destack_display_window_set_transient_for(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    transientfor: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    let _ = (binding, window, transientfor);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setTransientFor",
    ))
    .boxed())
}

/// Set one window taskbar visibility.
pub(crate) unsafe fn destack_display_window_set_taskbar_visible(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    let _ = (binding, window, visible);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setTaskbarVisible",
    ))
    .boxed())
}

/// Begin one native move-drag interaction.
pub(crate) unsafe fn destack_display_window_begin_move_drag(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _ = (binding, window);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.beginMoveDrag",
    ))
    .boxed())
}

/// Begin one native resize-drag interaction.
pub(crate) unsafe fn destack_display_window_begin_resize_drag(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    edge: WindowResizeEdge,
) -> RuntimeResult<()> {
    let _ = (binding, window, edge);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.beginResizeDrag",
    ))
    .boxed())
}

/// Set one window visibility state.
///
/// Apply one window visibility state transition.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific show and hide and minimize and maximize operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_set_visibility(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    visibility: WindowVisibility,
) -> RuntimeResult<()> {
    let _ = (binding, window, visibility);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setVisibility",
    ))
    .boxed())
}

/// Read one window state snapshot.
///
/// Read one point-in-time host window state snapshot.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific window state queries.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_display_window_state(
    binding: &BindingCallContext,
    out: *mut WindowState,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, window);

    Err(RuntimeError::from(PlatformError::not_supported("destack.display.window.state")).boxed())
}
