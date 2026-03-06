use std::ffi::CString;
use std::fs::File;
use std::os::fd::FromRawFd;
use std::sync::{Arc, Mutex};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayBackendCapabilityFlags, DisplayMode, WindowAspectRatio, WindowChromeKind,
    WindowLogicalSize, WindowModeOptions, WindowOcclusionState, WindowPhysicalSize, WindowRole,
    WindowSizeConstraints, WindowVisibility,
};
use crate::platform::{core as core_platform, display as display_platform, resource};
use crate::runtime::BindingCallContext;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1;

use super::super::model::WaylandWindowBinding;
use super::super::{backend_descriptor_state, core as backend_core, resource as display_resource};

/// Normalize one window logical-size payload.
pub(crate) fn normalize_logical_size(
    value: WindowLogicalSize,
    field: &'static str,
) -> RuntimeResult<WindowLogicalSize> {
    // reject non-finite values
    if !value.width.is_finite() || !value.height.is_finite() {
        return Err(core_platform::invalid_argument(
            field,
            "logical size must be finite",
        ));
    }

    // reject non-positive values
    if value.width <= 0.0 || value.height <= 0.0 {
        return Err(core_platform::invalid_argument(
            field,
            "logical size dimensions must be greater than zero",
        ));
    }

    Ok(value)
}

/// Normalize one window physical-size payload.
pub(crate) fn normalize_physical_size(
    value: WindowPhysicalSize,
    field: &'static str,
) -> RuntimeResult<WindowPhysicalSize> {
    // reject non-positive values
    if value.width == 0 || value.height == 0 {
        return Err(core_platform::invalid_argument(
            field,
            "physical size dimensions must be greater than zero",
        ));
    }

    Ok(value)
}

/// Normalize one whole-window opacity payload.
pub(crate) fn normalize_opacity(value: f64, field: &'static str) -> RuntimeResult<f64> {
    // reject non-finite values
    if !value.is_finite() {
        return Err(core_platform::invalid_argument(
            field,
            "opacity must be finite",
        ));
    }

    // reject values outside normalized opacity range
    if !(0.0..=1.0).contains(&value) {
        return Err(core_platform::invalid_argument(
            field,
            "opacity must be between 0.0 and 1.0 inclusive",
        ));
    }

    Ok(value)
}

/// Convert one normalized opacity value into one alpha-modifier multiplier.
pub(crate) fn opacity_multiplier(value: f64) -> u32 {
    let scaled = (value * u32::MAX as f64).round();
    scaled.clamp(0.0, u32::MAX as f64) as u32
}

/// Validate one optional size-constraint payload.
pub(crate) fn validate_size_constraints(
    constraints: Option<WindowSizeConstraints>,
    field: &'static str,
) -> RuntimeResult<()> {
    let Some(constraints) = constraints else {
        return Ok(());
    };

    // validate minimum bounds when configured
    if let Some(minimum) = constraints.min {
        normalize_logical_size(minimum, field)?;
    }

    // validate maximum bounds when configured
    if let Some(maximum) = constraints.max {
        normalize_logical_size(maximum, field)?;
    }

    // validate min and max ordering when both are configured
    if let (Some(minimum), Some(maximum)) = (constraints.min, constraints.max)
        && (minimum.width > maximum.width || minimum.height > maximum.height)
    {
        return Err(core_platform::invalid_argument(
            field,
            "minimum logical size must not exceed maximum logical size",
        ));
    }

    Ok(())
}

/// Convert one logical-size payload into one physical-size payload.
pub(crate) fn logical_to_physical(
    logical: WindowLogicalSize,
    scale_factor_milli: u32,
) -> WindowPhysicalSize {
    // resolve numeric scale factor from milli value
    let scale = if scale_factor_milli == 0 {
        1.0
    } else {
        scale_factor_milli as f64 / 1000.0
    };

    // convert and clamp each logical dimension
    let width = (logical.width * scale).round().max(1.0) as u32;
    let height = (logical.height * scale).round().max(1.0) as u32;

    WindowPhysicalSize { width, height }
}

/// Clamp one logical-size payload to optional size constraints.
pub(crate) fn clamp_logical_size(
    value: WindowLogicalSize,
    constraints: Option<WindowSizeConstraints>,
) -> WindowLogicalSize {
    let Some(constraints) = constraints else {
        return value;
    };

    let mut width = value.width;
    let mut height = value.height;

    // enforce minimum bounds when configured
    if let Some(minimum) = constraints.min {
        width = width.max(minimum.width);
        height = height.max(minimum.height);
    }

    // enforce maximum bounds when configured
    if let Some(maximum) = constraints.max {
        width = width.min(maximum.width);
        height = height.min(maximum.height);
    }

    WindowLogicalSize { width, height }
}

