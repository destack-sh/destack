use crate::diagnostic::RuntimeStatus;
use crate::host::android::abi::ingress::destack_host_android_notify_activity_lifecycle as runtime_notify_activity_lifecycle;

/// Forward the Android activity lifecycle ingress through the ABI artifact.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_notify_activity_lifecycle(
    runtime_id: u64,
    lifecycle_code: u32,
) -> RuntimeStatus {
    unsafe { runtime_notify_activity_lifecycle(runtime_id, lifecycle_code) }
}
