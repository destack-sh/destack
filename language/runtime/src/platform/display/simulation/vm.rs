#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::display::{
    DisplayBackendCapabilityFlags, DisplayBackendDescriptorVm, DisplayColorState,
    DisplayDescriptorVm, DisplayDragBeginOptionsVm, DisplayDragOperation, DisplayGammaRampVm,
    DisplayHdrMode, DisplayModeVm, DisplayMonitorEventOpenOptionsVm, DisplayMonitorEventVm,
    DisplayMonitorListRequestVm, DisplayMonitorOpenOptionsVm, WindowAspectRatio,
    WindowAttentionLevel, WindowChromeKind, WindowCursorIcon, WindowCursorMode, WindowDescriptorVm,
    WindowEventOpenOptionsVm, WindowEventVm, WindowIconSetVm, WindowLogicalSizeVm,
    WindowModeOptionsVm, WindowOptionsVm, WindowPhysicalSizeVm, WindowPositionVm, WindowResizeEdge,
    WindowSizeConstraintsVm, WindowStateVm, WindowVisibility,
};
use crate::platform::fs::OsPathVm;
use crate::platform::{PlatformError, VmArray, VmSlice, resource};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// List display backends that are available for the active target.
pub(crate) fn destack_display_backend_list(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<DisplayBackendDescriptorVm>> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.display.backend.list")).boxed())
}

/// Begin one drag session.
pub(crate) fn destack_display_drag_begin(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    options: DisplayDragBeginOptionsVm,
) -> RuntimeResult<DisplayDragOperation> {
    let _ = options;
    Err(RuntimeError::from(PlatformError::not_supported("destack.display.drag.begin")).boxed())
}

/// Set one drag session operation.
pub(crate) fn destack_display_drag_session_set_operation(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    session: resource::DisplayDragSessionHandle,
    operation: DisplayDragOperation,
) -> RuntimeResult<()> {
    let _ = (session, operation);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.drag.sessionSetOperation",
    ))
    .boxed())
}

/// Close one drag session.
pub(crate) fn destack_display_drag_session_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    session: resource::DisplayDragSessionHandle,
) -> RuntimeResult<()> {
    let _ = session;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.drag.sessionClose",
    ))
    .boxed())
}

/// Read one drag session item as bytes.
pub(crate) fn destack_display_drag_session_read_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    session: resource::DisplayDragSessionHandle,
    itemindex: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (session, itemindex);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.drag.sessionReadBytes",
    ))
    .boxed())
}

/// Read one drag session item as one path.
pub(crate) fn destack_display_drag_session_read_path(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    session: resource::DisplayDragSessionHandle,
    itemindex: u32,
) -> RuntimeResult<OsPathVm> {
    let _ = (session, itemindex);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.drag.sessionReadPath",
    ))
    .boxed())
}

/// Read one drag session item as text.
pub(crate) fn destack_display_drag_session_read_text(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    session: resource::DisplayDragSessionHandle,
    itemindex: u32,
) -> RuntimeResult<vm::StringHandle> {
    let _ = (session, itemindex);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.drag.sessionReadText",
    ))
    .boxed())
}

pub(crate) use crate::platform::display::vm::{
    destack_display_begin_frame_close, destack_display_begin_frame_open,
    destack_display_begin_frame_read, destack_display_begin_frame_read_batch,
    destack_display_begin_frame_try_read, destack_display_begin_frame_try_read_batch,
    destack_display_window_content_rect, destack_display_window_framebuffer_size,
    destack_display_window_invalidate, destack_display_window_render_state,
};

/// Close one display endpoint.
pub(crate) fn destack_display_monitor_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.close",
    ))
    .boxed())
}

/// Resolve one requested mode to the closest supported mode.
pub(crate) fn destack_display_monitor_closest_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
    requested: DisplayModeVm,
) -> RuntimeResult<DisplayModeVm> {
    let _ = (handle, requested);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.closestMode",
    ))
    .boxed())
}

/// Read the current mode for one opened display.
pub(crate) fn destack_display_monitor_current_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<DisplayModeVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.currentMode",
    ))
    .boxed())
}

/// Read descriptor metadata for one opened display.
pub(crate) fn destack_display_monitor_descriptor(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<DisplayDescriptorVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.descriptor",
    ))
    .boxed())
}

