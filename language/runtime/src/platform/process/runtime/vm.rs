use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::process::{Signal, SignalEventVm};
use crate::platform::{VmArray, VmSlice};
use crate::runtime::RuntimeCallContext;

use crate::platform::process::vm as process_vm;
use crate::platform::resource;

/// Return the process argument vector.
///
/// Read the immutable argument list captured by the runtime at process startup.
/// Argument decoding and quoting semantics follow the host process loader.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses startup argument capture, not a dedicated syscall.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `process.run`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_args(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<vm::StringHandle>> {
    process_vm::destack_process_args(runtime, context)
}

/// Delete an environment variable by UTF-8 name.
///
/// Remove one key from the process environment block.
/// Missing keys are handled according to host environment semantics.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses unsetenv(3) on Unix and SetEnvironmentVariableW with null value on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `env.write`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_env_delete(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    process_vm::destack_process_env_delete(runtime, context, name)
}

/// Delete an environment variable by raw byte name.
///
/// Remove one key from the environment block without UTF-8 normalization.
/// This is intended for byte-level Unix-style environment access.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses unsetenv(3)-style byte keys on Unix and runtime transcoding on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `env.write`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_env_delete_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    name: VmSlice<u8>,
) -> RuntimeResult<()> {
    process_vm::destack_process_env_delete_bytes(runtime, context, name)
}

/// Read an environment variable by UTF-8 name.
///
/// Resolve one key from the process environment block and decode it as a runtime string.
/// Missing keys and invalid entries are surfaced as platform errors.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses getenv(3) on Unix and GetEnvironmentVariableW on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `env.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_env_get(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    name: vm::StringHandle,
) -> RuntimeResult<vm::StringHandle> {
    process_vm::destack_process_env_get(runtime, context, name)
}

/// Read an environment variable by raw byte name.
///
/// Resolve one key from the process environment block without UTF-8 normalization.
/// This is intended for byte-level Unix-style environment access.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses getenv(3)-style byte keys on Unix and runtime transcoding on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `env.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_env_get_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    name: VmSlice<u8>,
) -> RuntimeResult<VmArray<u8>> {
    process_vm::destack_process_env_get_bytes(runtime, context, name)
}

/// Set an environment variable by UTF-8 name and value.
///
/// Insert or replace one key-value pair in the process environment block.
/// Persistence and inheritance semantics follow host process-spawn rules.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses setenv(3) on Unix and SetEnvironmentVariableW on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `env.write`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_env_set(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    name: vm::StringHandle,
    argument_value: vm::StringHandle,
) -> RuntimeResult<()> {
    process_vm::destack_process_env_set(runtime, context, name, argument_value)
}

/// Set an environment variable by raw byte name and value.
///
/// Insert or replace one key-value pair in the environment block without UTF-8 normalization.
/// This is intended for byte-level Unix-style environment access.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses setenv(3)-style byte keys on Unix and runtime transcoding on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `env.write`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_env_set_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    name: VmSlice<u8>,
    argument_value: VmSlice<u8>,
) -> RuntimeResult<()> {
    process_vm::destack_process_env_set_bytes(runtime, context, name, argument_value)
}

/// Receive the next signal event from a subscription.
///
/// Wait for the next queued signal event for the subscription.
/// Delivery ordering and batching follow runtime and host signal queue semantics.
///
/// # Platform
/// Runtime-integrated on Unix and Windows targets.
/// Uses runtime subscription delivery with host signal waiting primitives.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_signal_receive(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SignalHandle,
) -> RuntimeResult<SignalEventVm> {
    process_vm::destack_process_signal_receive(runtime, context, handle)
}

/// Subscribe to one signal value.
///
/// Register one runtime subscription handle for signal delivery.
/// Subscription mode and coalescing behavior follow runtime and host integration rules.
///
/// # Platform
/// Runtime-integrated on Unix and Windows targets.
/// Uses runtime subscription state with host signal integration.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_signal_subscribe(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    signal: Signal,
) -> RuntimeResult<resource::SignalHandle> {
    process_vm::destack_process_signal_subscribe(runtime, context, signal)
}

/// Poll one signal event without blocking.
///
/// Read a queued signal event when available and return immediately otherwise.
/// Empty queue behavior is reported through host-specific not-ready errors.
///
/// # Platform
/// Runtime-integrated on Unix and Windows targets.
/// Uses runtime subscription polling with nonblocking host signal probes.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_signal_try_receive(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SignalHandle,
) -> RuntimeResult<SignalEventVm> {
    process_vm::destack_process_signal_try_receive(runtime, context, handle)
}

/// Remove a signal subscription handle.
///
/// Unregister one signal subscription from runtime delivery.
/// Pending events may still be readable depending on host queueing behavior.
///
/// # Platform
/// Runtime-integrated on Unix and Windows targets.
/// Uses runtime subscription teardown with host signal integration.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_signal_unsubscribe(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SignalHandle,
) -> RuntimeResult<()> {
    process_vm::destack_process_signal_unsubscribe(runtime, context, handle)
}
