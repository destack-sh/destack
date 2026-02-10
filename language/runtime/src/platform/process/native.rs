#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::bindings_generated as bindings;
use crate::platform::{
    NativeArray, NativeSlice, NativeStringRef, NativeStringSlice, PlatformError,
};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::process::{
    ExecAtFlags, GroupId, ProcessCpuSet, ProcessFdAction, ProcessFdFlags, ProcessFdSignalFlags,
    ProcessId, ProcessLimit, ProcessLimitResource, ProcessNamespaceKind, ProcessSpawnOptions,
    ProcessStdio, ProcessUnshareFlags, ProcessWaitFlags, ProcessWaitStatus, Signal, SignalEvent,
    SignalFdFlags, SignalMaskHow, SyscallFilterFlags, UserId,
};
use crate::platform::{fs, resource};

/// Stub for destack.process.args.args.
pub unsafe fn destack_process_args(
    context: &RuntimeCallContext,
    out: *mut NativeStringSlice,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_ARGS_ARGS)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.args.args")).boxed())
}

/// Stub for destack.process.cwd.chdir.
pub unsafe fn destack_process_chdir(
    context: &RuntimeCallContext,
    path: fs::OsPath,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_CWD_CHDIR)?;
    let _ = path;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.cwd.chdir")).boxed())
}

/// Stub for destack.process.cwd.cwd.
pub unsafe fn destack_process_cwd(
    context: &RuntimeCallContext,
    out: *mut fs::OsPath,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_CWD_CWD)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.cwd.cwd")).boxed())
}

/// Stub for destack.process.env.delete.
pub unsafe fn destack_process_env_delete(
    context: &RuntimeCallContext,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_ENV_DELETE)?;
    let _ = name;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.env.delete")).boxed())
}

/// Stub for destack.process.env.deleteBytes.
pub unsafe fn destack_process_env_delete_bytes(
    context: &RuntimeCallContext,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_ENV_DELETE_BYTES)?;
    let _ = name;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.env.deleteBytes",
    ))
    .boxed())
}

/// Stub for destack.process.env.get.
pub unsafe fn destack_process_env_get(
    context: &RuntimeCallContext,
    out: *mut NativeStringRef,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_ENV_GET)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, name);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.env.get")).boxed())
}

/// Stub for destack.process.env.getBytes.
pub unsafe fn destack_process_env_get_bytes(
    context: &RuntimeCallContext,
    out: *mut NativeArray<u8>,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_ENV_GET_BYTES)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, name);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.env.getBytes")).boxed())
}

/// Stub for destack.process.env.set.
pub unsafe fn destack_process_env_set(
    context: &RuntimeCallContext,
    name: NativeStringRef,
    value: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_ENV_SET)?;
    let _ = (name, value);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.env.set")).boxed())
}

/// Stub for destack.process.env.setBytes.
pub unsafe fn destack_process_env_set_bytes(
    context: &RuntimeCallContext,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_ENV_SET_BYTES)?;
    let _ = (name, value);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.env.setBytes")).boxed())
}

/// Stub for destack.process.exec.exec.
pub unsafe fn destack_process_exec(
    context: &RuntimeCallContext,
    command: fs::OsPath,
    arguments: NativeStringSlice,
    environment: NativeStringSlice,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_EXEC_EXEC)?;
    let _ = (command, arguments, environment);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.exec.exec")).boxed())
}

/// Stub for destack.process.exec.execat.
pub unsafe fn destack_process_execat(
    context: &RuntimeCallContext,
    directory: resource::DirectoryHandle,
    path: fs::OsPath,
    arguments: NativeStringSlice,
    environment: NativeStringSlice,
    flags: ExecAtFlags,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_EXEC_EXECAT)?;
    let _ = (directory, path, arguments, environment, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.exec.execat")).boxed())
}

/// Stub for destack.process.exec.fexec.
pub unsafe fn destack_process_fexec(
    context: &RuntimeCallContext,
    executable: resource::FileHandle,
    arguments: NativeStringSlice,
    environment: NativeStringSlice,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_EXEC_FEXEC)?;
    let _ = (executable, arguments, environment);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.exec.fexec")).boxed())
}

