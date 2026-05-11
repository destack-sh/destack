#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef, NativeStringSlice};
use crate::platform::process::bindings_generated as bindings;
use crate::platform::{NativeArray, PlatformError};

use crate::runtime::BindingCallContext;
use bindings::*;

use crate::platform::process::{
    ExecAtFlags, GroupId, ProcessCpuSet, ProcessFdAction, ProcessFdFlags, ProcessFdSignalFlags,
    ProcessGroupIds, ProcessId, ProcessLimit, ProcessLimitResource, ProcessNamespaceKind,
    ProcessSchedulerConfig, ProcessSchedulerPolicy, ProcessSpawnOptions, ProcessStdio,
    ProcessUnshareFlags, ProcessUserIds, ProcessWaitFlags, ProcessWaitStatus, Signal, SignalEvent,
    SignalFdFlags, SignalMaskHow, SyscallFilterFlags, UserId,
};
use crate::platform::{fs, resource};

/// Return the process argument vector.
pub(crate) unsafe fn destack_process_args(
    binding: &BindingCallContext,
    out: *mut NativeStringSlice,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.args.list")).boxed())
}

/// Change the current working directory.
pub(crate) unsafe fn destack_process_chdir(
    binding: &BindingCallContext,
    path: fs::OsPath,
) -> RuntimeResult<()> {
    let _ = path;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.cwd.chdir")).boxed())
}

/// Return the current working directory.
pub(crate) unsafe fn destack_process_cwd(
    binding: &BindingCallContext,
    out: *mut fs::OsPath,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.cwd.get")).boxed())
}

/// Delete an environment variable by UTF-8 name.
pub(crate) unsafe fn destack_process_env_delete(
    binding: &BindingCallContext,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = name;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.env.delete")).boxed())
}

/// Delete an environment variable by raw byte name.
pub(crate) unsafe fn destack_process_env_delete_bytes(
    binding: &BindingCallContext,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = name;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.env.deleteBytes",
    ))
    .boxed())
}

/// Read an environment variable by UTF-8 name.
pub(crate) unsafe fn destack_process_env_get(
    binding: &BindingCallContext,
    out: *mut NativeStringRef,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, name);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.env.get")).boxed())
}

/// Read an environment variable by raw byte name.
pub(crate) unsafe fn destack_process_env_get_bytes(
    binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, name);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.env.getBytes")).boxed())
}

/// Set an environment variable by UTF-8 name and value.
pub(crate) unsafe fn destack_process_env_set(
    binding: &BindingCallContext,
    name: NativeStringRef,
    argument_value: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (name, argument_value);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.env.set")).boxed())
}

/// Set an environment variable by raw byte name and value.
pub(crate) unsafe fn destack_process_env_set_bytes(
    binding: &BindingCallContext,
    name: NativeSlice<u8>,
    argument_value: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (name, argument_value);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.env.setBytes")).boxed())
}

/// Replace the current process image with a command path.
pub(crate) unsafe fn destack_process_exec(
    binding: &BindingCallContext,
    command: fs::OsPath,
    arguments: NativeStringSlice,
    environment: NativeStringSlice,
) -> RuntimeResult<()> {
    let _ = (command, arguments, environment);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.exec.path")).boxed())
}

/// Replace the current process image using a directory-relative path.
pub(crate) unsafe fn destack_process_execat(
    binding: &BindingCallContext,
    directory: resource::DirectoryHandle,
    path: fs::OsPath,
    arguments: NativeStringSlice,
    environment: NativeStringSlice,
    flags: ExecAtFlags,
) -> RuntimeResult<()> {
    let _ = (directory, path, arguments, environment, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.exec.pathat")).boxed())
}

/// Replace the current process image using an executable file handle.
pub(crate) unsafe fn destack_process_fexec(
    binding: &BindingCallContext,
    executable: resource::FileHandle,
    arguments: NativeStringSlice,
    environment: NativeStringSlice,
) -> RuntimeResult<()> {
    let _ = (executable, arguments, environment);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.exec.fexec")).boxed())
}

/// Exit the current process with the given code.
pub(crate) unsafe fn destack_process_exit(
    binding: &BindingCallContext,
    code: u32,
) -> RuntimeResult<()> {
    let _ = code;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.exit.terminate",
    ))
    .boxed())
}

