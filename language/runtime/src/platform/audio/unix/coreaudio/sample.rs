#[cfg(target_os = "macos")]
use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;

#[cfg(target_os = "macos")]
use super::abi::AudioDeviceID;
#[cfg(target_os = "macos")]
use super::constants::{K_AUDIO_DEVICE_PROPERTY_DEVICE_UID, K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL};
#[cfg(target_os = "macos")]
use super::property::{device_ids, get_cfstring_optional};

/// Resolve a CoreAudio device identifier from a stable runtime device id.
#[cfg(target_os = "macos")]
pub(super) fn device_id_from_stable_id(stable_id: &str) -> RuntimeResult<AudioDeviceID> {
    let device_ids = device_ids()?;
    for device_id in device_ids {
        let uid = get_cfstring_optional(
            device_id,
            K_AUDIO_DEVICE_PROPERTY_DEVICE_UID,
            K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        );
        let Some(uid) = uid else {
            continue;
        };
        if format!("coreaudio:{uid}") == stable_id {
            return Ok(device_id);
        }
    }

    Err(core_platform::io_not_found(
        "destack.audio.stream.open",
        format!("coreaudio device id not found: {stable_id}"),
    ))
}
