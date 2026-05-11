use crate::diagnostic::{DiagnosticId, RuntimeResult};
use crate::platform::error::PlatformErrorVm;
use crate::platform::error::core::{VmStringStore, platform_error_vm, take_platform_error};
use crate::runtime::BindingCallContext;
use destack_vm;

/// Take a runtime platform error by id.
pub(crate) fn destack_error_take_platform_error(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    error_id: u64,
) -> RuntimeResult<PlatformErrorVm> {
    let error = take_platform_error(
        binding.worker().diagnostics.as_ref(),
        DiagnosticId::from_raw(error_id),
    );
    let mut store = VmStringStore::new(context);
    let platform_error = platform_error_vm(&mut store, &error)?;

    Ok(platform_error)
}