/// Clamp one logical-size payload to optional aspect-ratio lock.
pub(crate) fn clamp_logical_aspect(
    value: WindowLogicalSize,
    aspect_ratio: Option<WindowAspectRatio>,
) -> WindowLogicalSize {
    let Some(aspect_ratio) = aspect_ratio else {
        return value;
    };

    // reject invalid aspect values by leaving size unchanged
    if aspect_ratio.numerator == 0 || aspect_ratio.denominator == 0 {
        return value;
    }

    // compute target ratio and adjust height to current width
    let ratio = aspect_ratio.numerator as f64 / aspect_ratio.denominator as f64;
    let height = (value.width / ratio).max(1.0);

    WindowLogicalSize {
        width: value.width,
        height,
    }
}

/// Resolve one preferred display handle from one mode payload.
pub(crate) fn mode_display(mode: WindowModeOptions) -> Option<resource::DisplayHandle> {
    match mode {
        WindowModeOptions::WindowWindowedModeOptions(_) => None,
        WindowModeOptions::WindowBorderlessModeOptions(value) => value.display,
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(value) => Some(value.display),
    }
}

/// Resolve one preferred display mode from one mode payload.
pub(crate) fn mode_display_mode(mode: WindowModeOptions) -> Option<DisplayMode> {
    match mode {
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(value) => value.display_mode,
        _ => None,
    }
}

/// Compare two mode payloads by semantic fields.
pub(crate) fn same_window_mode(left: WindowModeOptions, right: WindowModeOptions) -> bool {
    match (left, right) {
        (
            WindowModeOptions::WindowWindowedModeOptions(_),
            WindowModeOptions::WindowWindowedModeOptions(_),
        ) => true,
        (
            WindowModeOptions::WindowBorderlessModeOptions(left),
            WindowModeOptions::WindowBorderlessModeOptions(right),
        ) => left.display == right.display,
        (
            WindowModeOptions::WindowExclusiveFullscreenModeOptions(left),
            WindowModeOptions::WindowExclusiveFullscreenModeOptions(right),
        ) => left.display == right.display && left.display_mode == right.display_mode,
        _ => false,
    }
}

/// Resolve one occlusion value from one wayland visibility state.
pub(crate) fn occlusion_from_visibility(visibility: WindowVisibility) -> WindowOcclusionState {
    // minimized windows are compositor-hidden from presentation
    if visibility == WindowVisibility::Minimized {
        return WindowOcclusionState::Occluded;
    }

    // wayland does not expose one portable visible-window occlusion query
    WindowOcclusionState::Unknown
}

/// Return one wayland decoration mode for one runtime chrome and decorated state.
pub(crate) fn decoration_mode_for_window(
    chrome: WindowChromeKind,
    decorated: bool,
) -> zxdg_toplevel_decoration_v1::Mode {
    // force client-side mode when decorations are disabled
    if !decorated {
        return zxdg_toplevel_decoration_v1::Mode::ClientSide;
    }

    // otherwise select the closest compositor decoration style
    match chrome {
        WindowChromeKind::Popup => zxdg_toplevel_decoration_v1::Mode::ClientSide,
        WindowChromeKind::Standard | WindowChromeKind::Tool => {
            zxdg_toplevel_decoration_v1::Mode::ServerSide
        }
    }
}

/// Ensure the calling thread owns one window binding.
pub(crate) fn ensure_window_thread(
    binding: &WaylandWindowBinding,
    operation: &'static str,
) -> RuntimeResult<()> {
    // reject operations from non-owner threads
    let current_thread_id = std::thread::current().id();
    if current_thread_id != binding.owner_thread_id {
        return Err(core_platform::invalid_argument(
            "window",
            format!("{operation} must run on the owner thread of this window"),
        ));
    }

    Ok(())
}

/// Resolve one window binding and enforce owner-thread affinity.
pub(crate) fn resolve_window_binding(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<WaylandWindowBinding>>> {
    let binding = display_resource::resolve_window_binding(context, window_handle, operation)?;
    let binding_guard = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding_guard, operation)?;
    drop(binding_guard);

    Ok(binding)
}

