use crate::diagnostic::{RuntimeErrorId, RuntimeResult};
use crate::platform::error::PlatformErrorVm;
use crate::platform::error::core::{VmStringStore, platform_error_vm, take_platform_error};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Take a runtime platform error by id.
pub(super) fn destack_error_take_platform_error(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    error_id: u64,
) -> RuntimeResult<PlatformErrorVm> {
    let error = take_platform_error(
        &runtime.runtime().errors,
        RuntimeErrorId::from_raw(error_id),
    );
    let mut store = VmStringStore::new(context);
    let platform_error = platform_error_vm(&mut store, &error);

    Ok(platform_error)
}
