#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::{
    ExecAtFlags, GroupId, ProcessCpuSetVm, ProcessFdActionVm, ProcessFdFlags, ProcessFdSignalFlags,
    ProcessGroupIdsVm, ProcessId, ProcessLimitResource, ProcessLimitVm, ProcessNamespaceKind,
    ProcessSchedulerConfigVm, ProcessSpawnOptionsVm, ProcessStdioVm, ProcessUnshareFlags,
    ProcessUserIdsVm, ProcessWaitFlags, ProcessWaitStatusVm, Signal, SignalEventVm, SignalFdFlags,
    SignalMaskHow, SyscallFilterFlags, UserId,
};
use crate::platform::{PlatformError, VmArray, VmSlice, fs, resource};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// Return the process argument vector.
pub(crate) fn destack_process_args(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<vm::StringHandle>> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.args.list")).boxed())
}

/// Change the current working directory.
pub(crate) fn destack_process_chdir(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: fs::OsPathVm,
) -> RuntimeResult<()> {
    let _ = path;
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.cwd.chdir")).boxed())
}

/// Return the current working directory.
pub(crate) fn destack_process_cwd(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<fs::OsPathVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.cwd.get")).boxed())
}

/// Delete an environment variable by UTF-8 name.
pub(crate) fn destack_process_env_delete(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = name;
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.env.delete")).boxed())
}

/// Delete an environment variable by raw byte name.
pub(crate) fn destack_process_env_delete_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    name: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = name;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.env.deleteBytes",
    ))
    .boxed())
}

/// Read an environment variable by UTF-8 name.
pub(crate) fn destack_process_env_get(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    name: vm::StringHandle,
) -> RuntimeResult<vm::StringHandle> {
    let _ = name;
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.env.get")).boxed())
}

/// Read an environment variable by raw byte name.
pub(crate) fn destack_process_env_get_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    name: VmSlice<u8>,
) -> RuntimeResult<VmArray<u8>> {
    let _ = name;
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.env.getBytes")).boxed())
}

/// Set an environment variable by UTF-8 name and value.
pub(crate) fn destack_process_env_set(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    name: vm::StringHandle,
    argument_value: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (name, argument_value);
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.env.set")).boxed())
}

/// Set an environment variable by raw byte name and value.
pub(crate) fn destack_process_env_set_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    name: VmSlice<u8>,
    argument_value: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (name, argument_value);
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.env.setBytes")).boxed())
}

/// Replace the current process image with a command path.
pub(crate) fn destack_process_exec(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    command: fs::OsPathVm,
    arguments: VmSlice<vm::StringHandle>,
    environment: VmSlice<vm::StringHandle>,
) -> RuntimeResult<()> {
    let _ = (command, arguments, environment);
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.exec.path")).boxed())
}

/// Replace the current process image using a directory-relative path.
pub(crate) fn destack_process_execat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    directory: resource::DirectoryHandle,
    path: fs::OsPathVm,
    arguments: VmSlice<vm::StringHandle>,
    environment: VmSlice<vm::StringHandle>,
    flags: ExecAtFlags,
) -> RuntimeResult<()> {
    let _ = (directory, path, arguments, environment, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.exec.pathat")).boxed())
}

/// Replace the current process image using an executable file handle.
pub(crate) fn destack_process_fexec(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    executable: resource::FileHandle,
    arguments: VmSlice<vm::StringHandle>,
    environment: VmSlice<vm::StringHandle>,
) -> RuntimeResult<()> {
    let _ = (executable, arguments, environment);
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.exec.fexec")).boxed())
}

/// Exit the current process with the given code.
pub(crate) fn destack_process_exit(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    code: u32,
) -> RuntimeResult<()> {
    let _ = code;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.exit.terminate",
    ))
    .boxed())
}

/// Close one process descriptor.
pub(crate) fn destack_process_process_fd_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::ProcessFdHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.processFdClose",
    ))
    .boxed())
}

/// Open one process descriptor for the target process id.
pub(crate) fn destack_process_process_fd_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    pid: ProcessId,
    flags: ProcessFdFlags,
) -> RuntimeResult<resource::ProcessFdHandle> {
    let _ = (pid, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.processFdOpen",
    ))
    .boxed())
}

/// Send one signal through a process descriptor.
pub(crate) fn destack_process_process_fd_send_signal(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::ProcessFdHandle,
    signal: Signal,
    flags: ProcessFdSignalFlags,
) -> RuntimeResult<()> {
    let _ = (handle, signal, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.processFdSendSignal",
    ))
    .boxed())
}

/// Poll one process descriptor state transition without blocking.
pub(crate) fn destack_process_process_fd_try_wait(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::ProcessFdHandle,
) -> RuntimeResult<ProcessWaitStatusVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.processFdTryWait",
    ))
    .boxed())
}