/// Close one process descriptor.
pub(crate) unsafe fn destack_process_process_fd_close(
    binding: &BindingCallContext,
    handle: resource::ProcessFdHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.processFdClose",
    ))
    .boxed())
}

/// Open one process descriptor for the target process id.
pub(crate) unsafe fn destack_process_process_fd_open(
    binding: &BindingCallContext,
    out: *mut resource::ProcessFdHandle,
    pid: ProcessId,
    flags: ProcessFdFlags,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, pid, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.processFdOpen",
    ))
    .boxed())
}

/// Send one signal through a process descriptor.
pub(crate) unsafe fn destack_process_process_fd_send_signal(
    binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_process_process_fd_try_wait(
    binding: &BindingCallContext,
    out: *mut ProcessWaitStatus,
    handle: resource::ProcessFdHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.processFdTryWait",
    ))
    .boxed())
}

/// Wait for one process descriptor state transition.
pub(crate) unsafe fn destack_process_process_fd_wait(
    binding: &BindingCallContext,
    out: *mut ProcessWaitStatus,
    handle: resource::ProcessFdHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.processFdWait",
    ))
    .boxed())
}

/// Close one signal descriptor.
pub(crate) unsafe fn destack_process_signal_fd_close(
    binding: &BindingCallContext,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdClose",
    ))
    .boxed())
}

/// Open one signal descriptor for the provided signal mask.
pub(crate) unsafe fn destack_process_signal_fd_open(
    binding: &BindingCallContext,
    out: *mut resource::SignalFdHandle,
    signals: NativeSlice<Signal>,
    flags: SignalFdFlags,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, signals, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdOpen",
    ))
    .boxed())
}

/// Read one queued signal event from a signal descriptor.
pub(crate) unsafe fn destack_process_signal_fd_read(
    binding: &BindingCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdRead",
    ))
    .boxed())
}

/// Replace the active signal mask for one signal descriptor.
pub(crate) unsafe fn destack_process_signal_fd_set_mask(
    binding: &BindingCallContext,
    handle: resource::SignalFdHandle,
    signals: NativeSlice<Signal>,
) -> RuntimeResult<()> {
    let _ = (handle, signals);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdSetMask",
    ))
    .boxed())
}

/// Poll one queued signal event from a signal descriptor without blocking.
pub(crate) unsafe fn destack_process_signal_fd_try_read(
    binding: &BindingCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalFdHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.signalFdTryRead",
    ))
    .boxed())
}

/// Open one standard error stream handle.
pub(crate) unsafe fn destack_process_stdio_stderr(
    binding: &BindingCallContext,
    out: *mut resource::FileHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.stdioStderr",
    ))
    .boxed())
}

/// Open one standard input stream handle.
pub(crate) unsafe fn destack_process_stdio_stdin(
    binding: &BindingCallContext,
    out: *mut resource::FileHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.stdioStdin",
    ))
    .boxed())
}

/// Open one standard output stream handle.
pub(crate) unsafe fn destack_process_stdio_stdout(
    binding: &BindingCallContext,
    out: *mut resource::FileHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.fd.stdioStdout",
    ))
    .boxed())
}

/// Read one control-group resource limit.
pub(crate) unsafe fn destack_process_cgroup_get_limit(
    binding: &BindingCallContext,
    out: *mut ProcessLimit,
    path: NativeStringRef,
    resource: ProcessLimitResource,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, path, resource);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.cgroupGetLimit",
    ))
    .boxed())
}

/// Join one control group.
pub(crate) unsafe fn destack_process_cgroup_join(
    binding: &BindingCallContext,
    path: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = path;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.cgroupJoin",
    ))
    .boxed())
}

/// Write one control-group resource limit.
pub(crate) unsafe fn destack_process_cgroup_set_limit(
    binding: &BindingCallContext,
    path: NativeStringRef,
    resource: ProcessLimitResource,
    limit: ProcessLimit,
) -> RuntimeResult<()> {
    let _ = (path, resource, limit);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.cgroupSetLimit",
    ))
    .boxed())
}