/// Stub for destack.process.exit.exit.
pub unsafe fn destack_process_exit(context: &RuntimeCallContext, code: u32) -> RuntimeResult<()> {
    context.check_policy(PROCESS_EXIT_EXIT)?;
    let _ = code;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.exit.exit")).boxed())
}

/// Stub for destack.process.fd.processFdClose.
pub unsafe fn destack_process_process_fd_close(
    context: &RuntimeCallContext,
    handle: resource::ProcessFdHandle,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_FD_PROCESS_FD_CLOSE)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.processFdClose",
    ))
    .boxed())
}

/// Stub for destack.process.fd.processFdOpen.
pub unsafe fn destack_process_process_fd_open(
    context: &RuntimeCallContext,
    out: *mut resource::ProcessFdHandle,
    pid: ProcessId,
    flags: ProcessFdFlags,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_FD_PROCESS_FD_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, pid, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.processFdOpen",
    ))
    .boxed())
}

/// Stub for destack.process.fd.processFdSendSignal.
pub unsafe fn destack_process_process_fd_send_signal(
    context: &RuntimeCallContext,
    handle: resource::ProcessFdHandle,
    signal: Signal,
    flags: ProcessFdSignalFlags,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_FD_PROCESS_FD_SEND_SIGNAL)?;
    let _ = (handle, signal, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.processFdSendSignal",
    ))
    .boxed())
}

/// Stub for destack.process.fd.processFdTryWait.
pub unsafe fn destack_process_process_fd_try_wait(
    context: &RuntimeCallContext,
    out: *mut ProcessWaitStatus,
    handle: resource::ProcessFdHandle,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_FD_PROCESS_FD_TRY_WAIT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.processFdTryWait",
    ))
    .boxed())
}

/// Stub for destack.process.fd.processFdWait.
pub unsafe fn destack_process_process_fd_wait(
    context: &RuntimeCallContext,
    out: *mut ProcessWaitStatus,
    handle: resource::ProcessFdHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_FD_PROCESS_FD_WAIT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.processFdWait",
    ))
    .boxed())
}

/// Stub for destack.process.fd.signalFdClose.
pub unsafe fn destack_process_signal_fd_close(
    context: &RuntimeCallContext,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_FD_SIGNAL_FD_CLOSE)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdClose",
    ))
    .boxed())
}

/// Stub for destack.process.fd.signalFdOpen.
pub unsafe fn destack_process_signal_fd_open(
    context: &RuntimeCallContext,
    out: *mut resource::SignalFdHandle,
    signals: NativeSlice<Signal>,
    flags: SignalFdFlags,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_FD_SIGNAL_FD_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, signals, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdOpen",
    ))
    .boxed())
}

/// Stub for destack.process.fd.signalFdRead.
pub unsafe fn destack_process_signal_fd_read(
    context: &RuntimeCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_FD_SIGNAL_FD_READ)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdRead",
    ))
    .boxed())
}

/// Stub for destack.process.fd.signalFdSetMask.
pub unsafe fn destack_process_signal_fd_set_mask(
    context: &RuntimeCallContext,
    handle: resource::SignalFdHandle,
    signals: NativeSlice<Signal>,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_FD_SIGNAL_FD_SET_MASK)?;
    let _ = (handle, signals);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdSetMask",
    ))
    .boxed())
}

/// Stub for destack.process.fd.signalFdTryRead.
pub unsafe fn destack_process_signal_fd_try_read(
    context: &RuntimeCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_FD_SIGNAL_FD_TRY_READ)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdTryRead",
    ))
    .boxed())
}

/// Stub for destack.process.group.cgroupGetLimit.
pub unsafe fn destack_process_cgroup_get_limit(
    context: &RuntimeCallContext,
    out: *mut ProcessLimit,
    path: NativeStringRef,
    resource: ProcessLimitResource,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_GROUP_CGROUP_GET_LIMIT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, path, resource);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.cgroupGetLimit",
    ))
    .boxed())
}

/// Stub for destack.process.group.cgroupJoin.
pub unsafe fn destack_process_cgroup_join(
    context: &RuntimeCallContext,
    path: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_GROUP_CGROUP_JOIN)?;
    let _ = path;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.cgroupJoin",
    ))
    .boxed())
}

