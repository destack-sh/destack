use super::core as input_core;
#[cfg(target_os = "linux")]
use super::linux as input_linux;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{
    InputDeviceKind, InputHapticEffectParameters, InputHapticEffectType, InputHapticsResult,
    validation as input_validation,
};
use crate::platform::{NativeArray, PlatformError, resource};
use crate::runtime::RuntimeCallContext;

/// Resolve one opened haptics-capable Unix binding.
fn resolve_haptics_binding(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<input_core::UnixInputBinding> {
    // validate one opened unix input binding
    let binding = input_core::resolve_unix_input_binding(context, handle, operation)?;

    // haptics currently ride on gamepad-capable handles
    if binding.device_kind != InputDeviceKind::Gamepad {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // require platform-backed descriptor handles for haptics operations
    if binding.backend != input_core::UnixInputBackend::Platform || binding.descriptor.is_none() {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(binding)
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

    // resolve one opened haptics-capable binding
    let binding = resolve_haptics_binding(context, handle, "destack.input.haptics.effects")?;

    // route effect discovery by host support
    #[cfg(target_os = "linux")]
    {
        let Some(descriptor) = binding.descriptor else {
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
            *out = context.store_array(effects);
        }

        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = binding;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.input.haptics.effects",
        ))
        .boxed())
    }
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

    // resolve one opened haptics-capable binding
    let binding = resolve_haptics_binding(context, handle, "destack.input.haptics.play")?;

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
        let Some(descriptor) = binding.descriptor else {
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
            context,
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
            context,
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

        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, effect, params);
        Err(RuntimeError::from(PlatformError::not_supported("destack.input.haptics.play")).boxed())
    }
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
    // resolve one opened haptics-capable binding
    let binding = resolve_haptics_binding(context, handle, "destack.input.haptics.stop")?;

    // route stop semantics by host support
    #[cfg(target_os = "linux")]
    {
        let Some(descriptor) = binding.descriptor else {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.input.haptics.stop",
            ))
            .boxed());
        };

        let active_effect_id = input_core::linux_active_rumble_effect_id(
            context,
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
            context,
            handle,
            None,
            "destack.input.haptics.stop",
        )?;
        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = binding;
        Err(RuntimeError::from(PlatformError::not_supported("destack.input.haptics.stop")).boxed())
    }
}