/// Assign processes to one Windows job object.
pub(crate) unsafe fn destack_process_job_assign(
    binding: &BindingCallContext,
    name: NativeStringRef,
    pids: NativeSlice<ProcessId>,
) -> RuntimeResult<()> {
    let _ = (name, pids);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.jobAssign",
    ))
    .boxed())
}

/// Set one Windows job object resource limit.
pub(crate) unsafe fn destack_process_job_set_limit(
    binding: &BindingCallContext,
    name: NativeStringRef,
    resource: ProcessLimitResource,
    limit: ProcessLimit,
) -> RuntimeResult<()> {
    let _ = (name, resource, limit);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.group.jobSetLimit",
    ))
    .boxed())
}

/// Return the effective group identifier.
pub(crate) unsafe fn destack_process_egid(
    binding: &BindingCallContext,
    out: *mut GroupId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.egid")).boxed())
}

/// Return the effective user identifier.
pub(crate) unsafe fn destack_process_euid(
    binding: &BindingCallContext,
    out: *mut UserId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.euid")).boxed())
}

/// Return the current group identifier.
pub(crate) unsafe fn destack_process_gid(
    binding: &BindingCallContext,
    out: *mut GroupId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.gid")).boxed())
}

/// Return real, effective, and saved-set group identifiers.
pub(crate) unsafe fn destack_process_group_ids(
    binding: &BindingCallContext,
    out: *mut ProcessGroupIds,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.groupIds")).boxed())
}

/// Return supplementary group identifiers.
pub(crate) unsafe fn destack_process_groups(
    binding: &BindingCallContext,
    out: *mut NativeSlice<GroupId>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.groups")).boxed())
}

/// Return the current process identifier.
pub(crate) unsafe fn destack_process_pid(
    binding: &BindingCallContext,
    out: *mut ProcessId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.pid")).boxed())
}

/// Return the parent process identifier.
pub(crate) unsafe fn destack_process_ppid(
    binding: &BindingCallContext,
    out: *mut ProcessId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.ppid")).boxed())
}

/// Set the effective group identifier only.
pub(crate) unsafe fn destack_process_set_egid(
    binding: &BindingCallContext,
    groupid: GroupId,
) -> RuntimeResult<()> {
    let _ = groupid;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.setEgid")).boxed())
}

/// Set the effective user identifier only.
pub(crate) unsafe fn destack_process_set_euid(
    binding: &BindingCallContext,
    userid: UserId,
) -> RuntimeResult<()> {
    let _ = userid;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.setEuid")).boxed())
}

/// Set the effective group identifier.
pub(crate) unsafe fn destack_process_set_gid(
    binding: &BindingCallContext,
    groupid: GroupId,
) -> RuntimeResult<()> {
    let _ = groupid;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.setGid")).boxed())
}

/// Set real, effective, and saved-set group identifiers together.
pub(crate) unsafe fn destack_process_set_group_ids(
    binding: &BindingCallContext,
    ids: ProcessGroupIds,
) -> RuntimeResult<()> {
    let _ = ids;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.ids.setGroupIds",
    ))
    .boxed())
}

/// Set supplementary group identifiers.
pub(crate) unsafe fn destack_process_set_groups(
    binding: &BindingCallContext,
    groups: NativeSlice<GroupId>,
) -> RuntimeResult<()> {
    let _ = groups;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.ids.setGroups",
    ))
    .boxed())
}

/// Set the effective user identifier.
pub(crate) unsafe fn destack_process_set_uid(
    binding: &BindingCallContext,
    userid: UserId,
) -> RuntimeResult<()> {
    let _ = userid;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.setUid")).boxed())
}

/// Set real, effective, and saved-set user identifiers together.
pub(crate) unsafe fn destack_process_set_user_ids(
    binding: &BindingCallContext,
    ids: ProcessUserIds,
) -> RuntimeResult<()> {
    let _ = ids;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.ids.setUserIds",
    ))
    .boxed())
}

/// Return the current user identifier.
pub(crate) unsafe fn destack_process_uid(
    binding: &BindingCallContext,
    out: *mut UserId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.uid")).boxed())
}

/// Return real, effective, and saved-set user identifiers.
pub(crate) unsafe fn destack_process_user_ids(
    binding: &BindingCallContext,
    out: *mut ProcessUserIds,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.userIds")).boxed())
}

