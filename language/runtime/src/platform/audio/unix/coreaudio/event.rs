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

#[cfg(target_os = "macos")]
use std::sync::{Mutex, OnceLock};

/// One shared CoreAudio native device-event monitor reference count.
#[cfg(target_os = "macos")]
static COREAUDIO_DEVICE_MONITOR_COUNT: OnceLock<Mutex<usize>> = OnceLock::new();

/// Return one shared CoreAudio monitor reference-count lock.
#[cfg(target_os = "macos")]
fn monitor_reference_count() -> &'static Mutex<usize> {
    COREAUDIO_DEVICE_MONITOR_COUNT.get_or_init(|| Mutex::new(0))
}

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
fn add_property_listener(selector: AudioObjectPropertySelector) -> RuntimeResult<()> {
    let address = property_address(selector, K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL);
    let status = unsafe {
        AudioObjectAddPropertyListener(
            K_AUDIO_OBJECT_SYSTEM_OBJECT,
            &address,
            Some(coreaudio_device_property_listener),
            std::ptr::null_mut(),
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
fn remove_property_listener(selector: AudioObjectPropertySelector) {
    let address = property_address(selector, K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL);
    let _ = unsafe {
        AudioObjectRemovePropertyListener(
            K_AUDIO_OBJECT_SYSTEM_OBJECT,
            &address,
            Some(coreaudio_device_property_listener),
            std::ptr::null_mut(),
        )
    };
}

/// Handle one CoreAudio backend device-property notification.
#[cfg(target_os = "macos")]
unsafe extern "C" fn coreaudio_device_property_listener(
    _in_object_id: AudioObjectID,
    _in_number_addresses: u32,
    _in_addresses: *const AudioObjectPropertyAddress,
    _in_client_data: *mut std::ffi::c_void,
) -> i32 {
    let _ = std::panic::catch_unwind(|| {
        audio_core::publish_device_snapshot_native(audio_core::AudioBackend::CoreAudio);
    });

    K_NO_ERR
}

/// Return whether CoreAudio native device-event monitoring is available.
pub(crate) fn native_device_events_supported() -> bool {
    cfg!(target_os = "macos")
}

/// Start CoreAudio native device-event monitoring.
pub(crate) fn start_native_device_event_monitor() -> RuntimeResult<()> {
    #[cfg(target_os = "macos")]
    {
        let mut reference_count = monitor_reference_count()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if *reference_count > 0 {
            *reference_count += 1;
            return Ok(());
        }

        let selectors = monitor_selectors();
        for (index, selector) in selectors.iter().copied().enumerate() {
            if let Err(error) = add_property_listener(selector) {
                for registered_selector in selectors.iter().take(index).copied() {
                    remove_property_listener(registered_selector);
                }
                return Err(error);
            }
        }

        *reference_count = 1;
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
pub(crate) fn stop_native_device_event_monitor() {
    #[cfg(target_os = "macos")]
    {
        let mut reference_count = monitor_reference_count()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if *reference_count == 0 {
            return;
        }

        *reference_count -= 1;
        if *reference_count > 0 {
            return;
        }

        for selector in monitor_selectors() {
            remove_property_listener(selector);
        }
    }
}
