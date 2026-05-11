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
pub(crate) fn destack_thread_local_create(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::ThreadLocalKey> {
    call_out(|out| unsafe { host_thread::destack_thread_local_create(binding, out) })
}

/// Delete one thread-local key.
pub(crate) fn destack_thread_local_delete(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    key: resource::ThreadLocalKey,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_local_delete(binding, key) }
}

/// Read one thread-local value.
pub(crate) fn destack_thread_local_get(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    key: resource::ThreadLocalKey,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { host_thread::destack_thread_local_get(binding, out, key) })
}

/// Store one thread-local value.
pub(crate) fn destack_thread_local_set(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    key: resource::ThreadLocalKey,
    argument_value: u64,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_local_set(binding, key, argument_value) }
}

/// Read thread CPU affinity.
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
pub(crate) fn destack_thread_get_priority(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::ThreadHandle,
) -> RuntimeResult<i32> {
    call_out(|out| unsafe { host_thread::destack_thread_get_priority(binding, out, handle) })
}

/// Set thread CPU affinity.
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
pub(crate) fn destack_thread_set_priority(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::ThreadHandle,
    priority: i32,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_set_priority(binding, handle, priority) }
}

/// Detach one host thread.
pub(crate) fn destack_thread_detach(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::ThreadHandle,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_detach(binding, handle) }
}

/// Join one host thread.
pub(crate) fn destack_thread_join(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::ThreadHandle,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { host_thread::destack_thread_join(binding, out, handle) })
}

/// Spawn one host thread.
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
pub(crate) fn destack_thread_address_wake_all(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_address_wake_all(binding, address) }
}

/// Wake one waiter on a memory address.
pub(crate) fn destack_thread_address_wake_one(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
) -> RuntimeResult<()> {
    unsafe { host_thread::destack_thread_address_wake_one(binding, address) }
}
