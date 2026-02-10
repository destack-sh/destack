use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::{
    ExecAtFlags, GroupId, ProcessCpuSetVm, ProcessFdActionVm, ProcessFdFlags, ProcessFdSignalFlags,
    ProcessId, ProcessLimitResource, ProcessLimitVm, ProcessNamespaceKind, ProcessSpawnOptionsVm,
    ProcessStdioVm, ProcessUnshareFlags, ProcessWaitFlags, ProcessWaitStatusVm, Signal,
    SignalEventVm, SignalFdFlags, SignalMaskHow, SyscallFilterFlags, UserId,
};
use crate::platform::{PlatformError, VmArray, VmSlice, fs, resource};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.process.args.args.
pub(super) fn destack_process_args(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<VmSlice<vm::StringHandle>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.args.args is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.cwd.chdir.
pub(super) fn destack_process_chdir(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: fs::OsPathVm,
) -> RuntimeResult<()> {
    let _ = path;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.cwd.chdir is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.cwd.cwd.
pub(super) fn destack_process_cwd(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<fs::OsPathVm> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.cwd.cwd is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.env.delete.
pub(super) fn destack_process_env_delete(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = name;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.env.delete is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.env.deleteBytes.
pub(super) fn destack_process_env_delete_bytes(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    name: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = name;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.env.deleteBytes is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.env.get.
pub(super) fn destack_process_env_get(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    name: vm::StringHandle,
) -> RuntimeResult<vm::StringHandle> {
    let _ = name;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.env.get is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.env.getBytes.
pub(super) fn destack_process_env_get_bytes(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    name: VmSlice<u8>,
) -> RuntimeResult<VmArray<u8>> {
    let _ = name;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.env.getBytes is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.env.set.
pub(super) fn destack_process_env_set(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    name: vm::StringHandle,
    value: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (name, value);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.env.set is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.env.setBytes.
pub(super) fn destack_process_env_set_bytes(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    name: VmSlice<u8>,
    value: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (name, value);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.env.setBytes is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.exec.exec.
pub(super) fn destack_process_exec(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    command: fs::OsPathVm,
    arguments: VmSlice<vm::StringHandle>,
    environment: VmSlice<vm::StringHandle>,
) -> RuntimeResult<()> {
    let _ = (command, arguments, environment);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.exec.exec is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.exec.execat.
pub(super) fn destack_process_execat(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    directory: resource::DirectoryHandle,
    path: fs::OsPathVm,
    arguments: VmSlice<vm::StringHandle>,
    environment: VmSlice<vm::StringHandle>,
    flags: ExecAtFlags,
) -> RuntimeResult<()> {
    let _ = (directory, path, arguments, environment, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.exec.execat is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.exec.fexec.
pub(super) fn destack_process_fexec(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    executable: resource::FileHandle,
    arguments: VmSlice<vm::StringHandle>,
    environment: VmSlice<vm::StringHandle>,
) -> RuntimeResult<()> {
    let _ = (executable, arguments, environment);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.exec.fexec is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.exit.exit.
pub(super) fn destack_process_exit(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    code: u32,
) -> RuntimeResult<()> {
    let _ = code;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.exit.exit is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.fd.processFdClose.
pub(super) fn destack_process_process_fd_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::ProcessFdHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.processFdClose is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.fd.processFdOpen.
pub(super) fn destack_process_process_fd_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    pid: ProcessId,
    flags: ProcessFdFlags,
) -> RuntimeResult<resource::ProcessFdHandle> {
    let _ = (pid, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.processFdOpen is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.fd.processFdSendSignal.
pub(super) fn destack_process_process_fd_send_signal(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::ProcessFdHandle,
    signal: Signal,
    flags: ProcessFdSignalFlags,
) -> RuntimeResult<()> {
    let _ = (handle, signal, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.processFdSendSignal is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.fd.processFdTryWait.
pub(super) fn destack_process_process_fd_try_wait(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::ProcessFdHandle,
) -> RuntimeResult<ProcessWaitStatusVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.processFdTryWait is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.fd.processFdWait.
pub(super) fn destack_process_process_fd_wait(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::ProcessFdHandle,
    timeoutns: u64,
) -> RuntimeResult<ProcessWaitStatusVm> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.processFdWait is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.fd.signalFdClose.
pub(super) fn destack_process_signal_fd_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdClose is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.fd.signalFdOpen.
pub(super) fn destack_process_signal_fd_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    signals: VmSlice<Signal>,
    flags: SignalFdFlags,
) -> RuntimeResult<resource::SignalFdHandle> {
    let _ = (signals, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdOpen is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.fd.signalFdRead.
pub(super) fn destack_process_signal_fd_read(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<SignalEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdRead is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.fd.signalFdSetMask.
pub(super) fn destack_process_signal_fd_set_mask(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SignalFdHandle,
    signals: VmSlice<Signal>,
) -> RuntimeResult<()> {
    let _ = (handle, signals);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdSetMask is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.fd.signalFdTryRead.
pub(super) fn destack_process_signal_fd_try_read(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<SignalEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdTryRead is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.group.cgroupGetLimit.
pub(super) fn destack_process_cgroup_get_limit(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: vm::StringHandle,
    resource: ProcessLimitResource,
) -> RuntimeResult<ProcessLimitVm> {
    let _ = (path, resource);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.cgroupGetLimit is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.group.cgroupJoin.
pub(super) fn destack_process_cgroup_join(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = path;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.cgroupJoin is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.group.cgroupSetLimit.
pub(super) fn destack_process_cgroup_set_limit(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: vm::StringHandle,
    resource: ProcessLimitResource,
    limit: ProcessLimitVm,
) -> RuntimeResult<()> {
    let _ = (path, resource, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.cgroupSetLimit is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.group.jobAssign.
pub(super) fn destack_process_job_assign(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    name: vm::StringHandle,
    pids: VmSlice<ProcessId>,
) -> RuntimeResult<()> {
    let _ = (name, pids);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.jobAssign is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.group.jobSetLimit.
pub(super) fn destack_process_job_set_limit(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    name: vm::StringHandle,
    resource: ProcessLimitResource,
    limit: ProcessLimitVm,
) -> RuntimeResult<()> {
    let _ = (name, resource, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.jobSetLimit is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.ids.gid.
pub(super) fn destack_process_gid(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<GroupId> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.ids.gid is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.ids.pid.
pub(super) fn destack_process_pid(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<ProcessId> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.ids.pid is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.ids.ppid.
pub(super) fn destack_process_ppid(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<ProcessId> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.ids.ppid is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.ids.setGid.
pub(super) fn destack_process_set_gid(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    groupid: GroupId,
) -> RuntimeResult<()> {
    let _ = groupid;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.ids.setGid is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.ids.setGroups.
pub(super) fn destack_process_set_groups(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    groups: VmSlice<GroupId>,
) -> RuntimeResult<()> {
    let _ = groups;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.ids.setGroups is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.ids.setUid.
pub(super) fn destack_process_set_uid(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    userid: UserId,
) -> RuntimeResult<()> {
    let _ = userid;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.ids.setUid is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.ids.uid.
pub(super) fn destack_process_uid(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<UserId> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.ids.uid is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.isolation.chroot.
pub(super) fn destack_process_chroot(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: fs::OsPathVm,
) -> RuntimeResult<()> {
    let _ = path;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.chroot is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.isolation.installSyscallFilter.
pub(super) fn destack_process_install_syscall_filter(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    program: VmArray<u8>,
    flags: SyscallFilterFlags,
) -> RuntimeResult<()> {
    let _ = (program, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.installSyscallFilter is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.isolation.setHostName.
pub(super) fn destack_process_set_host_name(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = name;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.setHostName is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.isolation.setNetworkNamespace.
pub(super) fn destack_process_set_network_namespace(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: fs::OsPathVm,
) -> RuntimeResult<()> {
    let _ = path;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.setNetworkNamespace is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.isolation.setns.
pub(super) fn destack_process_setns(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    pid: ProcessId,
    namespace: ProcessNamespaceKind,
) -> RuntimeResult<()> {
    let _ = (pid, namespace);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.setns is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.isolation.unshare.
pub(super) fn destack_process_unshare(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    flags: ProcessUnshareFlags,
) -> RuntimeResult<()> {
    let _ = flags;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.unshare is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.limits.getLimit.
pub(super) fn destack_process_get_limit(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    resource: ProcessLimitResource,
) -> RuntimeResult<ProcessLimitVm> {
    let _ = resource;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.limits.getLimit is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.limits.setLimit.
pub(super) fn destack_process_set_limit(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    resource: ProcessLimitResource,
    limit: ProcessLimitVm,
) -> RuntimeResult<()> {
    let _ = (resource, limit);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.limits.setLimit is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.sched.getAffinity.
pub(super) fn destack_process_get_affinity(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    pid: ProcessId,
) -> RuntimeResult<ProcessCpuSetVm> {
    let _ = pid;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.getAffinity is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.sched.getPriority.
pub(super) fn destack_process_get_priority(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    pid: ProcessId,
) -> RuntimeResult<i32> {
    let _ = pid;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.getPriority is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.sched.setAffinity.
pub(super) fn destack_process_set_affinity(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    pid: ProcessId,
    cpus: ProcessCpuSetVm,
) -> RuntimeResult<()> {
    let _ = (pid, cpus);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.setAffinity is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.sched.setPriority.
pub(super) fn destack_process_set_priority(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    pid: ProcessId,
    priority: i32,
) -> RuntimeResult<()> {
    let _ = (pid, priority);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.setPriority is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.session.getpgid.
pub(super) fn destack_process_getpgid(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    pid: ProcessId,
) -> RuntimeResult<ProcessId> {
    let _ = pid;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.session.getpgid is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.session.setpgid.
pub(super) fn destack_process_setpgid(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    pid: ProcessId,
    pgid: ProcessId,
) -> RuntimeResult<()> {
    let _ = (pid, pgid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.session.setpgid is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.session.setsid.
pub(super) fn destack_process_setsid(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<ProcessId> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.session.setsid is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.signals.kill.
pub(super) fn destack_process_kill(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    pid: ProcessId,
    signal: Signal,
) -> RuntimeResult<()> {
    let _ = (pid, signal);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.kill is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.signals.signalMaskRead.
pub(super) fn destack_process_signal_mask_read(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<VmArray<Signal>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalMaskRead is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.signals.signalMaskUpdate.
pub(super) fn destack_process_signal_mask_update(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    how: SignalMaskHow,
    signals: VmSlice<Signal>,
) -> RuntimeResult<()> {
    let _ = (how, signals);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalMaskUpdate is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.signals.signalReceive.
pub(super) fn destack_process_signal_receive(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SignalHandle,
) -> RuntimeResult<SignalEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalReceive is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.signals.signalSubscribe.
pub(super) fn destack_process_signal_subscribe(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    signal: Signal,
) -> RuntimeResult<resource::SignalHandle> {
    let _ = signal;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalSubscribe is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.signals.signalTryReceive.
pub(super) fn destack_process_signal_try_receive(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SignalHandle,
) -> RuntimeResult<SignalEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalTryReceive is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.signals.signalTryWait.
pub(super) fn destack_process_signal_try_wait(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    signals: VmSlice<Signal>,
) -> RuntimeResult<SignalEventVm> {
    let _ = signals;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalTryWait is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.signals.signalUnsubscribe.
pub(super) fn destack_process_signal_unsubscribe(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SignalHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalUnsubscribe is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.signals.signalWait.
pub(super) fn destack_process_signal_wait(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    signals: VmSlice<Signal>,
) -> RuntimeResult<SignalEventVm> {
    let _ = signals;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalWait is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.spawn.spawn.
pub(super) fn destack_process_spawn(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    command: fs::OsPathVm,
    arguments: VmSlice<vm::StringHandle>,
    environment: VmSlice<vm::StringHandle>,
    options: ProcessSpawnOptionsVm,
) -> RuntimeResult<resource::ProcessHandle> {
    let _ = (command, arguments, environment, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.spawn.spawn is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.spawn.withActions.
pub(super) fn destack_process_spawn_with_actions(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    command: fs::OsPathVm,
    arguments: VmSlice<vm::StringHandle>,
    environment: VmSlice<vm::StringHandle>,
    options: ProcessSpawnOptionsVm,
    stdio: VmSlice<ProcessStdioVm>,
    actions: VmSlice<ProcessFdActionVm>,
) -> RuntimeResult<resource::ProcessHandle> {
    let _ = (command, arguments, environment, options, stdio, actions);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.spawn.withActions is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.umask.umask.
pub(super) fn destack_process_umask(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    mask: u32,
) -> RuntimeResult<u32> {
    let _ = mask;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.umask.umask is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.wait.pid.
pub(super) fn destack_process_wait_pid(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    pid: ProcessId,
    flags: ProcessWaitFlags,
) -> RuntimeResult<ProcessWaitStatusVm> {
    let _ = (pid, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.wait.pid is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.wait.tryWait.
pub(super) fn destack_process_try_wait(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::ProcessHandle,
) -> RuntimeResult<ProcessWaitStatusVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.wait.tryWait is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.process.wait.wait.
pub(super) fn destack_process_wait(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::ProcessHandle,
    flags: ProcessWaitFlags,
) -> RuntimeResult<ProcessWaitStatusVm> {
    let _ = (handle, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.wait.wait is not available in the VM yet",
    ))
    .boxed())
}
