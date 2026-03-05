use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::error::{PlatformErrorVm, vm as error_vm};
use crate::runtime::BindingCallContext;

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
    context: &mut vm::ExternalCallContext<'_>,
    error_id: u64,
) -> RuntimeResult<PlatformErrorVm> {
    error_vm::destack_error_take_platform_error(binding, context, error_id)
}
