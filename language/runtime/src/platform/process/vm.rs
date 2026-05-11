use destack_vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::core::{
    bytes_array_to_vm, call_out, intern_string_to_vm as string_ref_to_vm,
    os_path_to_vm as path_ref_to_vm, store_bytes_array_from_vm,
    store_bytes_from_vm as bytes_slice_from_vm, store_os_path_from_vm as path_ref_from_vm,
    store_string_from_vm as string_ref_from_vm, store_string_slice_from_vm as string_slice_from_vm,
    store_values_array_from_vm, store_values_from_vm, string_slice_to_vm, values_array_to_vm,
    values_to_vm,
};
use crate::platform::process::{
    ExecAtFlags, GroupId, ProcessCpuSet, ProcessCpuSetVm, ProcessFdAction, ProcessFdActionClose,
    ProcessFdActionDup2, ProcessFdActionOpen, ProcessFdActionVm, ProcessFdFlags,
    ProcessFdSignalFlags, ProcessGroupIdsVm, ProcessId, ProcessLimitResource, ProcessLimitVm,
    ProcessNamespaceKind, ProcessSchedulerConfigVm, ProcessSpawnOptions, ProcessSpawnOptionsVm,
    ProcessStdio, ProcessStdioDescriptor, ProcessStdioFile, ProcessStdioInherit, ProcessStdioNull,
    ProcessStdioPipe, ProcessStdioVm, ProcessUnshareFlags, ProcessUserIdsVm,
    ProcessWaitContinuedStatusVm, ProcessWaitExitedStatusVm, ProcessWaitFlags,
    ProcessWaitRunningStatusVm, ProcessWaitSignaledStatusVm, ProcessWaitStatus,
    ProcessWaitStatusVm, ProcessWaitStoppedStatusVm, Signal, SignalEventVm, SignalFdFlags,
    SignalMaskHow, SyscallFilterFlags, UserId, host as host_process,
};
use crate::platform::{VmAggregateCodec, VmArray, VmSlice, fs, resource};
use crate::runtime::BindingCallContext;

fn process_spawn_options_from_vm(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    options: ProcessSpawnOptionsVm,
) -> RuntimeResult<ProcessSpawnOptions> {
    let cwd = path_ref_from_vm(binding, context, options.cwd)?;
    Ok(ProcessSpawnOptions {
        cwd,
        detached: options.detached,
        reset_signals: options.reset_signals,
        new_process_group: options.new_process_group,
    })
}

fn process_stdio_from_vm(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    value: ProcessStdioVm,
) -> RuntimeResult<ProcessStdio> {
    match value {
        ProcessStdioVm::ProcessStdioDescriptor(descriptor) => {
            let kind = string_ref_from_vm(binding, context, descriptor.kind)?;
            Ok(ProcessStdio::ProcessStdioDescriptor(
                ProcessStdioDescriptor {
                    kind,
                    descriptor: descriptor.descriptor,
                },
            ))
        }
        ProcessStdioVm::ProcessStdioFile(file) => {
            let kind = string_ref_from_vm(binding, context, file.kind)?;
            Ok(ProcessStdio::ProcessStdioFile(ProcessStdioFile {
                kind,
                file: file.file,
            }))
        }
        ProcessStdioVm::ProcessStdioInherit(inherit) => {
            let kind = string_ref_from_vm(binding, context, inherit.kind)?;
            Ok(ProcessStdio::ProcessStdioInherit(ProcessStdioInherit {
                kind,
            }))
        }
        ProcessStdioVm::ProcessStdioNull(null_value) => {
            let kind = string_ref_from_vm(binding, context, null_value.kind)?;
            Ok(ProcessStdio::ProcessStdioNull(ProcessStdioNull { kind }))
        }
        ProcessStdioVm::ProcessStdioPipe(pipe) => {
            let kind = string_ref_from_vm(binding, context, pipe.kind)?;
            Ok(ProcessStdio::ProcessStdioPipe(ProcessStdioPipe {
                kind,
                pipe: pipe.pipe,
            }))
        }
    }
}

