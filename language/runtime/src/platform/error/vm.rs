use crate::diagnostic::{DiagnosticId, RuntimeResult};
use crate::platform::error::PlatformErrorVm;
use crate::platform::error::core::{VmStringStore, platform_error_vm, take_platform_error};
use crate::runtime::BindingCallContext;
use destack_vm;

/// Take a runtime platform error by id.
///
/// Reads and consumes one stored platform error entry by id.
/// Removes the entry from the runtime error store after a successful read.
///
/// # Platform
/// Managed by the runtime platform layer rather than an OS-specific syscall.
/// Uses no direct syscall: reads and clears one runtime error-store entry.
///
/// # Errors
/// Returns `invalidArgument` or `generic`.
///
/// # Security
/// Requires `diagnostic.read`.
///
/// # Replay
/// Deterministic.
pub(crate) fn destack_error_take_platform_error(
    binding: &BindingCallContext,
    context: &mut destack_vm::ExternalCallContext<'_>,
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
