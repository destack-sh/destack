use super::core as input_core;
#[cfg(target_os = "linux")]
use super::linux as input_linux;
#[cfg(target_os = "macos")]
use super::macos as input_macos;
#[cfg(target_os = "linux")]
use std::os::unix::io::RawFd;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{
    InputDeviceKind, InputPointerGrabMode, InputPointerState, InputWindowTarget,
    validation as input_validation,
};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Return whether one binding targets the macos global-session pointer backend.
#[cfg(target_os = "macos")]
fn is_macos_session_pointer_binding(binding: &input_core::UnixInputBinding) -> bool {
    binding.backend == input_core::UnixInputBackend::Platform
        && binding.device_id == input_macos::MACOS_INPUT_SESSION_ID
}

/// Validate pointer capability for one opened unix input handle.
fn resolve_pointer_binding(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<input_core::UnixInputBinding> {
    // resolve one opened unix input resolved_binding
    let resolved_binding = input_core::resolve_unix_input_binding(binding, handle, operation)?;

    // accept macos global-session bindings as pointer-capable
    #[cfg(target_os = "macos")]
    if is_macos_session_pointer_binding(&resolved_binding) {
        return Ok(resolved_binding);
    }

    // validate one pointer-capable device kind
    if !matches!(
        resolved_binding.device_kind,
        InputDeviceKind::Mouse | InputDeviceKind::Pen | InputDeviceKind::Touch
    ) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(resolved_binding)
}

/// Read one pointer snapshot from one opened unix handle.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn pointer_state(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    relative: bool,
    operation: &'static str,
) -> RuntimeResult<InputPointerState> {
    // resolve one pointer-capable resolved_binding
    let resolved_binding = resolve_pointer_binding(binding, handle, operation)?;

    // derive one absolute pointer snapshot first
    let mut snapshot = match resolved_binding.backend {
        input_core::UnixInputBackend::UnixTerminal => {
            return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
        }
        input_core::UnixInputBackend::Platform => {
            #[cfg(target_os = "linux")]
            {
                let Some(descriptor) = resolved_binding.descriptor else {
                    return Err(input_core::input_not_found(operation, handle));
                };

                input_linux::pointer_state_snapshot(descriptor, operation)?
            }

            #[cfg(target_os = "macos")]
            {
                input_macos::pointer_state_snapshot(operation)?
            }

            #[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
            {
                return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
            }
        }
    };

    // return absolute snapshots directly
    if !relative {
        return Ok(snapshot);
    }

    // require one enabled relative mode before projecting deltas
    if !resolved_binding.relative_mode_enabled {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // project relative deltas from the per-handle baseline snapshot
    let delta_x = snapshot.x - resolved_binding.last_pointer_x;
    let delta_y = snapshot.y - resolved_binding.last_pointer_y;
    input_core::set_pointer_snapshot(binding, handle, snapshot.x, snapshot.y, operation)?;
    snapshot.x = delta_x;
    snapshot.y = delta_y;

    Ok(snapshot)
}

/// Read one pointer snapshot from one opened unix handle.
#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
fn pointer_state(
    _binding: &BindingCallContext,
    _handle: resource::InputDeviceHandle,
    _relative: bool,
    operation: &'static str,
) -> RuntimeResult<InputPointerState> {
    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Capture one pointer baseline and relative-mode flag for one opened unix handle.
#[cfg(target_os = "macos")]
fn set_relative_mode_macos(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    enabled: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    let snapshot = input_macos::pointer_state_snapshot(operation)?;
    input_core::set_pointer_snapshot(binding, handle, snapshot.x, snapshot.y, operation)?;
    input_core::set_relative_mode_flag(binding, handle, enabled, operation)?;
    Ok(())
}

/// Capture one relative-mode flag for one opened linux pointer handle.
#[cfg(target_os = "linux")]
fn set_relative_mode_linux(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    descriptor: Option<RawFd>,
    enabled: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    // require one descriptor-backed platform binding
    let Some(descriptor) = descriptor else {
        return Err(input_core::input_not_found(operation, handle));
    };

    // reset baseline when enabling relative mode
    if enabled {
        let snapshot = input_linux::pointer_state_snapshot(descriptor, operation)?;
        input_core::set_pointer_snapshot(binding, handle, snapshot.x, snapshot.y, operation)?;
    }

    input_core::set_relative_mode_flag(binding, handle, enabled, operation)
}

/// Capture one relative-mode flag for one opened unsupported unix pointer handle.
#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
fn set_relative_mode_other_unix(
    _binding: &BindingCallContext,
    _handle: resource::InputDeviceHandle,
    _enabled: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Set relative pointer mode for one opened unix handle.
fn set_relative_mode(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    enabled: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve one pointer-capable resolved_binding for handle validation
    let resolved_binding = resolve_pointer_binding(binding, handle, operation)?;

    // keep tty backends unsupported for pointer mode toggles
    if resolved_binding.backend == input_core::UnixInputBackend::UnixTerminal {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // route by host backend capabilities
    #[cfg(target_os = "macos")]
    {
        set_relative_mode_macos(binding, handle, enabled, operation)
    }

    #[cfg(target_os = "linux")]
    {
        return set_relative_mode_linux(
            binding,
            handle,
            resolved_binding.descriptor,
            enabled,
            operation,
        );
    }

    #[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
    {
        set_relative_mode_other_unix(binding, handle, enabled, operation)
    }
}

/// Set one pointer grab mode for one opened unix handle.
fn set_grab_mode(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    mode: InputPointerGrabMode,
    operation: &'static str,
) -> RuntimeResult<()> {
    // reject explicit window-scoped targets on unix backends
    input_validation::validate_global_window_target(target, operation)?;

    // resolve one pointer-capable resolved_binding
    let resolved_binding = resolve_pointer_binding(binding, handle, operation)?;

    // route supported modes to backend grab semantics
    match mode {
        InputPointerGrabMode::None => {
            set_relative_mode(binding, handle, false, operation)?;
            input_core::set_unix_grab(resolved_binding.descriptor, resolved_binding.backend, false)
        }
        InputPointerGrabMode::Locked => {
            #[cfg(target_os = "macos")]
            {
                // use relative-mode projection as the lock semantic on macos session backend
                set_relative_mode(binding, handle, true, operation)
            }

            #[cfg(not(target_os = "macos"))]
            {
                input_core::set_unix_grab(
                    resolved_binding.descriptor,
                    resolved_binding.backend,
                    true,
                )
            }
        }
        InputPointerGrabMode::Confined => {
            Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
        }
    }
}

/// Enable or disable pointer capture.
///
/// Toggle pointer capture for one opened pointer-capable device and one optional window target.
/// Captured pointers can continue delivering events outside focused bounds when supported for that target scope.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where capture or one window scope is unavailable.
/// Uses backend-specific pointer capture primitives for global or window-scoped paths.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.grab`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_pointer_capture(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    enabled: bool,
) -> RuntimeResult<()> {
    // reject explicit window targets on unix pointer capture
    input_validation::validate_global_window_target(target, "destack.input.pointer.capture")?;

    // resolve one pointer-capable resolved_binding
    let resolved_binding =
        resolve_pointer_binding(binding, handle, "destack.input.pointer.capture")?;

    // route capture by backend support
    #[cfg(target_os = "linux")]
    {
        if resolved_binding.backend != input_core::UnixInputBackend::Platform {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.input.pointer.capture",
            ))
            .boxed());
        }

        return input_core::set_unix_grab(
            resolved_binding.descriptor,
            resolved_binding.backend,
            enabled,
        );
    }

    #[cfg(target_os = "macos")]
    {
        if resolved_binding.backend != input_core::UnixInputBackend::Platform {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.input.pointer.capture",
            ))
            .boxed());
        }

        let _ = (resolved_binding, enabled);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.input.pointer.capture",
        ))
        .boxed())
    }

    #[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
    {
        let _ = (resolved_binding, enabled);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.input.pointer.capture",
        ))
        .boxed())
    }
}

