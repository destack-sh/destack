#[cfg(target_os = "macos")]
use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(target_os = "macos")]
use crate::platform::PlatformError;
#[cfg(target_os = "macos")]
use crate::platform::audio::core::error::audio_would_block;

#[cfg(target_os = "macos")]
use super::super::abi::{AudioDeviceID, CoreAudioStreamRuntime};
#[cfg(target_os = "macos")]
use super::super::constants::{
    K_AUDIO_DEVICE_PROPERTY_HOG_MODE, K_AUDIO_HARDWARE_PROPERTY_HOG_MODE_IS_ALLOWED,
    K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL, K_AUDIO_OBJECT_SYSTEM_OBJECT,
};
#[cfg(target_os = "macos")]
use super::access::{get_scalar_optional, set_data};

/// Return whether CoreAudio hog mode is globally allowed on this host.
#[cfg(target_os = "macos")]
pub(crate) fn hog_mode_allowed() -> bool {
    get_scalar_optional::<u32>(
        K_AUDIO_OBJECT_SYSTEM_OBJECT,
        K_AUDIO_HARDWARE_PROPERTY_HOG_MODE_IS_ALLOWED,
        K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
    )
    .unwrap_or(1)
        != 0
}

/// Return the current hog-mode owner process identifier for one device.
#[cfg(target_os = "macos")]
pub(crate) fn hog_owner_pid(device_id: AudioDeviceID) -> RuntimeResult<i32> {
    get_scalar_optional::<i32>(
        device_id,
        K_AUDIO_DEVICE_PROPERTY_HOG_MODE,
        K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
    )
    .ok_or_else(|| {
        RuntimeError::from(PlatformError::not_supported(
            "destack.audio.stream.open CoreAudio hog mode",
        ))
        .boxed()
    })
}

/// Toggle CoreAudio hog mode and return the resulting owner process identifier.
#[cfg(target_os = "macos")]
pub(crate) fn toggle_hog_mode(
    device_id: AudioDeviceID,
    operation: &'static str,
) -> RuntimeResult<i32> {
    let mut owner_pid = 0i32;
    let owner_pid_bytes = owner_pid.to_ne_bytes();
    set_data(
        device_id,
        K_AUDIO_DEVICE_PROPERTY_HOG_MODE,
        K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        &owner_pid_bytes,
    )
    .map_err(|status| {
        super::access::error(operation, status, "failed to toggle CoreAudio hog mode")
    })?;

    owner_pid = hog_owner_pid(device_id)?;
    Ok(owner_pid)
}

/// Acquire CoreAudio hog mode for one device when available.
#[cfg(target_os = "macos")]
pub(crate) fn enable_hog_mode(device_id: AudioDeviceID) -> RuntimeResult<bool> {
    // reject exclusive mode when the host disables hog mode globally
    if !hog_mode_allowed() {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.audio.stream.open CoreAudio exclusive mode",
        ))
        .boxed());
    }

    // inspect current ownership to enforce exclusive semantics
    let process_id = unsafe { libc::getpid() };
    let current_owner = hog_owner_pid(device_id)?;

    // no toggle is needed when this process already owns the device
    if current_owner == process_id {
        return Ok(false);
    }

    // return a would-block error when another process owns the device
    if current_owner != -1 {
        return Err(audio_would_block(
            "destack.audio.stream.open",
            format!("audio device is already hogged by pid {current_owner}"),
        ));
    }

    // claim ownership and verify this process became the owner
    let owner_after_toggle = toggle_hog_mode(device_id, "destack.audio.stream.open")?;
    if owner_after_toggle != process_id {
        return Err(audio_would_block(
            "destack.audio.stream.open",
            "audio device could not be acquired in exclusive mode",
        ));
    }

    Ok(true)
}

/// Release CoreAudio hog mode previously acquired by this runtime.
#[cfg(target_os = "macos")]
pub(crate) fn release_hog_mode(runtime: &CoreAudioStreamRuntime) {
    // skip release when this stream did not acquire hog mode
    if !runtime.release_hog_mode_on_drop {
        return;
    }

    // release only when this process currently owns the device
    let process_id = unsafe { libc::getpid() };
    let current_owner = hog_owner_pid(runtime.device_id);
    let Ok(current_owner) = current_owner else {
        return;
    };
    if current_owner != process_id {
        return;
    }

    let _ = toggle_hog_mode(runtime.device_id, "destack.audio.stream.close");
}