/// Change the root directory for path resolution.
pub(crate) unsafe fn destack_process_chroot(
    binding: &BindingCallContext,
    path: fs::OsPath,
) -> RuntimeResult<()> {
    let _ = path;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.chroot",
    ))
    .boxed())
}

/// Install one syscall filter program.
pub(crate) unsafe fn destack_process_install_syscall_filter(
    binding: &BindingCallContext,
    program: NativeArray<u8>,
    flags: SyscallFilterFlags,
) -> RuntimeResult<()> {
    let _ = (program, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.installSyscallFilter",
    ))
    .boxed())
}

/// Set process host name inside the active UTS namespace.
pub(crate) unsafe fn destack_process_set_host_name(
    binding: &BindingCallContext,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = name;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.setHostName",
    ))
    .boxed())
}

/// Set network namespace context for subsequent network operations.
pub(crate) unsafe fn destack_process_set_network_namespace(
    binding: &BindingCallContext,
    path: fs::OsPath,
) -> RuntimeResult<()> {
    let _ = path;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.setNetworkNamespace",
    ))
    .boxed())
}

/// Enter one namespace owned by another process.
pub(crate) unsafe fn destack_process_setns(
    binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_process_unshare(
    binding: &BindingCallContext,
    flags: ProcessUnshareFlags,
) -> RuntimeResult<()> {
    let _ = flags;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.isolation.unshare",
    ))
    .boxed())
}

/// Read a process resource limit.
pub(crate) unsafe fn destack_process_get_limit(
    binding: &BindingCallContext,
    out: *mut ProcessLimit,
    resource: ProcessLimitResource,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, resource);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.limits.getLimit",
    ))
    .boxed())
}

/// Set a process resource limit.
pub(crate) unsafe fn destack_process_set_limit(
    binding: &BindingCallContext,
    resource: ProcessLimitResource,
    limit: ProcessLimit,
) -> RuntimeResult<()> {
    let _ = (resource, limit);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.limits.setLimit",
    ))
    .boxed())
}

/// Read process CPU affinity.
pub(crate) unsafe fn destack_process_get_affinity(
    binding: &BindingCallContext,
    out: *mut ProcessCpuSet,
    pid: ProcessId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, pid);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.getAffinity",
    ))
    .boxed())
}

/// Read a process priority value.
pub(crate) unsafe fn destack_process_get_priority(
    binding: &BindingCallContext,
    out: *mut i32,
    pid: ProcessId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, pid);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.getPriority",
    ))
    .boxed())
}

/// Read scheduler policy and priority for a process.
pub(crate) unsafe fn destack_process_get_scheduler(
    binding: &BindingCallContext,
    out: *mut ProcessSchedulerConfig,
    pid: ProcessId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, pid);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.getScheduler",
    ))
    .boxed())
}

/// Set process CPU affinity.
pub(crate) unsafe fn destack_process_set_affinity(
    binding: &BindingCallContext,
    pid: ProcessId,
    cpus: ProcessCpuSet,
) -> RuntimeResult<()> {
    let _ = (pid, cpus);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.setAffinity",
    ))
    .boxed())
}

/// Set a process priority value.
pub(crate) unsafe fn destack_process_set_priority(
    binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_process_set_scheduler(
    binding: &BindingCallContext,
    pid: ProcessId,
    config: ProcessSchedulerConfig,
) -> RuntimeResult<()> {
    let _ = (pid, config);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.setScheduler",
    ))
    .boxed())
}

/// Yield the current thread to the scheduler.
pub(crate) unsafe fn destack_process_yield_now(binding: &BindingCallContext) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.sched.yieldNow",
    ))
    .boxed())
}

/// Read a process group id.
pub(crate) unsafe fn destack_process_getpgid(
    binding: &BindingCallContext,
    out: *mut ProcessId,
    pid: ProcessId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, pid);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.session.getpgid",
    ))
    .boxed())
}

/// Set a process group id for a process.
pub(crate) unsafe fn destack_process_setpgid(
    binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_process_setsid(
    binding: &BindingCallContext,
    out: *mut ProcessId,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.session.setsid",
    ))
    .boxed())
}