/// Wait for one process descriptor state transition.
pub(crate) fn destack_process_process_fd_wait(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::ProcessFdHandle,
    timeoutns: u64,
) -> RuntimeResult<ProcessWaitStatusVm> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.processFdWait",
    ))
    .boxed())
}

/// Close one signal descriptor.
pub(crate) fn destack_process_signal_fd_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdClose",
    ))
    .boxed())
}

/// Open one signal descriptor for the provided signal mask.
pub(crate) fn destack_process_signal_fd_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    signals: VmSlice<Signal>,
    flags: SignalFdFlags,
) -> RuntimeResult<resource::SignalFdHandle> {
    let _ = (signals, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdOpen",
    ))
    .boxed())
}

/// Read one queued signal event from a signal descriptor.
pub(crate) fn destack_process_signal_fd_read(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<SignalEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdRead",
    ))
    .boxed())
}

/// Replace the active signal mask for one signal descriptor.
pub(crate) fn destack_process_signal_fd_set_mask(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SignalFdHandle,
    signals: VmSlice<Signal>,
) -> RuntimeResult<()> {
    let _ = (handle, signals);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdSetMask",
    ))
    .boxed())
}

/// Poll one queued signal event from a signal descriptor without blocking.
pub(crate) fn destack_process_signal_fd_try_read(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<SignalEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdTryRead",
    ))
    .boxed())
}

/// Open one standard error stream handle.
pub(crate) fn destack_process_stdio_stderr(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::FileHandle> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.stdioStderr",
    ))
    .boxed())
}

/// Open one standard input stream handle.
pub(crate) fn destack_process_stdio_stdin(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::FileHandle> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.stdioStdin",
    ))
    .boxed())
}

/// Open one standard output stream handle.
pub(crate) fn destack_process_stdio_stdout(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::FileHandle> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.stdioStdout",
    ))
    .boxed())
}

/// Read one control-group resource limit.
pub(crate) fn destack_process_cgroup_get_limit(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: vm::StringHandle,
    resource: ProcessLimitResource,
) -> RuntimeResult<ProcessLimitVm> {
    let _ = (path, resource);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.cgroupGetLimit",
    ))
    .boxed())
}

/// Join one control group.
pub(crate) fn destack_process_cgroup_join(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = path;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.cgroupJoin",
    ))
    .boxed())
}

/// Write one control-group resource limit.
pub(crate) fn destack_process_cgroup_set_limit(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: vm::StringHandle,
    resource: ProcessLimitResource,
    limit: ProcessLimitVm,
) -> RuntimeResult<()> {
    let _ = (path, resource, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.cgroupSetLimit",
    ))
    .boxed())
}

/// Assign processes to one Windows job object.
pub(crate) fn destack_process_job_assign(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    name: vm::StringHandle,
    pids: VmSlice<ProcessId>,
) -> RuntimeResult<()> {
    let _ = (name, pids);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.jobAssign",
    ))
    .boxed())
}

/// Set one Windows job object resource limit.
pub(crate) fn destack_process_job_set_limit(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    name: vm::StringHandle,
    resource: ProcessLimitResource,
    limit: ProcessLimitVm,
) -> RuntimeResult<()> {
    let _ = (name, resource, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.jobSetLimit",
    ))
    .boxed())
}

/// Return the effective group identifier.
pub(crate) fn destack_process_egid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<GroupId> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.egid")).boxed())
}

/// Return the effective user identifier.
pub(crate) fn destack_process_euid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<UserId> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.euid")).boxed())
}

/// Return the current group identifier.
pub(crate) fn destack_process_gid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<GroupId> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.gid")).boxed())
}

/// Return real, effective, and saved-set group identifiers.
pub(crate) fn destack_process_group_ids(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<ProcessGroupIdsVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.groupIds")).boxed())
}

/// Return supplementary group identifiers.
pub(crate) fn destack_process_groups(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<GroupId>> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.groups")).boxed())
}

/// Return the current process identifier.
pub(crate) fn destack_process_pid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<ProcessId> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.pid")).boxed())
}

/// Return the parent process identifier.
pub(crate) fn destack_process_ppid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<ProcessId> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.ppid")).boxed())
}

/// Set the effective group identifier only.
pub(crate) fn destack_process_set_egid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    groupid: GroupId,
) -> RuntimeResult<()> {
    let _ = groupid;
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.setEgid")).boxed())
}

/// Set the effective user identifier only.
pub(crate) fn destack_process_set_euid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    userid: UserId,
) -> RuntimeResult<()> {
    let _ = userid;
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.setEuid")).boxed())
}

/// Set the effective group identifier.
pub(crate) fn destack_process_set_gid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    groupid: GroupId,
) -> RuntimeResult<()> {
    let _ = groupid;
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.setGid")).boxed())
}

