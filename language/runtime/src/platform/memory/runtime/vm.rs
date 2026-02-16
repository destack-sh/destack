use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::memory::vm as memory_vm;
use crate::runtime::RuntimeCallContext;

/// Set runtime W^X policy.
///
/// Enable or disable runtime write-xor-execute policy enforcement.
/// Policy update affects subsequent executable-memory transitions.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime memory policy controls layered over host page protections.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `memory.execute`.
///
/// # Replay
/// Deterministic.
pub(crate) fn destack_memory_set_write_xor_execute(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    enabled: bool,
) -> RuntimeResult<()> {
    memory_vm::destack_memory_set_write_xor_execute(runtime, context, enabled)
}
