use crate::diagnostic::{RuntimeResult, RuntimeStatus};
use crate::host::core::error::invalid_argument_value;
use crate::host::ios::ingress::{IosApplicationLifecycle, ios_notify_application_lifecycle};

use super::core::runtime_status;

/// iOS lifecycle code for `applicationDidFinishLaunching`.
pub(crate) const IOS_LIFECYCLE_DID_FINISH_LAUNCHING: u32 = 0;
/// iOS lifecycle code for `applicationDidBecomeActive`.
pub(crate) const IOS_LIFECYCLE_DID_BECOME_ACTIVE: u32 = 1;
/// iOS lifecycle code for `applicationWillResignActive`.
pub(crate) const IOS_LIFECYCLE_WILL_RESIGN_ACTIVE: u32 = 2;
/// iOS lifecycle code for `applicationDidEnterBackground`.
pub(crate) const IOS_LIFECYCLE_DID_ENTER_BACKGROUND: u32 = 3;
/// iOS lifecycle code for `applicationWillEnterForeground`.
pub(crate) const IOS_LIFECYCLE_WILL_ENTER_FOREGROUND: u32 = 4;
/// iOS lifecycle code for `applicationWillTerminate`.
pub(crate) const IOS_LIFECYCLE_WILL_TERMINATE: u32 = 5;

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle_code: u32,
) -> RuntimeStatus {
    let result = decode_ios_application_lifecycle(lifecycle_code)
        .and_then(|lifecycle| ios_notify_application_lifecycle(runtime_id, lifecycle));

    runtime_status(result)
}

/// Decode one iOS lifecycle code.
pub(crate) fn decode_ios_application_lifecycle(
    lifecycle_code: u32,
) -> RuntimeResult<IosApplicationLifecycle> {
    let lifecycle = match lifecycle_code {
        IOS_LIFECYCLE_DID_FINISH_LAUNCHING => IosApplicationLifecycle::DidFinishLaunching,
        IOS_LIFECYCLE_DID_BECOME_ACTIVE => IosApplicationLifecycle::DidBecomeActive,
        IOS_LIFECYCLE_WILL_RESIGN_ACTIVE => IosApplicationLifecycle::WillResignActive,
        IOS_LIFECYCLE_DID_ENTER_BACKGROUND => IosApplicationLifecycle::DidEnterBackground,
        IOS_LIFECYCLE_WILL_ENTER_FOREGROUND => IosApplicationLifecycle::WillEnterForeground,
        IOS_LIFECYCLE_WILL_TERMINATE => IosApplicationLifecycle::WillTerminate,
        _ => {
            return Err(invalid_argument_value(
                "lifecycle_code",
                "invalid ios lifecycle code",
            ));
        }
    };

    Ok(lifecycle)
}
