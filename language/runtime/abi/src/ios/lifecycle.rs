use crate::diagnostic::RuntimeStatus;
use crate::host::apple::abi::ingress::destack_host_ios_notify_application_lifecycle as runtime_notify_application_lifecycle;

/// Forward the Apple application lifecycle ingress through the ABI artifact.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_ios_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle_code: u32,
) -> RuntimeStatus {
    unsafe { runtime_notify_application_lifecycle(runtime_id, lifecycle_code) }
}
