use super::{core as input_core, xinput as xinput_input};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{
    InputHapticEffectParameters, InputHapticEffectType, InputHapticsResult,
    validation as input_validation,
};
use crate::platform::{NativeArray, PlatformError, resource};
use crate::runtime::BindingCallContext;

/// List supported haptic effects for one opened Windows input handle.
pub(super) fn haptics_effects(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<Vec<InputHapticEffectType>> {
    // resolve one backend binding and validate xinput gamepad support
    let resolved = input_core::resolve_input(binding, handle, operation)?;
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
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    effect: InputHapticEffectType,
    params: InputHapticEffectParameters,
    operation: &'static str,
) -> RuntimeResult<InputHapticsResult> {
    // resolve one backend binding and validate xinput gamepad support
    let resolved = input_core::resolve_input(binding, handle, operation)?;
    if resolved.backend != input_core::WindowsInputBackend::XInput {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // validate requested effect payload
    input_validation::validate_haptics_params(params)?;

    let Some(user_index) = resolved.xinput_user_index else {
        return Err(input_core::input_not_found(operation, handle));
    };
    xinput_input::play_haptics_for_xinput(user_index, effect, params, operation)
}

/// Stop active haptic effects for one opened Windows input handle.
pub(super) fn haptics_stop(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve one backend binding and validate xinput gamepad support
    let resolved = input_core::resolve_input(binding, handle, operation)?;
    if resolved.backend != input_core::WindowsInputBackend::XInput {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    let Some(user_index) = resolved.xinput_user_index else {
        return Err(input_core::input_not_found(operation, handle));
    };
    xinput_input::stop_haptics_for_xinput(user_index, operation)
}

/// List supported haptic effects.
pub(crate) unsafe fn destack_input_haptics_effects(
    binding: &BindingCallContext,
    out: *mut NativeArray<InputHapticEffectType>,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // query host-supported haptic effect kinds
    let effects = haptics_effects(binding, handle, "destack.input.haptics.effects")?;

    // write output array
    unsafe {
        *out = binding.store_array(effects);
    }

    Ok(())
}

/// Play one haptic effect.
pub(crate) unsafe fn destack_input_haptics_play(
    binding: &BindingCallContext,
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
        binding,
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
pub(crate) unsafe fn destack_input_haptics_stop(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    haptics_stop(binding, handle, "destack.input.haptics.stop")
}