/// Read one relative pointer state snapshot.
///
/// Return one relative motion and button state snapshot for one opened pointer-capable device.
/// Delta units follow backend-native relative motion semantics.
/// Pen-capable devices can populate pressure and tilt metadata.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific relative motion streams from evdev or libinput-style backends on Unix.
/// Uses raw-input relative motion on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
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
    let snapshot = pointer_state(binding, handle, true, "destack.input.pointer.relativeState")?;

    // write snapshot output
    unsafe {
        *out = snapshot;
    }

    Ok(())
}

/// Set pointer grab mode.
///
/// Apply one grab mode for one opened pointer-capable device and one optional window target.
/// Grab modes can confine or lock pointer movement depending on backend support and target scope.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where one grab mode or one window scope is unavailable.
/// Uses backend-specific pointer grab or lock primitives for global or window-scoped paths.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.grab`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_pointer_set_grab_mode(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    mode: InputPointerGrabMode,
) -> RuntimeResult<()> {
    set_grab_mode(
        binding,
        handle,
        target,
        mode,
        "destack.input.pointer.setGrabMode",
    )
}

/// Enable or disable relative pointer mode.
///
/// Toggle relative pointer mode for one opened pointer-capable device.
/// Relative mode semantics follow backend pointer-lock behavior.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where relative mode is unavailable.
/// Uses backend-specific relative mode toggles for active input endpoints.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_pointer_set_relative_mode(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    set_relative_mode(
        binding,
        handle,
        enabled,
        "destack.input.pointer.setRelativeMode",
    )
}

/// Read one absolute pointer state snapshot.
///
/// Return one current pointer position and button state snapshot for one opened pointer-capable device.
/// Position values follow backend coordinate space for that device.
/// Pen-capable devices can populate pressure and tilt metadata.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific pointer state queries from evdev or libinput-style streams on Unix.
/// Uses raw-input or console pointer state snapshots on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
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
    let snapshot = pointer_state(binding, handle, false, "destack.input.pointer.state")?;

    // write snapshot output
    unsafe {
        *out = snapshot;
    }

    Ok(())
}

/// Warp pointer position.
///
/// Set one pointer position for one opened pointer-capable device and one optional window target.
/// Warped coordinates are interpreted in backend-native window or surface space for the selected target scope.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where pointer warping or one window scope is unavailable.
/// Uses backend-specific pointer warp operations for global or window-scoped paths.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_pointer_warp(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    x: f64,
    y: f64,
) -> RuntimeResult<()> {
    // reject explicit window targets on unix pointer warps
    input_validation::validate_global_window_target(target, "destack.input.pointer.warp")?;

    // validate pointer-warp coordinates
    input_validation::validate_pointer_coordinates(x, y)?;

    // resolve one pointer-capable resolved_binding
    let resolved_binding = resolve_pointer_binding(binding, handle, "destack.input.pointer.warp")?;
    if resolved_binding.backend != input_core::UnixInputBackend::Platform {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.input.pointer.warp")).boxed(),
        );
    }

    // route pointer warp support by host backend
    #[cfg(target_os = "macos")]
    {
        input_macos::warp_pointer_position(x, y, "destack.input.pointer.warp")?;
        input_core::set_pointer_snapshot(binding, handle, x, y, "destack.input.pointer.warp")?;
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (binding, handle, x, y);
        Err(RuntimeError::from(PlatformError::not_supported("destack.input.pointer.warp")).boxed())
    }
}