/// Read the desktop-preferred mode for one opened display.
pub(crate) fn destack_display_monitor_desktop_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<DisplayModeVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.desktopMode",
    ))
    .boxed())
}

/// Close one global monitor-event stream.
pub(crate) fn destack_display_monitor_event_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.eventClose",
    ))
    .boxed())
}

/// Open one global monitor-event stream.
pub(crate) fn destack_display_monitor_event_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _options: DisplayMonitorEventOpenOptionsVm,
) -> RuntimeResult<resource::DisplayEventHandle> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.eventOpen",
    ))
    .boxed())
}

/// Wait for one monitor event.
pub(crate) fn destack_display_monitor_event_read(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::DisplayEventHandle,
    timeoutns: u64,
) -> RuntimeResult<DisplayMonitorEventVm> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.eventRead",
    ))
    .boxed())
}

/// Wait for one batch of monitor events.
pub(crate) fn destack_display_monitor_event_read_batch(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::DisplayEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<VmArray<DisplayMonitorEventVm>> {
    let _ = (handle, maxevents, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.eventReadBatch",
    ))
    .boxed())
}

/// Poll one monitor event without blocking.
pub(crate) fn destack_display_monitor_event_try_read(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<DisplayMonitorEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.eventTryRead",
    ))
    .boxed())
}

/// Poll one batch of monitor events without blocking.
pub(crate) fn destack_display_monitor_event_try_read_batch(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::DisplayEventHandle,
    maxevents: u32,
) -> RuntimeResult<VmArray<DisplayMonitorEventVm>> {
    let _ = (handle, maxevents);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.eventTryReadBatch",
    ))
    .boxed())
}

/// List available displays.
pub(crate) fn destack_display_monitor_list(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _request: DisplayMonitorListRequestVm,
) -> RuntimeResult<VmSlice<DisplayDescriptorVm>> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.display.monitor.list")).boxed())
}

/// Read available display modes.
pub(crate) fn destack_display_monitor_modes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<VmSlice<DisplayModeVm>> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.modes",
    ))
    .boxed())
}

/// Open one display endpoint.
pub(crate) fn destack_display_monitor_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    id: vm::StringHandle,
    _options: DisplayMonitorOpenOptionsVm,
) -> RuntimeResult<resource::DisplayHandle> {
    let _ = id;
    Err(RuntimeError::from(PlatformError::not_supported("destack.display.monitor.open")).boxed())
}

/// Read the current primary display handle.
pub(crate) fn destack_display_monitor_primary(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _request: DisplayMonitorListRequestVm,
) -> RuntimeResult<Option<resource::DisplayHandle>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.primary",
    ))
    .boxed())
}

/// Apply one display mode.
pub(crate) fn destack_display_monitor_set_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
    mode: DisplayModeVm,
) -> RuntimeResult<()> {
    let _ = (handle, mode);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.setMode",
    ))
    .boxed())
}

/// Close one window.
pub(crate) fn destack_display_window_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported("destack.display.window.close")).boxed())
}

/// Read descriptor metadata for one window.
pub(crate) fn destack_display_window_descriptor(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<WindowDescriptorVm> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.descriptor",
    ))
    .boxed())
}

/// Read one capability mask for one opened window backend.
pub(crate) fn destack_display_window_capabilities(
    _runtime: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<DisplayBackendCapabilityFlags> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.capabilities",
    ))
    .boxed())
}

/// Close one global window-event stream.
pub(crate) fn destack_display_window_event_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.eventClose",
    ))
    .boxed())
}

/// Open one global window-event stream.
pub(crate) fn destack_display_window_event_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _options: WindowEventOpenOptionsVm,
) -> RuntimeResult<resource::WindowEventHandle> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.eventOpen",
    ))
    .boxed())
}

/// Wait for one window event.
pub(crate) fn destack_display_window_event_read(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::WindowEventHandle,
    timeoutns: u64,
) -> RuntimeResult<WindowEventVm> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.eventRead",
    ))
    .boxed())
}

/// Wait for one batch of window events.
pub(crate) fn destack_display_window_event_read_batch(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<VmArray<WindowEventVm>> {
    let _ = (handle, maxevents, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.eventReadBatch",
    ))
    .boxed())
}

/// Poll one window event without blocking.
pub(crate) fn destack_display_window_event_try_read(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<WindowEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.eventTryRead",
    ))
    .boxed())
}