/// Set real, effective, and saved-set group identifiers together.
pub(crate) fn destack_process_set_group_ids(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    ids: ProcessGroupIdsVm,
) -> RuntimeResult<()> {
    let _ = ids;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.ids.setGroupIds",
    ))
    .boxed())
}

/// Set supplementary group identifiers.
pub(crate) fn destack_process_set_groups(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    groups: VmSlice<GroupId>,
) -> RuntimeResult<()> {
    let _ = groups;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.ids.setGroups",
    ))
    .boxed())
}

/// Set the effective user identifier.
pub(crate) fn destack_process_set_uid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    userid: UserId,
) -> RuntimeResult<()> {
    let _ = userid;
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.setUid")).boxed())
}

/// Set real, effective, and saved-set user identifiers together.
pub(crate) fn destack_process_set_user_ids(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    ids: ProcessUserIdsVm,
) -> RuntimeResult<()> {
    let _ = ids;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.ids.setUserIds",
    ))
    .boxed())
}

/// Return the current user identifier.
pub(crate) fn destack_process_uid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<UserId> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.uid")).boxed())
}

/// Return real, effective, and saved-set user identifiers.
pub(crate) fn destack_process_user_ids(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<ProcessUserIdsVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.userIds")).boxed())
}

/// Change the root directory for path resolution.
pub(crate) fn destack_process_chroot(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: fs::OsPathVm,
) -> RuntimeResult<()> {
    let _ = path;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.chroot",
    ))
    .boxed())
}

/// Install one syscall filter program.
pub(crate) fn destack_process_install_syscall_filter(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    program: VmArray<u8>,
    flags: SyscallFilterFlags,
) -> RuntimeResult<()> {
    let _ = (program, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.installSyscallFilter",
    ))
    .boxed())
}

/// Set process host name inside the active UTS namespace.
pub(crate) fn destack_process_set_host_name(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = name;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.setHostName",
    ))
    .boxed())
}

/// Set network namespace context for subsequent network operations.
pub(crate) fn destack_process_set_network_namespace(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: fs::OsPathVm,
) -> RuntimeResult<()> {
    let _ = path;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.setNetworkNamespace",
    ))
    .boxed())
}

/// Enter one namespace owned by another process.
pub(crate) fn destack_process_setns(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    pid: ProcessId,
    namespace: ProcessNamespaceKind,
) -> RuntimeResult<()> {
    let _ = (pid, namespace);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.setns",
    ))
    .boxed())
}

/// Unshare one or more namespaces.
pub(crate) fn destack_process_unshare(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    flags: ProcessUnshareFlags,
) -> RuntimeResult<()> {
    let _ = flags;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.unshare",
    ))
    .boxed())
}

/// Read a process resource limit.
pub(crate) fn destack_process_get_limit(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    resource: ProcessLimitResource,
) -> RuntimeResult<ProcessLimitVm> {
    let _ = resource;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.limits.getLimit",
    ))
    .boxed())
}

/// Set a process resource limit.
pub(crate) fn destack_process_set_limit(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    resource: ProcessLimitResource,
    limit: ProcessLimitVm,
) -> RuntimeResult<()> {
    let _ = (resource, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.limits.setLimit",
    ))
    .boxed())
}

/// Read process CPU affinity.
pub(crate) fn destack_process_get_affinity(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    pid: ProcessId,
) -> RuntimeResult<ProcessCpuSetVm> {
    let _ = pid;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.getAffinity",
    ))
    .boxed())
}

/// Read a process priority value.
pub(crate) fn destack_process_get_priority(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    pid: ProcessId,
) -> RuntimeResult<i32> {
    let _ = pid;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.getPriority",
    ))
    .boxed())
}

/// Read scheduler policy and priority for a process.
pub(crate) fn destack_process_get_scheduler(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    pid: ProcessId,
) -> RuntimeResult<ProcessSchedulerConfigVm> {
    let _ = pid;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.getScheduler",
    ))
    .boxed())
}

/// Set process CPU affinity.
pub(crate) fn destack_process_set_affinity(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    pid: ProcessId,
    cpus: ProcessCpuSetVm,
) -> RuntimeResult<()> {
    let _ = (pid, cpus);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.setAffinity",
    ))
    .boxed())
}

/// Set a process priority value.
pub(crate) fn destack_process_set_priority(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    pid: ProcessId,
    priority: i32,
) -> RuntimeResult<()> {
    let _ = (pid, priority);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.setPriority",
    ))
    .boxed())
}

/// Set scheduler policy and priority for a process.
pub(crate) fn destack_process_set_scheduler(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    pid: ProcessId,
    config: ProcessSchedulerConfigVm,
) -> RuntimeResult<()> {
    let _ = (pid, config);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.setScheduler",
    ))
    .boxed())
}

