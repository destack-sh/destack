use super::{core as input_core, raw as raw_input};

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{
    InputDeviceKind, InputPenState, InputPointerGrabMode, InputPointerState, InputWindowTarget,
    validation as input_validation,
};
use crate::platform::{PlatformError, core as core_platform, resource};
use crate::runtime::BindingCallContext;

/// Return whether one resolved binding supports pointer state queries.
fn is_pointer_capable_backend(resolved: &input_core::WindowsInputResolved) -> bool {
    if resolved.backend == input_core::WindowsInputBackend::Console {
        return true;
    }

    if resolved.backend == input_core::WindowsInputBackend::RawDevice {
        let Some(raw_device) = resolved.raw_device.as_ref() else {
            return false;
        };
        return raw_device.kind == InputDeviceKind::Mouse;
    }

    false
}

/// Read one pointer state snapshot for one opened Windows input handle.
pub(super) fn pointer_state(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    relative: bool,
    operation: &'static str,
) -> RuntimeResult<InputPointerState> {
    // resolve one backend binding and validate pointer capability
    let resolved = input_core::resolve_input(binding, handle, operation)?;
    if !is_pointer_capable_backend(&resolved) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // sample one current pointer position and modifiers
    let (host_pointer_x, host_pointer_y) = input_core::current_pointer_position();
    let buttons = input_core::pointer_buttons_from_host();
    let modifiers = input_core::modifier_bits_from_host();

    // project pen-specific snapshots from active raw touch state when available
    let mut has_pen_data = false;
    let mut pen_pressure = 0.0;
    let mut pen_tilt_x = 0.0;
    let mut pen_tilt_y = 0.0;
    let mut pen_in_contact = false;
    let mut pen_in_range = true;
    let mut pointer_x = host_pointer_x;
    let mut pointer_y = host_pointer_y;
    if resolved.backend == input_core::WindowsInputBackend::RawDevice
        && let Some(raw_device) = resolved.raw_device.as_ref()
        && let Some(pen_state) = raw_input::read_pen_state(binding, raw_device, operation)?
    {
        has_pen_data = true;
        pen_pressure = pen_state.pressure;
        pen_tilt_x = pen_state.tilt_x;
        pen_tilt_y = pen_state.tilt_y;
        pen_in_contact = pen_state.in_contact;
        pen_in_range = pen_state.in_range;
        pointer_x = pen_state.x;
        pointer_y = pen_state.y;
    }

    // derive absolute or relative coordinates based on call mode
    let (x, y) = if relative {
        if !resolved.relative_mode_enabled {
            return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
        }

        let delta_x = pointer_x - resolved.last_pointer_x;
        let delta_y = pointer_y - resolved.last_pointer_y;
        input_core::set_pointer_snapshot(binding, handle, pointer_x, pointer_y, operation)?;
        (delta_x, delta_y)
    } else {
        (pointer_x, pointer_y)
    };

    let pen = if has_pen_data {
        Some(InputPenState {
            pressure: pen_pressure,
            tangential_pressure: 0.0,
            tilt_x: pen_tilt_x,
            tilt_y: pen_tilt_y,
            twist: 0.0,
            in_contact: pen_in_contact,
            in_range: pen_in_range,
        })
    } else {
        None
    };

    Ok(InputPointerState {
        x,
        y,
        buttons,
        modifiers,
        pen,
    })
}

/// Set relative pointer mode for one opened Windows input handle.
pub(super) fn pointer_set_relative_mode(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    enabled: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve one backend binding and validate pointer capability
    let resolved = input_core::resolve_input(binding, handle, operation)?;
    if !is_pointer_capable_backend(&resolved) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // reset the relative baseline before enabling or disabling the mode flag
    let (mut pointer_x, mut pointer_y) = input_core::current_pointer_position();
    if resolved.backend == input_core::WindowsInputBackend::RawDevice
        && let Some(raw_device) = resolved.raw_device.as_ref()
        && let Some(pen_state) = raw_input::read_pen_state(binding, raw_device, operation)?
    {
        pointer_x = pen_state.x;
        pointer_y = pen_state.y;
    }

    input_core::set_pointer_snapshot(binding, handle, pointer_x, pointer_y, operation)?;
    input_core::set_relative_mode_flag(binding, handle, enabled, operation)?;
    Ok(())
}