fn process_fd_action_from_vm(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    value: ProcessFdActionVm,
) -> RuntimeResult<ProcessFdAction> {
    match value {
        ProcessFdActionVm::ProcessFdActionClose(close) => {
            let kind = string_ref_from_vm(binding, context, close.kind)?;
            Ok(ProcessFdAction::ProcessFdActionClose(
                ProcessFdActionClose {
                    kind,
                    descriptor: close.descriptor,
                },
            ))
        }
        ProcessFdActionVm::ProcessFdActionDup2(dup2) => {
            let kind = string_ref_from_vm(binding, context, dup2.kind)?;
            Ok(ProcessFdAction::ProcessFdActionDup2(ProcessFdActionDup2 {
                kind,
                source: dup2.source,
                target: dup2.target,
            }))
        }
        ProcessFdActionVm::ProcessFdActionOpen(open) => {
            let kind = string_ref_from_vm(binding, context, open.kind)?;
            let path = path_ref_from_vm(binding, context, open.path)?;
            Ok(ProcessFdAction::ProcessFdActionOpen(ProcessFdActionOpen {
                kind,
                target: open.target,
                path,
                flags: open.flags,
                mode: open.mode,
            }))
        }
    }
}

fn process_stdio_slice_from_vm(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    stdio: VmSlice<ProcessStdioVm>,
) -> RuntimeResult<NativeSlice<ProcessStdio>> {
    if stdio.len == 0 {
        return Ok(binding.store_slice(Vec::new()));
    }

    let values = stdio.values(&context.read())?;
    let mut native_values = Vec::with_capacity(values.len());
    for value in values {
        let value =
            <ProcessStdioVm as VmAggregateCodec>::decode_with_context(&context.read(), value)?;
        native_values.push(process_stdio_from_vm(binding, context, value)?);
    }

    Ok(binding.store_slice(native_values))
}

fn process_fd_action_slice_from_vm(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    actions: VmSlice<ProcessFdActionVm>,
) -> RuntimeResult<NativeSlice<ProcessFdAction>> {
    if actions.len == 0 {
        return Ok(binding.store_slice(Vec::new()));
    }

    let values = actions.values(&context.read())?;
    let mut native_values = Vec::with_capacity(values.len());
    for value in values {
        let value =
            <ProcessFdActionVm as VmAggregateCodec>::decode_with_context(&context.read(), value)?;
        native_values.push(process_fd_action_from_vm(binding, context, value)?);
    }

    Ok(binding.store_slice(native_values))
}

fn process_wait_status_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    value: ProcessWaitStatus,
) -> RuntimeResult<ProcessWaitStatusVm> {
    match value {
        ProcessWaitStatus::ProcessWaitContinuedStatus(status) => {
            let kind = string_ref_to_vm(context, status.kind)?;
            Ok(ProcessWaitStatusVm::ProcessWaitContinuedStatus(
                ProcessWaitContinuedStatusVm {
                    kind,
                    pid: status.pid,
                },
            ))
        }
        ProcessWaitStatus::ProcessWaitExitedStatus(status) => {
            let kind = string_ref_to_vm(context, status.kind)?;
            Ok(ProcessWaitStatusVm::ProcessWaitExitedStatus(
                ProcessWaitExitedStatusVm {
                    kind,
                    pid: status.pid,
                    exit_code: status.exit_code,
                },
            ))
        }
        ProcessWaitStatus::ProcessWaitRunningStatus(status) => {
            let kind = string_ref_to_vm(context, status.kind)?;
            Ok(ProcessWaitStatusVm::ProcessWaitRunningStatus(
                ProcessWaitRunningStatusVm {
                    kind,
                    pid: status.pid,
                },
            ))
        }
        ProcessWaitStatus::ProcessWaitSignaledStatus(status) => {
            let kind = string_ref_to_vm(context, status.kind)?;
            Ok(ProcessWaitStatusVm::ProcessWaitSignaledStatus(
                ProcessWaitSignaledStatusVm {
                    kind,
                    pid: status.pid,
                    signal: status.signal,
                    core_dumped: status.core_dumped,
                },
            ))
        }
        ProcessWaitStatus::ProcessWaitStoppedStatus(status) => {
            let kind = string_ref_to_vm(context, status.kind)?;
            Ok(ProcessWaitStatusVm::ProcessWaitStoppedStatus(
                ProcessWaitStoppedStatusVm {
                    kind,
                    pid: status.pid,
                    signal: status.signal,
                },
            ))
        }
    }
}