/// Send a signal to a target process.
pub(crate) unsafe fn destack_process_kill(
    binding: &BindingCallContext,
    pid: ProcessId,
    signal: Signal,
) -> RuntimeResult<()> {
    let _ = (pid, signal);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.signals.kill")).boxed())
}

/// Read the current thread signal mask.
pub(crate) unsafe fn destack_process_signal_mask_read(
    binding: &BindingCallContext,
    out: *mut NativeArray<Signal>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalMaskRead",
    ))
    .boxed())
}

/// Update the current thread signal mask.
pub(crate) unsafe fn destack_process_signal_mask_update(
    binding: &BindingCallContext,
    how: SignalMaskHow,
    signals: NativeSlice<Signal>,
) -> RuntimeResult<()> {
    let _ = (how, signals);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalMaskUpdate",
    ))
    .boxed())
}

/// Receive the next signal event from a subscription.
pub(crate) unsafe fn destack_process_signal_receive(
    binding: &BindingCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalReceive",
    ))
    .boxed())
}

/// Subscribe to one signal value.
pub(crate) unsafe fn destack_process_signal_subscribe(
    binding: &BindingCallContext,
    out: *mut resource::SignalHandle,
    signal: Signal,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, signal);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalSubscribe",
    ))
    .boxed())
}

/// Poll one signal event without blocking.
pub(crate) unsafe fn destack_process_signal_try_receive(
    binding: &BindingCallContext,
    out: *mut SignalEvent,
    handle: resource::SignalHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalTryReceive",
    ))
    .boxed())
}

/// Poll one signal from a requested set without blocking.
pub(crate) unsafe fn destack_process_signal_try_wait(
    binding: &BindingCallContext,
    out: *mut SignalEvent,
    signals: NativeSlice<Signal>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, signals);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalTryWait",
    ))
    .boxed())
}

/// Remove a signal subscription handle.
pub(crate) unsafe fn destack_process_signal_unsubscribe(
    binding: &BindingCallContext,
    handle: resource::SignalHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalUnsubscribe",
    ))
    .boxed())
}

/// Wait for one signal from a requested set.
pub(crate) unsafe fn destack_process_signal_wait(
    binding: &BindingCallContext,
    out: *mut SignalEvent,
    signals: NativeSlice<Signal>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, signals);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.process.signals.signalWait",
    ))
    .boxed())
}

/// Spawn a child process with default stdio inheritance.
pub(crate) unsafe fn destack_process_spawn(
    binding: &BindingCallContext,
    out: *mut resource::ProcessHandle,
    command: fs::OsPath,
    arguments: NativeStringSlice,
    environment: NativeStringSlice,
    options: ProcessSpawnOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, command, arguments, environment, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.spawn.start")).boxed())
}

/// Spawn a child process with explicit stdio and descriptor actions.
pub(crate) unsafe fn destack_process_spawn_with_actions(
    binding: &BindingCallContext,
    out: *mut resource::ProcessHandle,
    command: fs::OsPath,
    arguments: NativeStringSlice,
    environment: NativeStringSlice,
    options: ProcessSpawnOptions,
    stdio: NativeSlice<ProcessStdio>,
    actions: NativeSlice<ProcessFdAction>,
) -> RuntimeResult<()> {
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

/// Set process file-creation umask and return the previous value.
pub(crate) unsafe fn destack_process_umask(
    binding: &BindingCallContext,
    out: *mut u32,
    mask: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, mask);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.umask.set")).boxed())
}

/// Wait for a process identifier.
pub(crate) unsafe fn destack_process_wait_pid(
    binding: &BindingCallContext,
    out: *mut ProcessWaitStatus,
    pid: ProcessId,
    flags: ProcessWaitFlags,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, pid, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.wait.pid")).boxed())
}

/// Poll a child process handle without blocking.
pub(crate) unsafe fn destack_process_try_wait(
    binding: &BindingCallContext,
    out: *mut ProcessWaitStatus,
    handle: resource::ProcessHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.wait.tryWait")).boxed())
}

/// Wait for a child process handle.
pub(crate) unsafe fn destack_process_wait(
    binding: &BindingCallContext,
    out: *mut ProcessWaitStatus,
    handle: resource::ProcessHandle,
    flags: ProcessWaitFlags,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.process.wait.handle")).boxed())
}