/// Poll one batch of window events without blocking.
pub(crate) fn destack_display_window_event_try_read_batch(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
) -> RuntimeResult<VmArray<WindowEventVm>> {
    let _ = (handle, maxevents);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.eventTryReadBatch",
    ))
    .boxed())
}

/// Open one window.
pub(crate) fn destack_display_window_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    options: WindowOptionsVm,
) -> RuntimeResult<resource::WindowHandle> {
    let _ = options;
    Err(RuntimeError::from(PlatformError::not_supported("destack.display.window.open")).boxed())
}

/// Request user attention for one window.
pub(crate) fn destack_display_window_request_attention(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    level: WindowAttentionLevel,
) -> RuntimeResult<()> {
    let _ = (window, level);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.requestAttention",
    ))
    .boxed())
}

/// Request one redraw for one window.
pub(crate) fn destack_display_window_request_refresh(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.requestRefresh",
    ))
    .boxed())
}

/// Set always-on-top state.
pub(crate) fn destack_display_window_set_always_on_top(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    alwaysontop: bool,
) -> RuntimeResult<()> {
    let _ = (window, alwaysontop);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setAlwaysOnTop",
    ))
    .boxed())
}

/// Set cursor icon for one window.
pub(crate) fn destack_display_window_set_cursor_icon(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    icon: WindowCursorIcon,
) -> RuntimeResult<()> {
    let _ = (window, icon);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setCursorIcon",
    ))
    .boxed())
}

/// Set cursor interaction mode for one window.
pub(crate) fn destack_display_window_set_cursor_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    mode: WindowCursorMode,
) -> RuntimeResult<()> {
    let _ = (window, mode);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setCursorMode",
    ))
    .boxed())
}

/// Set cursor position for one window.
pub(crate) fn destack_display_window_set_cursor_position(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    position: WindowPositionVm,
) -> RuntimeResult<()> {
    let _ = (window, position);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setCursorPosition",
    ))
    .boxed())
}

/// Set cursor visibility for one window.
pub(crate) fn destack_display_window_set_cursor_visible(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    let _ = (window, visible);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setCursorVisible",
    ))
    .boxed())
}

/// Set window decoration state.
pub(crate) fn destack_display_window_set_decorated(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    let _ = (window, decorated);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setDecorated",
    ))
    .boxed())
}

/// Set one window mode.
pub(crate) fn destack_display_window_set_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    mode: WindowModeOptionsVm,
) -> RuntimeResult<()> {
    let _ = (window, mode);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setMode",
    ))
    .boxed())
}

/// Set one window position.
pub(crate) fn destack_display_window_set_position(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    position: WindowPositionVm,
) -> RuntimeResult<()> {
    let _ = (window, position);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setPosition",
    ))
    .boxed())
}

/// Set window resizable state.
pub(crate) fn destack_display_window_set_resizable(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    let _ = (window, resizable);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setResizable",
    ))
    .boxed())
}

/// Set one logical window size.
pub(crate) fn destack_display_window_set_size_logical(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    size: WindowLogicalSizeVm,
) -> RuntimeResult<()> {
    let _ = (window, size);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setSize",
    ))
    .boxed())
}

/// Set logical size constraints.
pub(crate) fn destack_display_window_set_size_constraints(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    constraints: Option<WindowSizeConstraintsVm>,
) -> RuntimeResult<()> {
    let _ = (window, constraints);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setSizeConstraints",
    ))
    .boxed())
}

/// Set one physical window size.
pub(crate) fn destack_display_window_set_size_physical(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    size: WindowPhysicalSizeVm,
) -> RuntimeResult<()> {
    let _ = (window, size);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setSizePhysical",
    ))
    .boxed())
}

/// Set one window title string.
pub(crate) fn destack_display_window_set_title(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    title: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (window, title);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setTitle",
    ))
    .boxed())
}

/// Set one window visibility state.
pub(crate) fn destack_display_window_set_visibility(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    visibility: WindowVisibility,
) -> RuntimeResult<()> {
    let _ = (window, visibility);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setVisibility",
    ))
    .boxed())
}

/// Read one window state snapshot.
pub(crate) fn destack_display_window_state(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<WindowStateVm> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported("destack.display.window.state")).boxed())
}

