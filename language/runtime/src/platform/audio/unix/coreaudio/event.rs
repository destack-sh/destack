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
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "macos")]
use crate::platform::PlatformError;
#[cfg(target_os = "macos")]
use crate::platform::audio::AudioBackend;
#[cfg(not(target_os = "macos"))]
use crate::platform::audio::backend::backend_not_supported;
use crate::platform::audio::core::AudioMonitorHandle;
#[cfg(target_os = "macos")]
use crate::platform::audio::core::publish_device_snapshot_native;
#[cfg(target_os = "macos")]
use crate::platform::diagnostic::PlatformErrorCode;

#[cfg(target_os = "macos")]
static COREAUDIO_MONITOR_TOKEN: u8 = 0;

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

/// Build one backend-unavailable error for CoreAudio monitor registration.
#[cfg(target_os = "macos")]
fn monitor_registration_error(
    operation: &'static str,
    status: i32,
    message: impl Into<String>,
) -> Box<crate::diagnostic::RuntimeError> {
    crate::diagnostic::RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::AudioUnavailable),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("{} (osstatus {status})", message.into()),
    ))
    .boxed()
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

    Err(monitor_registration_error(
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

    let _ = in_client_data;
    let _ = std::panic::catch_unwind(|| {
        publish_device_snapshot_native(AudioBackend::CoreAudio);
    });

    K_NO_ERR
}

#[cfg(target_os = "macos")]
struct CoreAudioDeviceMonitor;

#[cfg(target_os = "macos")]
impl AudioMonitorHandle for CoreAudioDeviceMonitor {
    fn stop(self: Box<Self>) {
        let user_data = (&COREAUDIO_MONITOR_TOKEN as *const u8).cast_mut().cast();
        for selector in monitor_selectors() {
            remove_property_listener_with_user_data(selector, user_data);
        }
    }
}

/// Start CoreAudio native device-event monitoring.
pub(crate) fn start_native_device_event_monitor() -> RuntimeResult<Box<dyn AudioMonitorHandle>> {
    #[cfg(target_os = "macos")]
    {
        let user_data = (&COREAUDIO_MONITOR_TOKEN as *const u8).cast_mut().cast();
        let selectors = monitor_selectors();
        for (index, selector) in selectors.iter().copied().enumerate() {
            if let Err(error) = add_property_listener_with_user_data(selector, user_data) {
                for registered_selector in selectors.iter().take(index).copied() {
                    remove_property_listener_with_user_data(registered_selector, user_data);
                }
                return Err(error);
            }
        }

        Ok(Box::new(CoreAudioDeviceMonitor))
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err(backend_not_supported(
            "destack.audio.event.open",
            "coreaudio",
        ))
    }
}