/// Stub for destack.process.group.cgroupSetLimit.
pub unsafe fn destack_process_cgroup_set_limit(
    context: &RuntimeCallContext,
    path: NativeStringRef,
    resource: ProcessLimitResource,
    limit: ProcessLimit,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_GROUP_CGROUP_SET_LIMIT)?;
    let _ = (path, resource, limit);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.cgroupSetLimit",
    ))
    .boxed())
}

/// Stub for destack.process.group.jobAssign.
pub unsafe fn destack_process_job_assign(
    context: &RuntimeCallContext,
    name: NativeStringRef,
    pids: NativeSlice<ProcessId>,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_GROUP_JOB_ASSIGN)?;
    let _ = (name, pids);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.jobAssign",
    ))
    .boxed())
}

/// Stub for destack.process.group.jobSetLimit.
pub unsafe fn destack_process_job_set_limit(
    context: &RuntimeCallContext,
    name: NativeStringRef,
    resource: ProcessLimitResource,
    limit: ProcessLimit,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_GROUP_JOB_SET_LIMIT)?;
    let _ = (name, resource, limit);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.jobSetLimit",
    ))
    .boxed())
}

/// Stub for destack.process.ids.gid.
pub unsafe fn destack_process_gid(
    context: &RuntimeCallContext,
    out: *mut GroupId,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_IDS_GID)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.gid")).boxed())
}

/// Stub for destack.process.ids.pid.
pub unsafe fn destack_process_pid(
    context: &RuntimeCallContext,
    out: *mut ProcessId,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_IDS_PID)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.pid")).boxed())
}

/// Stub for destack.process.ids.ppid.
pub unsafe fn destack_process_ppid(
    context: &RuntimeCallContext,
    out: *mut ProcessId,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_IDS_PPID)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.ppid")).boxed())
}

/// Stub for destack.process.ids.setGid.
pub unsafe fn destack_process_set_gid(
    context: &RuntimeCallContext,
    groupid: GroupId,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_IDS_SET_GID)?;
    let _ = groupid;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.setGid")).boxed())
}

/// Stub for destack.process.ids.setGroups.
pub unsafe fn destack_process_set_groups(
    context: &RuntimeCallContext,
    groups: NativeSlice<GroupId>,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_IDS_SET_GROUPS)?;
    let _ = groups;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.ids.setGroups",
    ))
    .boxed())
}

/// Stub for destack.process.ids.setUid.
pub unsafe fn destack_process_set_uid(
    context: &RuntimeCallContext,
    userid: UserId,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_IDS_SET_UID)?;
    let _ = userid;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.setUid")).boxed())
}

/// Stub for destack.process.ids.uid.
pub unsafe fn destack_process_uid(
    context: &RuntimeCallContext,
    out: *mut UserId,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_IDS_UID)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.uid")).boxed())
}

/// Stub for destack.process.isolation.chroot.
pub unsafe fn destack_process_chroot(
    context: &RuntimeCallContext,
    path: fs::OsPath,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_ISOLATION_CHROOT)?;
    let _ = path;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.chroot",
    ))
    .boxed())
}

/// Stub for destack.process.isolation.installSyscallFilter.
pub unsafe fn destack_process_install_syscall_filter(
    context: &RuntimeCallContext,
    program: NativeArray<u8>,
    flags: SyscallFilterFlags,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_ISOLATION_INSTALL_SYSCALL_FILTER)?;
    let _ = (program, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.installSyscallFilter",
    ))
    .boxed())
}

/// Stub for destack.process.isolation.setHostName.
pub unsafe fn destack_process_set_host_name(
    context: &RuntimeCallContext,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_ISOLATION_SET_HOST_NAME)?;
    let _ = name;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.setHostName",
    ))
    .boxed())
}

/// Stub for destack.process.isolation.setNetworkNamespace.
pub unsafe fn destack_process_set_network_namespace(
    context: &RuntimeCallContext,
    path: fs::OsPath,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_ISOLATION_SET_NETWORK_NAMESPACE)?;
    let _ = path;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.setNetworkNamespace",
    ))
    .boxed())
}