/// Return the process argument vector.
pub(crate) fn destack_process_args(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<destack_vm::StringHandle>> {
    let values = call_out(|out| unsafe { host_process::destack_process_args(binding, out) })?;
    string_slice_to_vm(context, values)
}

/// Change the current working directory.
pub(crate) fn destack_process_chdir(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: fs::OsPathVm,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_process::destack_process_chdir(binding, path) }
}

/// Return the current working directory.
pub(crate) fn destack_process_cwd(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<fs::OsPathVm> {
    let path = call_out(|out| unsafe { host_process::destack_process_cwd(binding, out) })?;
    path_ref_to_vm(context, path)
}

/// Delete an environment variable by UTF-8 name.
pub(crate) fn destack_process_env_delete(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    name: destack_vm::StringHandle,
) -> RuntimeResult<()> {
    let name = string_ref_from_vm(binding, context, name)?;
    unsafe { host_process::destack_process_env_delete(binding, name) }
}

/// Delete an environment variable by raw byte name.
pub(crate) fn destack_process_env_delete_bytes(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    name: VmSlice<u8>,
) -> RuntimeResult<()> {
    let name = bytes_slice_from_vm(binding, context, name)?;
    unsafe { host_process::destack_process_env_delete_bytes(binding, name) }
}

/// Read an environment variable by UTF-8 name.
pub(crate) fn destack_process_env_get(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    name: destack_vm::StringHandle,
) -> RuntimeResult<destack_vm::StringHandle> {
    let name = string_ref_from_vm(binding, context, name)?;
    let value =
        call_out(|out| unsafe { host_process::destack_process_env_get(binding, out, name) })?;
    string_ref_to_vm(context, value)
}

/// Read an environment variable by raw byte name.
pub(crate) fn destack_process_env_get_bytes(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    name: VmSlice<u8>,
) -> RuntimeResult<VmArray<u8>> {
    let name = bytes_slice_from_vm(binding, context, name)?;
    let value =
        call_out(|out| unsafe { host_process::destack_process_env_get_bytes(binding, out, name) })?;
    bytes_array_to_vm(context, value)
}

/// Set an environment variable by UTF-8 name and value.
pub(crate) fn destack_process_env_set(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    name: destack_vm::StringHandle,
    argument_value: destack_vm::StringHandle,
) -> RuntimeResult<()> {
    let name = string_ref_from_vm(binding, context, name)?;
    let argument_value = string_ref_from_vm(binding, context, argument_value)?;
    unsafe { host_process::destack_process_env_set(binding, name, argument_value) }
}

/// Set an environment variable by raw byte name and value.
pub(crate) fn destack_process_env_set_bytes(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    name: VmSlice<u8>,
    argument_value: VmSlice<u8>,
) -> RuntimeResult<()> {
    let name = bytes_slice_from_vm(binding, context, name)?;
    let argument_value = bytes_slice_from_vm(binding, context, argument_value)?;
    unsafe { host_process::destack_process_env_set_bytes(binding, name, argument_value) }
}

/// Replace the current process image with a command path.
pub(crate) fn destack_process_exec(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    command: fs::OsPathVm,
    arguments: VmSlice<destack_vm::StringHandle>,
    environment: VmSlice<destack_vm::StringHandle>,
) -> RuntimeResult<()> {
    let command = path_ref_from_vm(binding, context, command)?;
    let arguments = string_slice_from_vm(binding, context, arguments)?;
    let environment = string_slice_from_vm(binding, context, environment)?;
    unsafe { host_process::destack_process_exec(binding, command, arguments, environment) }
}

/// Replace the current process image using a directory-relative path.
pub(crate) fn destack_process_execat(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    directory: resource::DirectoryHandle,
    path: fs::OsPathVm,
    arguments: VmSlice<destack_vm::StringHandle>,
    environment: VmSlice<destack_vm::StringHandle>,
    flags: ExecAtFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    let arguments = string_slice_from_vm(binding, context, arguments)?;
    let environment = string_slice_from_vm(binding, context, environment)?;
    unsafe {
        host_process::destack_process_execat(
            binding,
            directory,
            path,
            arguments,
            environment,
            flags,
        )
    }
}

/// Replace the current process image using an executable file handle.
pub(crate) fn destack_process_fexec(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    executable: resource::FileHandle,
    arguments: VmSlice<destack_vm::StringHandle>,
    environment: VmSlice<destack_vm::StringHandle>,
) -> RuntimeResult<()> {
    let arguments = string_slice_from_vm(binding, context, arguments)?;
    let environment = string_slice_from_vm(binding, context, environment)?;
    unsafe { host_process::destack_process_fexec(binding, executable, arguments, environment) }
}

/// Exit the current process with the given code.
pub(crate) fn destack_process_exit(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    code: u32,
) -> RuntimeResult<()> {
    unsafe { host_process::destack_process_exit(binding, code) }
}

/// Open one standard input stream handle.
pub(crate) fn destack_process_stdio_stdin(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<resource::FileHandle> {
    call_out(|out| unsafe { host_process::destack_process_stdio_stdin(binding, out) })
}

/// Open one standard output stream handle.
pub(crate) fn destack_process_stdio_stdout(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<resource::FileHandle> {
    call_out(|out| unsafe { host_process::destack_process_stdio_stdout(binding, out) })
}

/// Open one standard error stream handle.
pub(crate) fn destack_process_stdio_stderr(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<resource::FileHandle> {
    call_out(|out| unsafe { host_process::destack_process_stdio_stderr(binding, out) })
}

/// Close one process descriptor.
pub(crate) fn destack_process_process_fd_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::ProcessFdHandle,
) -> RuntimeResult<()> {
    unsafe { host_process::destack_process_process_fd_close(binding, handle) }
}

/// Open one process descriptor for the target process id.
pub(crate) fn destack_process_process_fd_open(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    pid: ProcessId,
    flags: ProcessFdFlags,
) -> RuntimeResult<resource::ProcessFdHandle> {
    call_out(|out| unsafe {
        host_process::destack_process_process_fd_open(binding, out, pid, flags)
    })
}

/// Send one signal through a process descriptor.
pub(crate) fn destack_process_process_fd_send_signal(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::ProcessFdHandle,
    signal: Signal,
    flags: ProcessFdSignalFlags,
) -> RuntimeResult<()> {
    unsafe { host_process::destack_process_process_fd_send_signal(binding, handle, signal, flags) }
}

/// Poll one process descriptor state transition without blocking.
pub(crate) fn destack_process_process_fd_try_wait(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::ProcessFdHandle,
) -> RuntimeResult<ProcessWaitStatusVm> {
    let value = call_out(|out| unsafe {
        host_process::destack_process_process_fd_try_wait(binding, out, handle)
    })?;
    process_wait_status_to_vm(context, value)
}

/// Wait for one process descriptor state transition.
pub(crate) fn destack_process_process_fd_wait(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::ProcessFdHandle,
    timeoutns: u64,
) -> RuntimeResult<ProcessWaitStatusVm> {
    let value = call_out(|out| unsafe {
        host_process::destack_process_process_fd_wait(binding, out, handle, timeoutns)
    })?;
    process_wait_status_to_vm(context, value)
}

/// Close one signal descriptor.
pub(crate) fn destack_process_signal_fd_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<()> {
    unsafe { host_process::destack_process_signal_fd_close(binding, handle) }
}

/// Open one signal descriptor for the provided signal mask.
pub(crate) fn destack_process_signal_fd_open(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    signals: VmSlice<Signal>,
    flags: SignalFdFlags,
) -> RuntimeResult<resource::SignalFdHandle> {
    let signals = store_values_from_vm(binding, context, signals)?;
    call_out(|out| unsafe {
        host_process::destack_process_signal_fd_open(binding, out, signals, flags)
    })
}

/// Read one queued signal event from a signal descriptor.
pub(crate) fn destack_process_signal_fd_read(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<SignalEventVm> {
    call_out(|out| unsafe { host_process::destack_process_signal_fd_read(binding, out, handle) })
}

/// Replace the active signal mask for one signal descriptor.
pub(crate) fn destack_process_signal_fd_set_mask(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::SignalFdHandle,
    signals: VmSlice<Signal>,
) -> RuntimeResult<()> {
    let signals = store_values_from_vm(binding, context, signals)?;
    unsafe { host_process::destack_process_signal_fd_set_mask(binding, handle, signals) }
}

/// Poll one queued signal event from a signal descriptor without blocking.
pub(crate) fn destack_process_signal_fd_try_read(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<SignalEventVm> {
    call_out(|out| unsafe {
        host_process::destack_process_signal_fd_try_read(binding, out, handle)
    })
}

/// Read one control-group resource limit.
pub(crate) fn destack_process_cgroup_get_limit(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: destack_vm::StringHandle,
    resource: ProcessLimitResource,
) -> RuntimeResult<ProcessLimitVm> {
    let path = string_ref_from_vm(binding, context, path)?;
    call_out(|out| unsafe {
        host_process::destack_process_cgroup_get_limit(binding, out, path, resource)
    })
}

/// Join one control group.
pub(crate) fn destack_process_cgroup_join(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: destack_vm::StringHandle,
) -> RuntimeResult<()> {
    let path = string_ref_from_vm(binding, context, path)?;
    unsafe { host_process::destack_process_cgroup_join(binding, path) }
}

/// Write one control-group resource limit.
pub(crate) fn destack_process_cgroup_set_limit(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: destack_vm::StringHandle,
    resource: ProcessLimitResource,
    limit: ProcessLimitVm,
) -> RuntimeResult<()> {
    let path = string_ref_from_vm(binding, context, path)?;
    unsafe { host_process::destack_process_cgroup_set_limit(binding, path, resource, limit) }
}

/// Assign processes to one Windows job object.
pub(crate) fn destack_process_job_assign(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    name: destack_vm::StringHandle,
    pids: VmSlice<ProcessId>,
) -> RuntimeResult<()> {
    let name = string_ref_from_vm(binding, context, name)?;
    let pids = store_values_from_vm(binding, context, pids)?;
    unsafe { host_process::destack_process_job_assign(binding, name, pids) }
}

/// Set one Windows job object resource limit.
pub(crate) fn destack_process_job_set_limit(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    name: destack_vm::StringHandle,
    resource: ProcessLimitResource,
    limit: ProcessLimitVm,
) -> RuntimeResult<()> {
    let name = string_ref_from_vm(binding, context, name)?;
    unsafe { host_process::destack_process_job_set_limit(binding, name, resource, limit) }
}

/// Return the effective group identifier.
pub(crate) fn destack_process_egid(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<GroupId> {
    call_out(|out| unsafe { host_process::destack_process_egid(binding, out) })
}

/// Return the effective user identifier.
pub(crate) fn destack_process_euid(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<UserId> {
    call_out(|out| unsafe { host_process::destack_process_euid(binding, out) })
}

/// Return the current group identifier.
pub(crate) fn destack_process_gid(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<GroupId> {
    call_out(|out| unsafe { host_process::destack_process_gid(binding, out) })
}

/// Return real, effective, and saved-set group identifiers.
pub(crate) fn destack_process_group_ids(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<ProcessGroupIdsVm> {
    call_out(|out| unsafe { host_process::destack_process_group_ids(binding, out) })
}

/// Return supplementary group identifiers.
pub(crate) fn destack_process_groups(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<GroupId>> {
    let groups = call_out(|out| unsafe { host_process::destack_process_groups(binding, out) })?;
    values_to_vm(context, groups)
}

/// Return the current process identifier.
pub(crate) fn destack_process_pid(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<ProcessId> {
    call_out(|out| unsafe { host_process::destack_process_pid(binding, out) })
}

/// Return the parent process identifier.
pub(crate) fn destack_process_ppid(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<ProcessId> {
    call_out(|out| unsafe { host_process::destack_process_ppid(binding, out) })
}

/// Set the effective group identifier only.
pub(crate) fn destack_process_set_egid(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    groupid: GroupId,
) -> RuntimeResult<()> {
    unsafe { host_process::destack_process_set_egid(binding, groupid) }
}

/// Set the effective user identifier only.
pub(crate) fn destack_process_set_euid(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    userid: UserId,
) -> RuntimeResult<()> {
    unsafe { host_process::destack_process_set_euid(binding, userid) }
}

/// Set the effective group identifier.
pub(crate) fn destack_process_set_gid(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    groupid: GroupId,
) -> RuntimeResult<()> {
    unsafe { host_process::destack_process_set_gid(binding, groupid) }
}

/// Set real, effective, and saved-set group identifiers together.
pub(crate) fn destack_process_set_group_ids(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    ids: ProcessGroupIdsVm,
) -> RuntimeResult<()> {
    unsafe { host_process::destack_process_set_group_ids(binding, ids) }
}

/// Set supplementary group identifiers.
pub(crate) fn destack_process_set_groups(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    groups: VmSlice<GroupId>,
) -> RuntimeResult<()> {
    let groups = store_values_from_vm(binding, context, groups)?;
    unsafe { host_process::destack_process_set_groups(binding, groups) }
}

/// Set the effective user identifier.
pub(crate) fn destack_process_set_uid(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    userid: UserId,
) -> RuntimeResult<()> {
    unsafe { host_process::destack_process_set_uid(binding, userid) }
}

/// Set real, effective, and saved-set user identifiers together.
pub(crate) fn destack_process_set_user_ids(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    ids: ProcessUserIdsVm,
) -> RuntimeResult<()> {
    unsafe { host_process::destack_process_set_user_ids(binding, ids) }
}

/// Return the current user identifier.
pub(crate) fn destack_process_uid(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<UserId> {
    call_out(|out| unsafe { host_process::destack_process_uid(binding, out) })
}

/// Return real, effective, and saved-set user identifiers.
pub(crate) fn destack_process_user_ids(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<ProcessUserIdsVm> {
    call_out(|out| unsafe { host_process::destack_process_user_ids(binding, out) })
}

/// Change the root directory for path resolution.
pub(crate) fn destack_process_chroot(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: fs::OsPathVm,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_process::destack_process_chroot(binding, path) }
}

/// Install one syscall filter program.
pub(crate) fn destack_process_install_syscall_filter(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    program: VmArray<u8>,
    flags: SyscallFilterFlags,
) -> RuntimeResult<()> {
    let program = store_bytes_array_from_vm(binding, context, program)?;
    unsafe { host_process::destack_process_install_syscall_filter(binding, program, flags) }
}

/// Set process host name inside the active UTS namespace.
pub(crate) fn destack_process_set_host_name(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    name: destack_vm::StringHandle,
) -> RuntimeResult<()> {
    let name = string_ref_from_vm(binding, context, name)?;
    unsafe { host_process::destack_process_set_host_name(binding, name) }
}

/// Set network namespace context for subsequent network operations.
pub(crate) fn destack_process_set_network_namespace(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: fs::OsPathVm,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_process::destack_process_set_network_namespace(binding, path) }
}

/// Enter one namespace owned by another process.
pub(crate) fn destack_process_setns(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    pid: ProcessId,
    namespace: ProcessNamespaceKind,
) -> RuntimeResult<()> {
    unsafe { host_process::destack_process_setns(binding, pid, namespace) }
}

/// Unshare one or more namespaces.
pub(crate) fn destack_process_unshare(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    flags: ProcessUnshareFlags,
) -> RuntimeResult<()> {
    unsafe { host_process::destack_process_unshare(binding, flags) }
}

/// Read a process resource limit.
pub(crate) fn destack_process_get_limit(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    resource: ProcessLimitResource,
) -> RuntimeResult<ProcessLimitVm> {
    call_out(|out| unsafe { host_process::destack_process_get_limit(binding, out, resource) })
}

/// Set a process resource limit.
pub(crate) fn destack_process_set_limit(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    resource: ProcessLimitResource,
    limit: ProcessLimitVm,
) -> RuntimeResult<()> {
    unsafe { host_process::destack_process_set_limit(binding, resource, limit) }
}

/// Read process CPU affinity.
pub(crate) fn destack_process_get_affinity(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    pid: ProcessId,
) -> RuntimeResult<ProcessCpuSetVm> {
    let cpus =
        call_out(|out| unsafe { host_process::destack_process_get_affinity(binding, out, pid) })?;
    let cpus = values_array_to_vm(context, cpus.cpus)?;

    Ok(ProcessCpuSetVm { cpus })
}

/// Read a process priority value.
pub(crate) fn destack_process_get_priority(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    pid: ProcessId,
) -> RuntimeResult<i32> {
    call_out(|out| unsafe { host_process::destack_process_get_priority(binding, out, pid) })
}

/// Read scheduler policy and priority for a process.
pub(crate) fn destack_process_get_scheduler(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    pid: ProcessId,
) -> RuntimeResult<ProcessSchedulerConfigVm> {
    call_out(|out| unsafe { host_process::destack_process_get_scheduler(binding, out, pid) })
}

/// Set process CPU affinity.
pub(crate) fn destack_process_set_affinity(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    pid: ProcessId,
    cpus: ProcessCpuSetVm,
) -> RuntimeResult<()> {
    let cpus = store_values_array_from_vm(binding, context, cpus.cpus)?;
    let cpus = ProcessCpuSet { cpus };

    unsafe { host_process::destack_process_set_affinity(binding, pid, cpus) }
}

/// Set a process priority value.
pub(crate) fn destack_process_set_priority(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    pid: ProcessId,
    priority: i32,
) -> RuntimeResult<()> {
    unsafe { host_process::destack_process_set_priority(binding, pid, priority) }
}

/// Set scheduler policy and priority for a process.
pub(crate) fn destack_process_set_scheduler(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    pid: ProcessId,
    config: ProcessSchedulerConfigVm,
) -> RuntimeResult<()> {
    unsafe { host_process::destack_process_set_scheduler(binding, pid, config) }
}

/// Yield the current thread to the scheduler.
pub(crate) fn destack_process_yield_now(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<()> {
    unsafe { host_process::destack_process_yield_now(binding) }
}

/// Read a process group id.
pub(crate) fn destack_process_getpgid(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    pid: ProcessId,
) -> RuntimeResult<ProcessId> {
    call_out(|out| unsafe { host_process::destack_process_getpgid(binding, out, pid) })
}

/// Set a process group id for a process.
pub(crate) fn destack_process_setpgid(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    pid: ProcessId,
    pgid: ProcessId,
) -> RuntimeResult<()> {
    unsafe { host_process::destack_process_setpgid(binding, pid, pgid) }
}

/// Create a new session and return the new session leader id.
pub(crate) fn destack_process_setsid(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<ProcessId> {
    call_out(|out| unsafe { host_process::destack_process_setsid(binding, out) })
}

/// Send a signal to a target process.
pub(crate) fn destack_process_kill(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    pid: ProcessId,
    signal: Signal,
) -> RuntimeResult<()> {
    unsafe { host_process::destack_process_kill(binding, pid, signal) }
}

/// Read the current thread signal mask.
pub(crate) fn destack_process_signal_mask_read(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<VmArray<Signal>> {
    let signals =
        call_out(|out| unsafe { host_process::destack_process_signal_mask_read(binding, out) })?;
    values_array_to_vm(context, signals)
}

/// Update the current thread signal mask.
pub(crate) fn destack_process_signal_mask_update(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    how: SignalMaskHow,
    signals: VmSlice<Signal>,
) -> RuntimeResult<()> {
    let signals = store_values_from_vm(binding, context, signals)?;
    unsafe { host_process::destack_process_signal_mask_update(binding, how, signals) }
}

/// Receive the next signal event from a subscription.
pub(crate) fn destack_process_signal_receive(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::SignalHandle,
) -> RuntimeResult<SignalEventVm> {
    call_out(|out| unsafe { host_process::destack_process_signal_receive(binding, out, handle) })
}

/// Subscribe to one signal value.
pub(crate) fn destack_process_signal_subscribe(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    signal: Signal,
) -> RuntimeResult<resource::SignalHandle> {
    call_out(|out| unsafe { host_process::destack_process_signal_subscribe(binding, out, signal) })
}

/// Poll one signal event without blocking.
pub(crate) fn destack_process_signal_try_receive(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::SignalHandle,
) -> RuntimeResult<SignalEventVm> {
    call_out(|out| unsafe {
        host_process::destack_process_signal_try_receive(binding, out, handle)
    })
}

/// Poll one signal from a requested set without blocking.
pub(crate) fn destack_process_signal_try_wait(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    signals: VmSlice<Signal>,
) -> RuntimeResult<SignalEventVm> {
    let signals = store_values_from_vm(binding, context, signals)?;
    call_out(|out| unsafe { host_process::destack_process_signal_try_wait(binding, out, signals) })
}

/// Remove a signal subscription handle.
pub(crate) fn destack_process_signal_unsubscribe(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::SignalHandle,
) -> RuntimeResult<()> {
    unsafe { host_process::destack_process_signal_unsubscribe(binding, handle) }
}

/// Wait for one signal from a requested set.
pub(crate) fn destack_process_signal_wait(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    signals: VmSlice<Signal>,
) -> RuntimeResult<SignalEventVm> {
    let signals = store_values_from_vm(binding, context, signals)?;
    call_out(|out| unsafe { host_process::destack_process_signal_wait(binding, out, signals) })
}

/// Spawn a child process with default stdio inheritance.
pub(crate) fn destack_process_spawn(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    command: fs::OsPathVm,
    arguments: VmSlice<destack_vm::StringHandle>,
    environment: VmSlice<destack_vm::StringHandle>,
    options: ProcessSpawnOptionsVm,
) -> RuntimeResult<resource::ProcessHandle> {
    let command = path_ref_from_vm(binding, context, command)?;
    let arguments = string_slice_from_vm(binding, context, arguments)?;
    let environment = string_slice_from_vm(binding, context, environment)?;
    let options = process_spawn_options_from_vm(binding, context, options)?;
    call_out(|out| unsafe {
        host_process::destack_process_spawn(binding, out, command, arguments, environment, options)
    })
}

/// Spawn a child process with explicit stdio and descriptor actions.
pub(crate) fn destack_process_spawn_with_actions(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    command: fs::OsPathVm,
    arguments: VmSlice<destack_vm::StringHandle>,
    environment: VmSlice<destack_vm::StringHandle>,
    options: ProcessSpawnOptionsVm,
    stdio: VmSlice<ProcessStdioVm>,
    actions: VmSlice<ProcessFdActionVm>,
) -> RuntimeResult<resource::ProcessHandle> {
    let command = path_ref_from_vm(binding, context, command)?;
    let arguments = string_slice_from_vm(binding, context, arguments)?;
    let environment = string_slice_from_vm(binding, context, environment)?;
    let options = process_spawn_options_from_vm(binding, context, options)?;
    let stdio = process_stdio_slice_from_vm(binding, context, stdio)?;
    let actions = process_fd_action_slice_from_vm(binding, context, actions)?;

    call_out(|out| unsafe {
        host_process::destack_process_spawn_with_actions(
            binding,
            out,
            command,
            arguments,
            environment,
            options,
            stdio,
            actions,
        )
    })
}

/// Set process file-creation umask and return the previous value.
pub(crate) fn destack_process_umask(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    mask: u32,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_process::destack_process_umask(binding, out, mask) })
}

/// Wait for a process identifier.
pub(crate) fn destack_process_wait_pid(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    pid: ProcessId,
    flags: ProcessWaitFlags,
) -> RuntimeResult<ProcessWaitStatusVm> {
    let value = call_out(|out| unsafe {
        host_process::destack_process_wait_pid(binding, out, pid, flags)
    })?;
    process_wait_status_to_vm(context, value)
}

/// Poll a child process handle without blocking.
pub(crate) fn destack_process_try_wait(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::ProcessHandle,
) -> RuntimeResult<ProcessWaitStatusVm> {
    let value =
        call_out(|out| unsafe { host_process::destack_process_try_wait(binding, out, handle) })?;
    process_wait_status_to_vm(context, value)
}

/// Wait for a child process handle.
pub(crate) fn destack_process_wait(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::ProcessHandle,
    flags: ProcessWaitFlags,
) -> RuntimeResult<ProcessWaitStatusVm> {
    let value =
        call_out(|out| unsafe { host_process::destack_process_wait(binding, out, handle, flags) })?;
    process_wait_status_to_vm(context, value)
}
