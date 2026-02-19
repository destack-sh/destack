use super::{core as input_core, xinput as xinput_input};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{
    InputHapticEffectParameters, InputHapticEffectType, InputHapticsResult,
};
use crate::platform::{NativeArray, PlatformError, resource};
use crate::runtime::RuntimeCallContext;

/// List supported haptic effects for one opened Windows input handle.
pub(super) fn haptics_effects(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<Vec<InputHapticEffectType>> {
    // resolve one backend binding and validate xinput gamepad support
    let resolved = input_core::resolve_input(context, handle, operation)?;
    if resolved.backend != input_core::WindowsInputBackend::XInput {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    let Some(user_index) = resolved.xinput_user_index else {
        return Err(input_core::input_not_found(operation, handle));
    };
    xinput_input::haptics_effects_for_xinput(user_index, operation)
}

/// Play one haptic effect for one opened Windows input handle.
pub(super) fn haptics_play(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    effect: InputHapticEffectType,
    params: InputHapticEffectParameters,
    operation: &'static str,
) -> RuntimeResult<InputHapticsResult> {
    // resolve one backend binding and validate xinput gamepad support
    let resolved = input_core::resolve_input(context, handle, operation)?;
    if resolved.backend != input_core::WindowsInputBackend::XInput {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    let Some(user_index) = resolved.xinput_user_index else {
        return Err(input_core::input_not_found(operation, handle));
    };
    xinput_input::play_haptics_for_xinput(user_index, effect, params, operation)
}

/// Stop active haptic effects for one opened Windows input handle.
pub(super) fn haptics_stop(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve one backend binding and validate xinput gamepad support
    let resolved = input_core::resolve_input(context, handle, operation)?;
    if resolved.backend != input_core::WindowsInputBackend::XInput {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    let Some(user_index) = resolved.xinput_user_index else {
        return Err(input_core::input_not_found(operation, handle));
    };
    xinput_input::stop_haptics_for_xinput(user_index, operation)
}

/// List supported haptic effects.
///
/// Return supported haptic effect kinds for one opened haptics-capable device.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where haptics is unavailable.
/// Uses backend-specific haptic capability queries for controller and endpoint actuators.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `input.haptics`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_haptics_effects(
    context: &RuntimeCallContext,
    out: *mut NativeArray<InputHapticEffectType>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // query host-supported haptic effect kinds
    let effects = haptics_effects(context, handle, "destack.input.haptics.effects")?;

    // write output array
    unsafe {
        *out = context.store_array(effects);
    }

    Ok(())
}

/// Play one haptic effect.
///
/// Schedule one haptic effect on one opened haptics-capable device.
/// Effect playback timing and motor resolution follow backend capabilities.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where one effect type is unavailable.
/// Uses backend-specific rumble and haptics APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.haptics`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_haptics_play(
    context: &RuntimeCallContext,
    out: *mut InputHapticsResult,
    handle: resource::InputDeviceHandle,
    effect: InputHapticEffectType,
    params: InputHapticEffectParameters,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // submit one host haptic command
    let result = haptics_play(
        context,
        handle,
        effect,
        params,
        "destack.input.haptics.play",
    )?;

    // write command result
    unsafe {
        *out = result;
    }

    Ok(())
}

/// Stop active haptic effects.
///
/// Stop active haptic playback on one opened haptics-capable device.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific haptic stop operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.haptics`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_haptics_stop(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    haptics_stop(context, handle, "destack.input.haptics.stop")
}