/// Resolve one required xdg_toplevel id from one window binding.
pub(crate) fn require_xdg_toplevel_id(
    binding: &WaylandWindowBinding,
    operation: &'static str,
) -> RuntimeResult<wayland_client::backend::ObjectId> {
    let Some(toplevel_id) = binding.host.xdg_toplevel.clone() else {
        return Err(core_platform::not_supported(operation));
    };

    if toplevel_id.is_null() {
        return Err(core_platform::not_supported(operation));
    }

    Ok(toplevel_id)
}

/// Resolve one window binding and run one immutable callback under the binding lock.
pub(crate) fn with_window_binding<R>(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    operation: &'static str,
    callback: impl FnOnce(&WaylandWindowBinding) -> RuntimeResult<R>,
) -> RuntimeResult<R> {
    let binding = resolve_window_binding(context, window_handle, operation)?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    callback(&binding)
}

/// Resolve one window binding and run one mutable callback under the binding lock.
pub(crate) fn with_window_binding_mut<R>(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    operation: &'static str,
    callback: impl FnOnce(&mut WaylandWindowBinding) -> RuntimeResult<R>,
) -> RuntimeResult<R> {
    let binding = resolve_window_binding(context, window_handle, operation)?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    callback(&mut binding)
}

/// Resolve one effective capability mask for one opened wayland window.
pub(crate) fn effective_window_capabilities(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    operation: &'static str,
) -> RuntimeResult<DisplayBackendCapabilityFlags> {
    // resolve backend descriptor capabilities and this window role snapshot
    let mut capability_flags = backend_descriptor_state(context).1.0;
    let role = with_window_binding(
        context,
        window_handle,
        operation,
        |binding| Ok(binding.role),
    )?;

    // window capability flags are method-level and role-specific: role-open bits are not applicable
    capability_flags &= !display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_POPUP.0;
    capability_flags &= !display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_OVERLAY.0;

    // non-toplevel roles do not expose xdg_toplevel-only lanes
    if role != WindowRole::Toplevel {
        capability_flags &= !display_platform::DISPLAY_BACKEND_CAP_BORDERLESS_FULLSCREEN.0;
        capability_flags &= !display_platform::DISPLAY_BACKEND_CAP_WINDOW_DRAG_INTERACTION.0;
        capability_flags &= !display_platform::DISPLAY_BACKEND_CAP_WINDOW_PARENTING.0;
        capability_flags &= !display_platform::DISPLAY_BACKEND_CAP_WINDOW_MODAL.0;
        capability_flags &= !display_platform::DISPLAY_BACKEND_CAP_WINDOW_CHROME.0;
        capability_flags &= !display_platform::DISPLAY_BACKEND_CAP_WINDOW_ICON.0;
        capability_flags &= !display_platform::DISPLAY_BACKEND_CAP_ALWAYS_ON_TOP.0;
        capability_flags &= !display_platform::DISPLAY_BACKEND_CAP_WINDOW_TASKBAR_VISIBILITY.0;
    }

    Ok(DisplayBackendCapabilityFlags(capability_flags))
}

/// Create one anonymous in-memory file descriptor for wayland shm uploads.
pub(crate) fn create_memfd_file(
    operation: &'static str,
    name: &'static str,
    length: usize,
) -> RuntimeResult<File> {
    // build one stable memfd label for diagnostics
    let name = CString::new(name).map_err(|error| {
        backend_core::io_error(operation, format!("invalid memfd name: {error}"))
    })?;

    // allocate one anonymous file descriptor
    let file_descriptor = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };
    if file_descriptor < 0 {
        let error = std::io::Error::last_os_error();
        return Err(backend_core::io_error(
            operation,
            format!("memfd_create failed: {error}"),
        ));
    }

    // grow file to requested payload length
    let truncated = unsafe { libc::ftruncate(file_descriptor, length as libc::off_t) };
    if truncated != 0 {
        let error = std::io::Error::last_os_error();
        unsafe {
            libc::close(file_descriptor);
        }
        return Err(backend_core::io_error(
            operation,
            format!("ftruncate failed: {error}"),
        ));
    }

    // transfer ownership to std::fs::File
    let file = unsafe { File::from_raw_fd(file_descriptor) };
    Ok(file)
}

/// Pump one iteration of pending wayland window messages.
pub(crate) fn pump_window_messages(context: &BindingCallContext) -> RuntimeResult<()> {
    backend_core::dispatch_pending(context, "destack.display.window.eventRead")
}