/// Stub for destack.process.isolation.setns.
pub unsafe fn destack_process_setns(
    context: &RuntimeCallContext,
    pid: ProcessId,
    namespace: ProcessNamespaceKind,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_ISOLATION_SETNS)?;
    let _ = (pid, namespace);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.setns",
    ))
    .boxed())
}

/// Stub for destack.process.isolation.unshare.
pub unsafe fn destack_process_unshare(
    context: &RuntimeCallContext,
    flags: ProcessUnshareFlags,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_ISOLATION_UNSHARE)?;
    let _ = flags;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.unshare",
    ))
    .boxed())
}

/// Stub for destack.process.limits.getLimit.
pub unsafe fn destack_process_get_limit(
    context: &RuntimeCallContext,
    out: *mut ProcessLimit,
    resource: ProcessLimitResource,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_LIMITS_GET_LIMIT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, resource);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.limits.getLimit",
    ))
    .boxed())
}

/// Stub for destack.process.limits.setLimit.
pub unsafe fn destack_process_set_limit(
    context: &RuntimeCallContext,
    resource: ProcessLimitResource,
    limit: ProcessLimit,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_LIMITS_SET_LIMIT)?;
    let _ = (resource, limit);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.limits.setLimit",
    ))
    .boxed())
}

/// Stub for destack.process.sched.getAffinity.
pub unsafe fn destack_process_get_affinity(
    context: &RuntimeCallContext,
    out: *mut ProcessCpuSet,
    pid: ProcessId,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SCHED_GET_AFFINITY)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, pid);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.getAffinity",
    ))
    .boxed())
}

/// Stub for destack.process.sched.getPriority.
pub unsafe fn destack_process_get_priority(
    context: &RuntimeCallContext,
    out: *mut i32,
    pid: ProcessId,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SCHED_GET_PRIORITY)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, pid);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.getPriority",
    ))
    .boxed())
}

/// Stub for destack.process.sched.setAffinity.
pub unsafe fn destack_process_set_affinity(
    context: &RuntimeCallContext,
    pid: ProcessId,
    cpus: ProcessCpuSet,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SCHED_SET_AFFINITY)?;
    let _ = (pid, cpus);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.setAffinity",
    ))
    .boxed())
}

/// Stub for destack.process.sched.setPriority.
pub unsafe fn destack_process_set_priority(
    context: &RuntimeCallContext,
    pid: ProcessId,
    priority: i32,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SCHED_SET_PRIORITY)?;
    let _ = (pid, priority);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.setPriority",
    ))
    .boxed())
}

/// Stub for destack.process.session.getpgid.
pub unsafe fn destack_process_getpgid(
    context: &RuntimeCallContext,
    out: *mut ProcessId,
    pid: ProcessId,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SESSION_GETPGID)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, pid);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.session.getpgid",
    ))
    .boxed())
}

/// Stub for destack.process.session.setpgid.
pub unsafe fn destack_process_setpgid(
    context: &RuntimeCallContext,
    pid: ProcessId,
    pgid: ProcessId,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SESSION_SETPGID)?;
    let _ = (pid, pgid);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.session.setpgid",
    ))
    .boxed())
}

/// Stub for destack.process.session.setsid.
pub unsafe fn destack_process_setsid(
    context: &RuntimeCallContext,
    out: *mut ProcessId,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SESSION_SETSID)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.session.setsid",
    ))
    .boxed())
}

/// Stub for destack.process.signals.kill.
pub unsafe fn destack_process_kill(
    context: &RuntimeCallContext,
    pid: ProcessId,
    signal: Signal,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SIGNALS_KILL)?;
    let _ = (pid, signal);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.signals.kill")).boxed())
}

/// Stub for destack.process.signals.signalMaskRead.
pub unsafe fn destack_process_signal_mask_read(
    context: &RuntimeCallContext,
    out: *mut NativeArray<Signal>,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SIGNALS_SIGNAL_MASK_READ)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalMaskRead",
    ))
    .boxed())
}

/// Stub for destack.process.signals.signalMaskUpdate.
pub unsafe fn destack_process_signal_mask_update(
    context: &RuntimeCallContext,
    how: SignalMaskHow,
    signals: NativeSlice<Signal>,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SIGNALS_SIGNAL_MASK_UPDATE)?;
    let _ = (how, signals);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalMaskUpdate",
    ))
    .boxed())
}

