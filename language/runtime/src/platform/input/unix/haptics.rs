use super::core as input_core;
#[cfg(target_os = "linux")]
use super::linux as input_linux;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{
    InputDeviceKind, InputHapticEffectParameters, InputHapticEffectType, InputHapticsResult,
    validation as input_validation,
};
use crate::platform::{NativeArray, PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Resolve one opened haptics-capable Unix binding.
fn resolve_haptics_binding(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<input_core::UnixInputBinding> {
    // validate one opened unix input resolved_binding
    let resolved_binding = input_core::resolve_unix_input_binding(binding, handle, operation)?;

    // haptics currently ride on gamepad-capable handles
    if resolved_binding.device_kind != InputDeviceKind::Gamepad {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // require platform-backed descriptor handles for haptics operations
    if resolved_binding.backend != input_core::UnixInputBackend::Platform
        || resolved_binding.descriptor.is_none()
    {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(resolved_binding)
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

    // resolve one opened haptics-capable resolved_binding
    let resolved_binding =
        resolve_haptics_binding(binding, handle, "destack.input.haptics.effects")?;

    // route effect discovery by host support
    #[cfg(target_os = "linux")]
    {
        let Some(descriptor) = resolved_binding.descriptor else {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.input.haptics.effects",
            ))
            .boxed());
        };

        let effects = input_linux::linux_haptics_effects(descriptor);
        if effects.is_empty() {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.input.haptics.effects",
            ))
            .boxed());
        }
        unsafe {
            *out = binding.store_array(effects);
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = resolved_binding;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.input.haptics.effects",
        ))
        .boxed())
    }
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

    // resolve one opened haptics-capable resolved_binding
    let resolved_binding = resolve_haptics_binding(binding, handle, "destack.input.haptics.play")?;

    // validate requested effect payload
    input_validation::validate_haptics_params(params)?;
    if effect != InputHapticEffectType::DualRumble {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.input.haptics.play")).boxed(),
        );
    }

    // route playback by host support
    #[cfg(target_os = "linux")]
    {
        let Some(descriptor) = resolved_binding.descriptor else {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.input.haptics.play",
            ))
            .boxed());
        };

        let effects = input_linux::linux_haptics_effects(descriptor);
        if effects.is_empty() {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.input.haptics.play",
            ))
            .boxed());
        }

        let previous_effect_id = input_core::linux_active_rumble_effect_id(
            binding,
            handle,
            "destack.input.haptics.play",
        )?;
        if let Some(previous_effect_id) = previous_effect_id {
            input_linux::stop_linux_rumble(
                descriptor,
                previous_effect_id,
                "destack.input.haptics.play",
            )?;
        }

        let effect_id =
            input_linux::play_linux_rumble(descriptor, params, "destack.input.haptics.play")?;
        input_core::set_linux_active_rumble_effect_id(
            binding,
            handle,
            Some(effect_id),
            "destack.input.haptics.play",
        )?;

        let result = if previous_effect_id.is_some() {
            InputHapticsResult::Preempted
        } else {
            InputHapticsResult::Complete
        };
        unsafe {
            *out = result;
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (resolved_binding, effect, params);
        Err(RuntimeError::from(PlatformError::not_supported("destack.input.haptics.play")).boxed())
    }
}

/// Stop active haptic effects.
pub(crate) unsafe fn destack_input_haptics_stop(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // resolve one opened haptics-capable resolved_binding
    let resolved_binding = resolve_haptics_binding(binding, handle, "destack.input.haptics.stop")?;

    // route stop semantics by host support
    #[cfg(target_os = "linux")]
    {
        let Some(descriptor) = resolved_binding.descriptor else {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.input.haptics.stop",
            ))
            .boxed());
        };

        let active_effect_id = input_core::linux_active_rumble_effect_id(
            binding,
            handle,
            "destack.input.haptics.stop",
        )?;
        if let Some(active_effect_id) = active_effect_id {
            input_linux::stop_linux_rumble(
                descriptor,
                active_effect_id,
                "destack.input.haptics.stop",
            )?;
        }

        input_core::set_linux_active_rumble_effect_id(
            binding,
            handle,
            None,
            "destack.input.haptics.stop",
        )?;
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = resolved_binding;
        Err(RuntimeError::from(PlatformError::not_supported("destack.input.haptics.stop")).boxed())
    }
}
