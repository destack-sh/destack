#[cfg(not(target_os = "macos"))]
use super::super::backend::backend_not_supported;
#[cfg(target_os = "macos")]
use super::abi::{
    AudioObjectAddPropertyListener, AudioObjectID, AudioObjectPropertyAddress,
    AudioObjectPropertySelector, AudioObjectRemovePropertyListener,
};
#[cfg(target_os = "macos")]
use super::constants::{
    K_AUDIO_HARDWARE_PROPERTY_DEFAULT_INPUT_DEVICE,
    K_AUDIO_HARDWARE_PROPERTY_DEFAULT_OUTPUT_DEVICE,
    K_AUDIO_HARDWARE_PROPERTY_DEFAULT_SYSTEM_OUTPUT_DEVICE, K_AUDIO_HARDWARE_PROPERTY_DEVICES,
    K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL, K_AUDIO_OBJECT_SYSTEM_OBJECT, K_NO_ERR,
};
#[cfg(target_os = "macos")]
use super::format::property_address;
#[cfg(target_os = "macos")]
use super::property::error;
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "macos")]
use crate::platform::audio::core as audio_core;
use crate::runtime::BindingCallContext;

#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicBool, Ordering};

/// Return the CoreAudio system selectors used for backend-wide device notifications.
#[cfg(target_os = "macos")]
const fn monitor_selectors() -> [AudioObjectPropertySelector; 4] {
    [
        K_AUDIO_HARDWARE_PROPERTY_DEVICES,
        K_AUDIO_HARDWARE_PROPERTY_DEFAULT_OUTPUT_DEVICE,
        K_AUDIO_HARDWARE_PROPERTY_DEFAULT_INPUT_DEVICE,
        K_AUDIO_HARDWARE_PROPERTY_DEFAULT_SYSTEM_OUTPUT_DEVICE,
    ]
}

/// Register one CoreAudio property listener for one selector.
#[cfg(target_os = "macos")]
fn add_property_listener_with_user_data(
    selector: AudioObjectPropertySelector,
    user_data: *mut std::ffi::c_void,
) -> RuntimeResult<()> {
    let address = property_address(selector, K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL);
    let status = unsafe {
        AudioObjectAddPropertyListener(
            K_AUDIO_OBJECT_SYSTEM_OBJECT,
            &address,
            Some(coreaudio_device_property_listener),
            user_data,
        )
    };
    if status == K_NO_ERR {
        return Ok(());
    }

    Err(error(
        "destack.audio.event.open",
        status,
        "failed to register CoreAudio device property listener",
    ))
}

/// Unregister one CoreAudio property listener for one selector.
#[cfg(target_os = "macos")]
fn remove_property_listener_with_user_data(
    selector: AudioObjectPropertySelector,
    user_data: *mut std::ffi::c_void,
) {
    let address = property_address(selector, K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL);
    let _ = unsafe {
        AudioObjectRemovePropertyListener(
            K_AUDIO_OBJECT_SYSTEM_OBJECT,
            &address,
            Some(coreaudio_device_property_listener),
            user_data,
        )
    };
}

/// Handle one CoreAudio backend device-property notification.
#[cfg(target_os = "macos")]
unsafe extern "C" fn coreaudio_device_property_listener(
    _in_object_id: AudioObjectID,
    _in_number_addresses: u32,
    _in_addresses: *const AudioObjectPropertyAddress,
    in_client_data: *mut std::ffi::c_void,
) -> i32 {
    if in_client_data.is_null() {
        return K_NO_ERR;
    }

    let runtime_state = unsafe { &*(in_client_data as *const audio_core::AudioEventRuntimeState) };
    let _ = std::panic::catch_unwind(|| {
        audio_core::publish_device_snapshot_native(
            runtime_state,
            audio_core::AudioBackend::CoreAudio,
        );
    });

    K_NO_ERR
}

/// Runtime-owned mutable state for CoreAudio monitor registration.
#[cfg(target_os = "macos")]
#[derive(Debug, Default)]
struct CoreAudioMonitorRuntimeState {
    /// Whether listeners are currently registered.
    started: AtomicBool,
    /// Whether teardown finalizer was registered.
    shutdown_registered: AtomicBool,
}

/// Return runtime-owned monitor state for CoreAudio listeners.
#[cfg(target_os = "macos")]
fn coreaudio_monitor_runtime_state(
    context: &BindingCallContext,
) -> std::sync::Arc<CoreAudioMonitorRuntimeState> {
    let runtime_state = context
        .runtime()
        .module_state
        .get_or_init(CoreAudioMonitorRuntimeState::default);
    register_runtime_finalizer(context, &runtime_state);

    runtime_state
}

/// Register one runtime finalizer for CoreAudio monitor teardown.
#[cfg(target_os = "macos")]
fn register_runtime_finalizer(
    context: &BindingCallContext,
    runtime_state: &std::sync::Arc<CoreAudioMonitorRuntimeState>,
) {
    if runtime_state
        .shutdown_registered
        .swap(true, Ordering::AcqRel)
    {
        return;
    }

    let runtime_state = std::sync::Arc::clone(runtime_state);
    let audio_runtime_state = audio_core::audio_event_runtime_state(context);
    context.runtime().finalizers.register(move || {
        if !runtime_state.started.swap(false, Ordering::AcqRel) {
            return;
        }

        let user_data = std::sync::Arc::as_ptr(&audio_runtime_state) as *mut std::ffi::c_void;
        for selector in monitor_selectors() {
            remove_property_listener_with_user_data(selector, user_data);
        }

        drop(audio_runtime_state);
    });
}

/// Return whether CoreAudio native device-event monitoring is available.
pub(crate) fn native_device_events_supported() -> bool {
    cfg!(target_os = "macos")
}

/// Start CoreAudio native device-event monitoring.
pub(crate) fn start_native_device_event_monitor(context: &BindingCallContext) -> RuntimeResult<()> {
    #[cfg(target_os = "macos")]
    {
        let runtime_state = coreaudio_monitor_runtime_state(context);
        if runtime_state.started.swap(true, Ordering::AcqRel) {
            return Ok(());
        }

        let audio_runtime_state = audio_core::audio_event_runtime_state(context);
        let user_data = std::sync::Arc::as_ptr(&audio_runtime_state) as *mut std::ffi::c_void;
        let selectors = monitor_selectors();
        for (index, selector) in selectors.iter().copied().enumerate() {
            if let Err(error) = add_property_listener_with_user_data(selector, user_data) {
                for registered_selector in selectors.iter().take(index).copied() {
                    remove_property_listener_with_user_data(registered_selector, user_data);
                }
                runtime_state.started.store(false, Ordering::Release);
                return Err(error);
            }
        }

        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err(backend_not_supported(
            "destack.audio.event.open",
            "coreaudio",
        ))
    }
}

/// Stop CoreAudio native device-event monitoring.
pub(crate) fn stop_native_device_event_monitor(context: &BindingCallContext) {
    #[cfg(target_os = "macos")]
    {
        let runtime_state = coreaudio_monitor_runtime_state(context);
        if !runtime_state.started.swap(false, Ordering::AcqRel) {
            return;
        }

        let audio_runtime_state = audio_core::audio_event_runtime_state(context);
        let user_data = std::sync::Arc::as_ptr(&audio_runtime_state) as *mut std::ffi::c_void;
        for selector in monitor_selectors() {
            remove_property_listener_with_user_data(selector, user_data);
        }
    }
}