/// Read display color state.
pub(crate) fn destack_display_monitor_color_state(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<DisplayColorState> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.colorState",
    ))
    .boxed())
}

/// Read display gamma ramp.
pub(crate) fn destack_display_monitor_gamma_ramp(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<DisplayGammaRampVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.gammaRamp",
    ))
    .boxed())
}

/// Read display HDR mode.
pub(crate) fn destack_display_monitor_hdr_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<DisplayHdrMode> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.hdrMode",
    ))
    .boxed())
}

/// Set display gamma ramp.
pub(crate) fn destack_display_monitor_set_gamma_ramp(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
    ramp: DisplayGammaRampVm,
) -> RuntimeResult<()> {
    let _ = (handle, ramp);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.setGammaRamp",
    ))
    .boxed())
}

/// Set display HDR mode.
pub(crate) fn destack_display_monitor_set_hdr_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::DisplayHandle,
    mode: DisplayHdrMode,
) -> RuntimeResult<()> {
    let _ = (handle, mode);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.setHdrMode",
    ))
    .boxed())
}

/// Begin one native move drag interaction.
pub(crate) fn destack_display_window_begin_move_drag(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.beginMoveDrag",
    ))
    .boxed())
}

/// Begin one native resize drag interaction.
pub(crate) fn destack_display_window_begin_resize_drag(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    edge: WindowResizeEdge,
) -> RuntimeResult<()> {
    let _ = (window, edge);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.beginResizeDrag",
    ))
    .boxed())
}

/// Focus one window.
pub(crate) fn destack_display_window_focus(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported("destack.display.window.focus")).boxed())
}

/// Maximize one window.
pub(crate) fn destack_display_window_maximize(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.maximize",
    ))
    .boxed())
}

/// Minimize one window.
pub(crate) fn destack_display_window_minimize(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.minimize",
    ))
    .boxed())
}

/// Read one window opacity.
pub(crate) fn destack_display_window_opacity(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<f64> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.opacity",
    ))
    .boxed())
}

/// Raise one window.
pub(crate) fn destack_display_window_raise(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported("destack.display.window.raise")).boxed())
}

/// Restore one window.
pub(crate) fn destack_display_window_restore(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.restore",
    ))
    .boxed())
}

/// Set one window aspect-ratio lock.
pub(crate) fn destack_display_window_set_aspect_ratio(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    aspectratio: Option<WindowAspectRatio>,
) -> RuntimeResult<()> {
    let _ = (window, aspectratio);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setAspectRatio",
    ))
    .boxed())
}

/// Set one window chrome kind.
pub(crate) fn destack_display_window_set_chrome(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    chrome: WindowChromeKind,
) -> RuntimeResult<()> {
    let _ = (window, chrome);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setChrome",
    ))
    .boxed())
}

/// Set one window icon set.
pub(crate) fn destack_display_window_set_icons(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    icons: Option<WindowIconSetVm>,
) -> RuntimeResult<()> {
    let _ = (window, icons);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setIcons",
    ))
    .boxed())
}

/// Set one window modal state.
pub(crate) fn destack_display_window_set_modal(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    modal: bool,
) -> RuntimeResult<()> {
    let _ = (window, modal);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setModal",
    ))
    .boxed())
}

/// Set one window mouse-passthrough state.
pub(crate) fn destack_display_window_set_mouse_passthrough(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    passthrough: bool,
) -> RuntimeResult<()> {
    let _ = (window, passthrough);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setMousePassthrough",
    ))
    .boxed())
}

/// Set one window opacity.
pub(crate) fn destack_display_window_set_opacity(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    opacity: f64,
) -> RuntimeResult<()> {
    let _ = (window, opacity);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setOpacity",
    ))
    .boxed())
}

/// Set one window parent relationship.
pub(crate) fn destack_display_window_set_parent(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    parent: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    let _ = (window, parent);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setParent",
    ))
    .boxed())
}

/// Set one window taskbar visibility state.
pub(crate) fn destack_display_window_set_taskbar_visible(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    let _ = (window, visible);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setTaskbarVisible",
    ))
    .boxed())
}

/// Set one window transient-owner relationship.
pub(crate) fn destack_display_window_set_transient_for(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    window: resource::WindowHandle,
    transientfor: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    let _ = (window, transientfor);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setTransientFor",
    ))
    .boxed())
}
