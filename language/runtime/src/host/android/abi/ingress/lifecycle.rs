use crate::diagnostic::{RuntimeResult, RuntimeStatus};
use crate::host::core::error::invalid_argument_value;
use crate::host::os::android::ingress::{
    AndroidActivityLifecycle, android_notify_activity_lifecycle,
};

use super::core::runtime_status;

/// Android lifecycle code for `onCreate`.
pub(crate) const ANDROID_LIFECYCLE_CREATED: u32 = 0;
/// Android lifecycle code for `onStart`.
pub(crate) const ANDROID_LIFECYCLE_STARTED: u32 = 1;
/// Android lifecycle code for `onResume`.
pub(crate) const ANDROID_LIFECYCLE_RESUMED: u32 = 2;
/// Android lifecycle code for `onPause`.
pub(crate) const ANDROID_LIFECYCLE_PAUSED: u32 = 3;
/// Android lifecycle code for `onStop`.
pub(crate) const ANDROID_LIFECYCLE_STOPPED: u32 = 4;
/// Android lifecycle code for `onDestroy`.
pub(crate) const ANDROID_LIFECYCLE_DESTROYED: u32 = 5;

pub unsafe fn destack_host_android_notify_activity_lifecycle(
    runtime_id: u64,
    lifecycle_code: u32,
) -> RuntimeStatus {
    let result = decode_android_activity_lifecycle(lifecycle_code)
        .and_then(|lifecycle| android_notify_activity_lifecycle(runtime_id, lifecycle));

    runtime_status(result)
}

/// Decode one Android lifecycle code.
pub(crate) fn decode_android_activity_lifecycle(
    lifecycle_code: u32,
) -> RuntimeResult<AndroidActivityLifecycle> {
    let lifecycle = match lifecycle_code {
        ANDROID_LIFECYCLE_CREATED => AndroidActivityLifecycle::Created,
        ANDROID_LIFECYCLE_STARTED => AndroidActivityLifecycle::Started,
        ANDROID_LIFECYCLE_RESUMED => AndroidActivityLifecycle::Resumed,
        ANDROID_LIFECYCLE_PAUSED => AndroidActivityLifecycle::Paused,
        ANDROID_LIFECYCLE_STOPPED => AndroidActivityLifecycle::Stopped,
        ANDROID_LIFECYCLE_DESTROYED => AndroidActivityLifecycle::Destroyed,
        _ => {
            return Err(invalid_argument_value(
                "lifecycle_code",
                "invalid android lifecycle code",
            ));
        }
    };

    Ok(lifecycle)
}