/// Stub for destack.process.signals.signalReceive.
pub unsafe fn destack_process_signal_receive(
    context: &RuntimeCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalHandle,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SIGNALS_SIGNAL_RECEIVE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalReceive",
    ))
    .boxed())
}

/// Stub for destack.process.signals.signalSubscribe.
pub unsafe fn destack_process_signal_subscribe(
    context: &RuntimeCallContext,
    out: *mut resource::SignalHandle,
    signal: Signal,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SIGNALS_SIGNAL_SUBSCRIBE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, signal);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalSubscribe",
    ))
    .boxed())
}

/// Stub for destack.process.signals.signalTryReceive.
pub unsafe fn destack_process_signal_try_receive(
    context: &RuntimeCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalHandle,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SIGNALS_SIGNAL_TRY_RECEIVE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalTryReceive",
    ))
    .boxed())
}

/// Stub for destack.process.signals.signalTryWait.
pub unsafe fn destack_process_signal_try_wait(
    context: &RuntimeCallContext,
    out: *mut SignalEvent,
    signals: NativeSlice<Signal>,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SIGNALS_SIGNAL_TRY_WAIT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, signals);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalTryWait",
    ))
    .boxed())
}

/// Stub for destack.process.signals.signalUnsubscribe.
pub unsafe fn destack_process_signal_unsubscribe(
    context: &RuntimeCallContext,
    handle: resource::SignalHandle,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SIGNALS_SIGNAL_UNSUBSCRIBE)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalUnsubscribe",
    ))
    .boxed())
}

/// Stub for destack.process.signals.signalWait.
pub unsafe fn destack_process_signal_wait(
    context: &RuntimeCallContext,
    out: *mut SignalEvent,
    signals: NativeSlice<Signal>,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SIGNALS_SIGNAL_WAIT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, signals);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalWait",
    ))
    .boxed())
}

/// Stub for destack.process.spawn.spawn.
pub unsafe fn destack_process_spawn(
    context: &RuntimeCallContext,
    out: *mut resource::ProcessHandle,
    command: fs::OsPath,
    arguments: NativeStringSlice,
    environment: NativeStringSlice,
    options: ProcessSpawnOptions,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SPAWN_SPAWN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, command, arguments, environment, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.spawn.spawn")).boxed())
}

/// Stub for destack.process.spawn.withActions.
pub unsafe fn destack_process_spawn_with_actions(
    context: &RuntimeCallContext,
    out: *mut resource::ProcessHandle,
    command: fs::OsPath,
    arguments: NativeStringSlice,
    environment: NativeStringSlice,
    options: ProcessSpawnOptions,
    stdio: NativeSlice<ProcessStdio>,
    actions: NativeSlice<ProcessFdAction>,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_SPAWN_WITH_ACTIONS)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (
        out,
        command,
        arguments,
        environment,
        options,
        stdio,
        actions,
    );

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.spawn.withActions",
    ))
    .boxed())
}

/// Stub for destack.process.umask.umask.
pub unsafe fn destack_process_umask(
    context: &RuntimeCallContext,
    out: *mut u32,
    mask: u32,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_UMASK_UMASK)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, mask);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.umask.umask")).boxed())
}

/// Stub for destack.process.wait.pid.
pub unsafe fn destack_process_wait_pid(
    context: &RuntimeCallContext,
    out: *mut ProcessWaitStatus,
    pid: ProcessId,
    flags: ProcessWaitFlags,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_WAIT_PID)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, pid, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.wait.pid")).boxed())
}

/// Stub for destack.process.wait.tryWait.
pub unsafe fn destack_process_try_wait(
    context: &RuntimeCallContext,
    out: *mut ProcessWaitStatus,
    handle: resource::ProcessHandle,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_WAIT_TRY_WAIT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.wait.tryWait")).boxed())
}

/// Stub for destack.process.wait.wait.
pub unsafe fn destack_process_wait(
    context: &RuntimeCallContext,
    out: *mut ProcessWaitStatus,
    handle: resource::ProcessHandle,
    flags: ProcessWaitFlags,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_WAIT_WAIT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.wait.wait")).boxed())
}