/// Set one pointer grab mode for one opened Windows input handle.
pub(super) fn pointer_set_grab_mode(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    mode: InputPointerGrabMode,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve one backend binding and validate pointer capability
    let resolved = input_core::resolve_input(binding, handle, operation)?;
    if !is_pointer_capable_backend(&resolved) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // handle raw-input pointer grabs without routing through capture semantics
    if resolved.backend == input_core::WindowsInputBackend::RawDevice {
        match mode {
            InputPointerGrabMode::None => {
                pointer_set_relative_mode(binding, handle, false, operation)?;
                return input_core::release_cursor_confine(operation);
            }
            InputPointerGrabMode::Locked => {
                let Some(target_window) =
                    input_core::resolve_window_target_handle(binding, target, operation)?
                else {
                    return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
                };

                pointer_set_relative_mode(binding, handle, true, operation)?;
                return input_core::confine_cursor_to_window(target_window, operation);
            }
            InputPointerGrabMode::Confined => {
                let Some(target_window) =
                    input_core::resolve_window_target_handle(binding, target, operation)?
                else {
                    return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
                };

                pointer_set_relative_mode(binding, handle, false, operation)?;
                return input_core::confine_cursor_to_window(target_window, operation);
            }
        }
    }

    // keep console pointer grabs unsupported: relative mode exists, but console input does not expose real capture or lock semantics
    if resolved.backend == input_core::WindowsInputBackend::Console {
        if mode == InputPointerGrabMode::None {
            pointer_set_relative_mode(binding, handle, false, operation)?;
            return input_core::release_cursor_confine(operation);
        }

        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // resolve one optional explicit target window handle
    let target_window = input_core::resolve_window_target_handle(binding, target, operation)?;

    // map locked mode to relative-pointer mode and allow none as explicit release
    if mode == InputPointerGrabMode::Locked {
        pointer_capture(binding, handle, target, true, operation)?;
        return pointer_set_relative_mode(binding, handle, true, operation);
    }
    if mode == InputPointerGrabMode::None {
        pointer_set_relative_mode(binding, handle, false, operation)?;
        pointer_capture(binding, handle, target, false, operation)?;
        return input_core::release_cursor_confine(operation);
    }

    // confine mode requires one explicit window target for deterministic bounds
    if mode == InputPointerGrabMode::Confined {
        let Some(target_window) = target_window else {
            return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
        };
        pointer_capture(binding, handle, target, true, operation)?;
        pointer_set_relative_mode(binding, handle, false, operation)?;
        return input_core::confine_cursor_to_window(target_window, operation);
    }

    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Toggle pointer capture mode for one opened Windows input handle.
pub(super) fn pointer_capture(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    enabled: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve one backend binding and validate pointer capability
    let resolved = input_core::resolve_input(binding, handle, operation)?;
    if !is_pointer_capable_backend(&resolved) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // keep raw-input pointer capture unsupported even for explicit window targets
    if resolved.backend == input_core::WindowsInputBackend::RawDevice {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // resolve one optional explicit target window handle
    let target_window = input_core::resolve_window_target_handle(binding, target, operation)?;

    // apply explicit per-window capture when a window target is provided
    if let Some(target_window) = target_window {
        if enabled {
            unsafe {
                SetCapture(target_window);
            }
            return Ok(());
        }

        let status = unsafe { ReleaseCapture() };
        if status == 0 {
            let code = core_platform::last_error_code() as u32;
            if code != 0 {
                return Err(input_core::io_error_with_code(
                    operation,
                    "ReleaseCapture",
                    code,
                    "failed to release pointer capture",
                ));
            }
        }
        return Ok(());
    }

    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Warp pointer position for one opened Windows input handle.
pub(super) fn pointer_warp(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    x: f64,
    y: f64,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve one optional explicit target window handle
    let target_window = input_core::resolve_window_target_handle(binding, target, operation)?;

    // resolve one backend binding and validate pointer capability
    let resolved = input_core::resolve_input(binding, handle, operation)?;
    if !is_pointer_capable_backend(&resolved) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // validate pointer-warp coordinates
    input_validation::validate_pointer_coordinates(x, y)?;

    // map optional window-relative coordinates into screen coordinates
    let (target_x, target_y) = if let Some(target_window) = target_window {
        let point = input_core::client_to_screen_point(target_window, x, y, operation)?;
        (f64::from(point.x), f64::from(point.y))
    } else {
        (x, y)
    };

    // apply one host pointer warp and update per-handle baseline state
    let status = unsafe { SetCursorPos(target_x.round() as i32, target_y.round() as i32) };
    if status == 0 {
        let code = core_platform::last_error_code() as u32;
        return Err(input_core::io_error_with_code(
            operation,
            "SetCursorPos",
            code,
            "failed to warp pointer position",
        ));
    }

    input_core::set_pointer_snapshot(binding, handle, target_x, target_y, operation)?;
    Ok(())
}

/// Enable or disable pointer capture.
pub(crate) unsafe fn destack_input_pointer_capture(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    enabled: bool,
) -> RuntimeResult<()> {
    pointer_capture(
        binding,
        handle,
        target,
        enabled,
        "destack.input.pointer.capture",
    )
}

/// Read one relative pointer state snapshot.
pub(crate) unsafe fn destack_input_pointer_relative_state(
    binding: &BindingCallContext,
    out: *mut InputPointerState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // query one relative pointer snapshot
    let state = pointer_state(binding, handle, true, "destack.input.pointer.relativeState")?;

    // write snapshot output
    unsafe {
        *out = state;
    }

    Ok(())
}

/// Set pointer grab mode.
pub(crate) unsafe fn destack_input_pointer_set_grab_mode(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    mode: InputPointerGrabMode,
) -> RuntimeResult<()> {
    pointer_set_grab_mode(
        binding,
        handle,
        target,
        mode,
        "destack.input.pointer.setGrabMode",
    )
}

/// Enable or disable relative pointer mode.
pub(crate) unsafe fn destack_input_pointer_set_relative_mode(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    pointer_set_relative_mode(
        binding,
        handle,
        enabled,
        "destack.input.pointer.setRelativeMode",
    )
}

/// Read one absolute pointer state snapshot.
pub(crate) unsafe fn destack_input_pointer_state(
    binding: &BindingCallContext,
    out: *mut InputPointerState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // query one absolute pointer snapshot
    let state = pointer_state(binding, handle, false, "destack.input.pointer.state")?;

    // write snapshot output
    unsafe {
        *out = state;
    }

    Ok(())
}

/// Warp pointer position.
pub(crate) unsafe fn destack_input_pointer_warp(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    x: f64,
    y: f64,
) -> RuntimeResult<()> {
    pointer_warp(binding, handle, target, x, y, "destack.input.pointer.warp")
}
