use std::time::{SystemTime, UNIX_EPOCH};

use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::{
    HostIdentity, HostIdentityVm, LoadAverage, LoadAverageVm, SystemSnapshot, SystemSnapshotVm,
};

use super::{HarnessValue, OsHarnessContext};

/// Return one raw VM context pointer when this harness run uses VM bindings.
fn vm_context_pointer(context: &OsHarnessContext<'_>) -> Option<*mut ()> {
    context.vm_context
}

/// Assert one platform error code from one runtime error payload.
pub(super) fn assert_platform_error_code(error: &RuntimeError, code: PlatformErrorCode) {
    let platform_error = error
        .platform_error()
        .expect("expected one platform error payload");
    assert_eq!(platform_error.code, code);
}

/// Return current unix time in nanoseconds.
pub(super) fn now_unix_ns() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch");

    u64::try_from(now.as_nanos()).expect("system time nanoseconds should fit in u64")
}

/// Decode one host-identity harness value into owned fields.
pub(super) fn decode_host_identity_value(
    context: &mut OsHarnessContext<'_>,
    value: HarnessValue<HostIdentity, HostIdentityVm>,
) -> RuntimeResult<(String, String, String, String)> {
    match value {
        HarnessValue::Native(value) => {
            let hostname = unsafe { value.hostname.as_str() }?.to_string();
            let kernel = unsafe { value.kernel.as_str() }?.to_string();
            let release = unsafe { value.release.as_str() }?.to_string();
            let architecture = unsafe { value.architecture.as_str() }?.to_string();

            Ok((hostname, kernel, release, architecture))
        }
        HarnessValue::Vm(value) => {
            let vm_context = vm_context_pointer(context).expect("vm context should be available");
            let vm_context = unsafe { &mut *(vm_context as *mut vm::ExternalCallContext<'_>) };
            let hostname = vm_context
                .string_ref(value.hostname)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string();
            let kernel = vm_context
                .string_ref(value.kernel)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string();
            let release = vm_context
                .string_ref(value.release)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string();
            let architecture = vm_context
                .string_ref(value.architecture)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string();

            Ok((hostname, kernel, release, architecture))
        }
    }
}

/// Decode one load-average harness value.
pub(super) fn decode_load_average_value(
    value: HarnessValue<LoadAverage, LoadAverageVm>,
) -> LoadAverage {
    match value {
        HarnessValue::Native(value) => value,
        HarnessValue::Vm(value) => value,
    }
}

/// Decode one system-snapshot harness value.
pub(super) fn decode_system_snapshot_value(
    value: HarnessValue<SystemSnapshot, SystemSnapshotVm>,
) -> SystemSnapshot {
    match value {
        HarnessValue::Native(value) => value,
        HarnessValue::Vm(value) => value,
    }
}
