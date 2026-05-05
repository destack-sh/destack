use crate::diagnostic::RuntimeResult;
use crate::platform::core::{call_out, store_values_array_from_vm, values_array_to_vm};
use crate::platform::resource;
use crate::platform::thread::{
    ThreadCpu, ThreadCpuSet, ThreadCpuSetVm, ThreadOptions, ThreadOptionsVm, host as host_thread,
};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// Convert one VM thread-options aggregate into the native ABI value.
fn thread_options_from_vm(options: ThreadOptionsVm) -> ThreadOptions {
    ThreadOptions {
        stack_bytes: options.stack_bytes,
        flags: options.flags,
    }
}

/// Create one thread-local key.
///
/// Allocate one runtime thread-local storage key.
/// Key lifetime is explicit and must be released with delete.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread TLS keys on Unix and TlsAlloc on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.local`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_thread_local_create(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::ThreadLocalKey> {
    call_out(|out| unsafe { host_thread::destack_thread_local_create(binding, out) })
}

/// Delete one thread-local key.
///
/// Release one thread-local key and associated host resources.
/// Existing per-thread values become invalid after deletion.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread TLS key deletion on Unix and TlsFree on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.local`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_thread_local_delete(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    key: resource::ThreadLocalKey,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_local_delete(binding, key) }
}

/// Read one thread-local value.
///
/// Read one machine-word value from one thread-local key.
/// Value interpretation is caller-defined and ABI-dependent.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread TLS storage on Unix and TlsGetValue on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.local`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_thread_local_get(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    key: resource::ThreadLocalKey,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { host_thread::destack_thread_local_get(binding, out, key) })
}

/// Store one thread-local value.
///
/// Write one machine-word value into one thread-local key.
/// Value interpretation is caller-defined and ABI-dependent.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread TLS storage on Unix and TlsSetValue on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.local`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_thread_local_set(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    key: resource::ThreadLocalKey,
    argument_value: u64,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_local_set(binding, key, argument_value) }
}

/// Read thread CPU affinity.
///
/// Read one thread logical-processor affinity set.
/// Unix targets always report group `0`.
/// Windows reports group-local logical processors for the active thread affinity.
///
/// # Platform
/// Unix and Windows.
/// Uses sched affinity APIs on Unix and GetThreadGroupAffinity on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `thread.sched`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_thread_get_affinity(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::ThreadHandle,
) -> RuntimeResult<ThreadCpuSetVm> {
    let cpus =
        call_out(|out| unsafe { host_thread::destack_thread_get_affinity(binding, out, handle) })?;
    let cpus = values_array_to_vm::<ThreadCpu>(context, cpus.cpus)?;

    Ok(ThreadCpuSetVm { cpus })
}

/// Read thread priority.
///
/// Read one thread priority value from host scheduler state.
/// Priority value normalization is runtime-defined per host.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread scheduling APIs on Unix and GetThreadPriority on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `thread.sched`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_thread_get_priority(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::ThreadHandle,
) -> RuntimeResult<i32> {
    call_out(|out| unsafe { host_thread::destack_thread_get_priority(binding, out, handle) })
}

/// Set thread CPU affinity.
///
/// Bind one thread to one set of logical processors.
/// Unix targets interpret every entry with group `0`.
/// Windows maps entries to processor groups and group-local logical processors.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread affinity APIs on Unix and SetThreadGroupAffinity on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `thread.sched`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_thread_set_affinity(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::ThreadHandle,
    cpus: ThreadCpuSetVm,
) -> RuntimeResult<()> {
    let cpus = store_values_array_from_vm::<ThreadCpu>(binding, context, cpus.cpus)?;
    let cpus = ThreadCpuSet { cpus };

    unsafe { host_thread::destack_thread_set_affinity(binding, handle, cpus) }
}

/// Set thread priority.
///
/// Set one thread priority value using host scheduler controls.
/// Priority range and interpretation are host-specific.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread scheduling APIs on Unix and SetThreadPriority on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `thread.sched`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_thread_set_priority(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::ThreadHandle,
    priority: i32,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_set_priority(binding, handle, priority) }
}

/// Detach one host thread.
///
/// Detach one thread from join tracking.
/// Detached thread lifecycle and cleanup are host-managed.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread_detach on Unix and handle-release semantics on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.spawn`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_thread_detach(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::ThreadHandle,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_detach(binding, handle) }
}

/// Join one host thread.
///
/// Wait for one joinable thread to exit and return its machine-word result.
/// Join lifecycle follows host thread rules, but the returned value is runtime-defined.
///
/// # Platform
/// Unix and Windows.
/// Uses one runtime-managed completion slot on supported hosts.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.spawn`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_thread_join(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::ThreadHandle,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { host_thread::destack_thread_join(binding, out, handle) })
}

/// Spawn one host thread.
///
/// Spawn one host thread that enters one runtime-provided thread entry handle.
/// Thread entry creation and argument interpretation are runtime ABI contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread_create on Unix and CreateThread on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.spawn`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_thread_spawn(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    entry: resource::ThreadEntryHandle,
    argument: u64,
    options: ThreadOptionsVm,
) -> RuntimeResult<resource::ThreadHandle> {
    let options = thread_options_from_vm(options);

    call_out(|out| unsafe {
        host_thread::destack_thread_spawn(binding, out, entry, argument, options)
    })
}

/// Wait on one memory address value.
///
/// Wait while the target memory word matches the expected value.
/// Address wait semantics follow host futex or WaitOnAddress primitives.
///
/// # Platform
/// Unix and Windows.
/// Uses futex wait on Linux and WaitOnAddress on Windows.
///
/// # Errors
/// Returns invalidArgument, ioTimedOut, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.wait`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_thread_address_wait(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    expected: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_address_wait(binding, address, expected, timeoutns) }
}

/// Wake all waiters on a memory address.
///
/// Wake all waiters blocked on the target memory address.
/// Wake ordering follows host wait-address primitive behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses futex wake on Linux and WakeByAddressAll on Windows.
///
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.wait`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_thread_address_wake_all(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_address_wake_all(binding, address) }
}

/// Wake one waiter on a memory address.
///
/// Wake one waiter blocked on the target memory address.
/// Wake ordering follows host wait-address primitive behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses futex wake on Linux and WakeByAddressSingle on Windows.
///
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.wait`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_thread_address_wake_one(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_address_wake_one(binding, address) }
}