/// Yield the current thread to the scheduler.
pub(crate) fn destack_process_yield_now(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.yieldNow",
    ))
    .boxed())
}

/// Read a process group id.
pub(crate) fn destack_process_getpgid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    pid: ProcessId,
) -> RuntimeResult<ProcessId> {
    let _ = pid;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.session.getpgid",
    ))
    .boxed())
}

/// Set a process group id for a process.
pub(crate) fn destack_process_setpgid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    pid: ProcessId,
    pgid: ProcessId,
) -> RuntimeResult<()> {
    let _ = (pid, pgid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.session.setpgid",
    ))
    .boxed())
}

/// Create a new session and return the new session leader id.
pub(crate) fn destack_process_setsid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<ProcessId> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.session.setsid",
    ))
    .boxed())
}

/// Send a signal to a target process.
pub(crate) fn destack_process_kill(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    pid: ProcessId,
    signal: Signal,
) -> RuntimeResult<()> {
    let _ = (pid, signal);
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.signals.kill")).boxed())
}

/// Read the current thread signal mask.
pub(crate) fn destack_process_signal_mask_read(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmArray<Signal>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalMaskRead",
    ))
    .boxed())
}

/// Update the current thread signal mask.
pub(crate) fn destack_process_signal_mask_update(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    how: SignalMaskHow,
    signals: VmSlice<Signal>,
) -> RuntimeResult<()> {
    let _ = (how, signals);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalMaskUpdate",
    ))
    .boxed())
}

/// Receive the next signal event from a subscription.
pub(crate) fn destack_process_signal_receive(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SignalHandle,
) -> RuntimeResult<SignalEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalReceive",
    ))
    .boxed())
}

/// Subscribe to one signal value.
pub(crate) fn destack_process_signal_subscribe(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    signal: Signal,
) -> RuntimeResult<resource::SignalHandle> {
    let _ = signal;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalSubscribe",
    ))
    .boxed())
}

/// Poll one signal event without blocking.
pub(crate) fn destack_process_signal_try_receive(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SignalHandle,
) -> RuntimeResult<SignalEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalTryReceive",
    ))
    .boxed())
}

/// Poll one signal from a requested set without blocking.
pub(crate) fn destack_process_signal_try_wait(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    signals: VmSlice<Signal>,
) -> RuntimeResult<SignalEventVm> {
    let _ = signals;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalTryWait",
    ))
    .boxed())
}

/// Remove a signal subscription handle.
pub(crate) fn destack_process_signal_unsubscribe(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SignalHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalUnsubscribe",
    ))
    .boxed())
}

/// Wait for one signal from a requested set.
pub(crate) fn destack_process_signal_wait(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    signals: VmSlice<Signal>,
) -> RuntimeResult<SignalEventVm> {
    let _ = signals;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalWait",
    ))
    .boxed())
}

/// Spawn a child process with default stdio inheritance.
pub(crate) fn destack_process_spawn(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    command: fs::OsPathVm,
    arguments: VmSlice<vm::StringHandle>,
    environment: VmSlice<vm::StringHandle>,
    options: ProcessSpawnOptionsVm,
) -> RuntimeResult<resource::ProcessHandle> {
    let _ = (command, arguments, environment, options);
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.spawn.start")).boxed())
}

/// Spawn a child process with explicit stdio and descriptor actions.
pub(crate) fn destack_process_spawn_with_actions(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    command: fs::OsPathVm,
    arguments: VmSlice<vm::StringHandle>,
    environment: VmSlice<vm::StringHandle>,
    options: ProcessSpawnOptionsVm,
    stdio: VmSlice<ProcessStdioVm>,
    actions: VmSlice<ProcessFdActionVm>,
) -> RuntimeResult<resource::ProcessHandle> {
    let _ = (command, arguments, environment, options, stdio, actions);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.spawn.withActions",
    ))
    .boxed())
}

/// Set process file-creation umask and return the previous value.
pub(crate) fn destack_process_umask(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    mask: u32,
) -> RuntimeResult<u32> {
    let _ = mask;
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.umask.set")).boxed())
}

/// Wait for a process identifier.
pub(crate) fn destack_process_wait_pid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    pid: ProcessId,
    flags: ProcessWaitFlags,
) -> RuntimeResult<ProcessWaitStatusVm> {
    let _ = (pid, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.wait.pid")).boxed())
}

/// Poll a child process handle without blocking.
pub(crate) fn destack_process_try_wait(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::ProcessHandle,
) -> RuntimeResult<ProcessWaitStatusVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.wait.tryWait")).boxed())
}

/// Wait for a child process handle.
pub(crate) fn destack_process_wait(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::ProcessHandle,
    flags: ProcessWaitFlags,
) -> RuntimeResult<ProcessWaitStatusVm> {
    let _ = (handle, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.wait.handle")).boxed())
}
