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
///
/// Read the immutable argument list captured by the runtime at process startup.
/// Argument decoding and quoting semantics follow the host process loader.
///
/// # Platform
/// Unix and Windows.
/// Uses startup argument capture from the host process loader.
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<vm::StringHandle>> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.args.list")).boxed())
}

/// Change the current working directory.
///
/// Update process working directory state for subsequent relative path resolution.
/// Directory existence and permission checks are performed by the host kernel.
///
/// # Platform
/// Unix and Windows.
/// Uses chdir(2) on Unix and SetCurrentDirectoryW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.workdir.write`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_chdir(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: fs::OsPathVm,
) -> RuntimeResult<()> {
    let _ = path;
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.cwd.chdir")).boxed())
}

/// Return the current working directory.
///
/// Query the calling process working directory without changing process state.
/// The returned path preserves host encoding through `OsPath`.
///
/// # Platform
/// Unix and Windows.
/// Uses getcwd(3) on Unix and GetCurrentDirectoryW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.workdir.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_cwd(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<fs::OsPathVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.cwd.get")).boxed())
}

/// Delete an environment variable by UTF-8 name.
///
/// Remove one key from the process environment block.
/// Missing keys are handled according to host environment semantics.
///
/// # Platform
/// Unix and Windows.
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = name;
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.env.delete")).boxed())
}

/// Delete an environment variable by raw byte name.
///
/// Remove one key from the environment block without UTF-8 normalization.
/// This is intended for byte-level Unix-style environment access.
///
/// # Platform
/// Unix and Windows.
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
///
/// Resolve one key from the process environment block and decode it as a runtime string.
/// Missing keys and invalid entries are surfaced as platform errors.
///
/// # Platform
/// Unix and Windows.
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    name: vm::StringHandle,
) -> RuntimeResult<vm::StringHandle> {
    let _ = name;
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.env.get")).boxed())
}

/// Read an environment variable by raw byte name.
///
/// Resolve one key from the process environment block without UTF-8 normalization.
/// This is intended for byte-level Unix-style environment access.
///
/// # Platform
/// Unix and Windows.
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    name: VmSlice<u8>,
) -> RuntimeResult<VmArray<u8>> {
    let _ = name;
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.env.getBytes")).boxed())
}

/// Set an environment variable by UTF-8 name and value.
///
/// Insert or replace one key-value pair in the process environment block.
/// Persistence and inheritance semantics follow host process-spawn rules.
///
/// # Platform
/// Unix and Windows.
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    name: vm::StringHandle,
    argument_value: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (name, argument_value);
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.env.set")).boxed())
}

/// Set an environment variable by raw byte name and value.
///
/// Insert or replace one key-value pair in the environment block without UTF-8 normalization.
/// This is intended for byte-level Unix-style environment access.
///
/// # Platform
/// Unix and Windows.
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    name: VmSlice<u8>,
    argument_value: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (name, argument_value);
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.env.setBytes")).boxed())
}

/// Replace the current process image with a command path.
///
/// Replace the current process image in-place by executing the given command path.
/// This call does not return on success and preserves host exec semantics for inherited descriptors.
///
/// # Platform
/// Unix and Windows.
/// Uses execve(2) on Unix and process-replacement emulation with CreateProcessW plus exit on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.exec`.
///
/// # Replay
/// External, recordable.
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
///
/// Replace the current process image in-place by executing a directory-relative target.
/// Flag behavior follows host exec-at semantics and may reject unsupported combinations.
///
/// # Platform
/// Unix and Windows.
/// Uses execveat(2) on Unix where available and runtime fallback or `notSupported` on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.exec`.
///
/// # Replay
/// External, recordable.
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
///
/// Replace the current process image in-place from an already-open executable descriptor.
/// Descriptor validity, executable format, and permission checks are enforced by the host kernel.
///
/// # Platform
/// Unix and Windows.
/// Uses fexecve(2) on Unix where available and runtime fallback or `notSupported` on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.exec`.
///
/// # Replay
/// External, recordable.
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
///
/// Terminate the current process without returning to the caller.
/// Exit code interpretation is host-defined and propagated to the parent process.
///
/// # Platform
/// Unix and Windows.
/// Uses _exit(2) or exit(3) on Unix and ExitProcess on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.run`.
///
/// # Replay
/// External, recordable.
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
///
/// Close one host process descriptor and release the kernel object reference.
/// Closing semantics follow host descriptor teardown behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses close(2) on Unix and CloseHandle on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.handle`.
///
/// # Replay
/// External, recordable.
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
///
/// Open one host process descriptor that can be used for wait and signal operations without pid reuse races.
/// Descriptor semantics follow pidfd on Linux and host-equivalent process-handle semantics on other targets.
///
/// # Platform
/// Unix and Windows.
/// Uses pidfd_open(2) on Linux and process handle duplication on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.handle`.
///
/// # Replay
/// External, recordable.
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
///
/// Deliver one signal using a stable process descriptor rather than a numeric pid.
/// Delivery semantics follow pidfd_send_signal on Linux and host-equivalent process-signal APIs on other targets.
///
/// # Platform
/// Unix and Windows.
/// Uses pidfd_send_signal(2) on Linux and process-handle control APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.send`.
///
/// # Replay
/// External, recordable.
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
///
/// Poll one process descriptor for state transition readiness and return immediately when no transition is pending.
/// Non-ready state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking poll over pidfd on Linux and zero-timeout process wait on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.wait`.
///
/// # Replay
/// External, recordable.
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
///
/// Wait for one child-state transition associated with the process descriptor.
/// Wait semantics follow pollable pidfd readiness on Linux and host process wait APIs on other targets.
///
/// # Platform
/// Unix and Windows.
/// Uses poll or waitid over pidfd on Linux and WaitForSingleObject plus status queries on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.wait`.
///
/// # Replay
/// External, recordable.
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
///
/// Close one descriptor-backed signal queue and release host resources.
/// Close semantics follow host descriptor teardown behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses close(2) on Unix and CloseHandle on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
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
///
/// Open one descriptor-backed signal queue that can be polled and read like other fd resources.
/// Signal mask semantics follow signalfd on Linux and host-equivalent runtime adapters on other targets.
///
/// # Platform
/// Unix and Windows.
/// Uses signalfd(2) on Linux and host-equivalent process-signal descriptor adapters elsewhere.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
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
///
/// Read the next queued signal payload from one descriptor-backed signal queue.
/// Queue ordering and coalescing behavior follow host signal queue semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses read(2) over signalfd on Linux and host-equivalent signal descriptor reads on other targets.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
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
///
/// Replace the descriptor signal mask with one explicit signal-set value.
/// Mask transitions are atomic under host signal-descriptor APIs.
///
/// # Platform
/// Unix and Windows.
/// Uses signalfd mask update semantics on Linux and host-equivalent signal descriptor mask updates elsewhere.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
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
///
/// Poll one descriptor-backed signal queue for one signal event and return immediately when empty.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking reads over signalfd on Linux and host-equivalent signal descriptor polling elsewhere.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
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
///
/// Open one handle for the current process standard error stream.
/// The returned handle can be used with file-handle write, sync, and close operations.
///
/// # Platform
/// Unix and Windows.
/// Uses dup(2) from descriptor 2 on Unix and DuplicateHandle from GetStdHandle(STD_ERROR_HANDLE) on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.stdio`.
///
/// # Replay
/// External, recordable.
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
///
/// Open one handle for the current process standard input stream.
/// The returned handle can be used with file-handle read and close operations.
///
/// # Platform
/// Unix and Windows.
/// Uses dup(2) from descriptor 0 on Unix and DuplicateHandle from GetStdHandle(STD_INPUT_HANDLE) on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.stdio`.
///
/// # Replay
/// External, recordable.
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
///
/// Open one handle for the current process standard output stream.
/// The returned handle can be used with file-handle write, sync, and close operations.
///
/// # Platform
/// Unix and Windows.
/// Uses dup(2) from descriptor 1 on Unix and DuplicateHandle from GetStdHandle(STD_OUTPUT_HANDLE) on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.stdio`.
///
/// # Replay
/// External, recordable.
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
///
/// Read one controller limit value from one control-group path.
/// Resource selector interpretation follows host controller semantics.
///
/// # Platform
/// Linux.
/// Uses cgroup controller files in v2 hierarchies.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.cgroup`.
///
/// # Replay
/// External, nonrecordable.
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
///
/// Attach the current process to one control-group path.
/// Group hierarchy and controller behavior follow host kernel rules.
///
/// # Platform
/// Linux.
/// Uses cgroup v2 control files.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.cgroup`.
///
/// # Replay
/// External, nonrecordable.
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
///
/// Write one controller limit value to one control-group path.
/// Controller validation and privilege checks are host-enforced.
///
/// # Platform
/// Linux.
/// Uses cgroup controller files in v2 hierarchies.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.cgroup`.
///
/// # Replay
/// External, nonrecordable.
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
///
/// Attach one or more target processes to one named job object.
/// Job object lifetime and inheritance follow host process-manager semantics.
///
/// # Platform
/// Windows.
/// Uses job object assignment APIs.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.cgroup`.
///
/// # Replay
/// External, nonrecordable.
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
///
/// Write one resource limit entry to one named job object.
/// Resource selector interpretation follows host job object semantics.
///
/// # Platform
/// Windows.
/// Uses job object limit APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.cgroup`.
///
/// # Replay
/// External, nonrecordable.
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
///
/// Read the effective primary group for the calling process.
/// Effective group can differ from the real group under setgid-style execution.
///
/// # Platform
/// Unix and Windows.
/// Uses getegid(2) on Unix and token group translation on Windows.
///
/// # Errors
/// Returns ioInvalidData, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_egid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<GroupId> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.egid")).boxed())
}

/// Return the effective user identifier.
///
/// Read the effective user identity for the calling process.
/// Effective identity can differ from the real identity under setuid-style execution.
///
/// # Platform
/// Unix and Windows.
/// Uses geteuid(2) on Unix and token SID translation on Windows.
///
/// # Errors
/// Returns ioInvalidData, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_euid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<UserId> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.euid")).boxed())
}

/// Return the current group identifier.
///
/// Read the real primary group for the calling process.
/// Group mapping on Windows is best-effort and may require token translation.
///
/// # Platform
/// Unix and Windows.
/// Uses getgid(2) on Unix and token group translation on Windows.
///
/// # Errors
/// Returns ioInvalidData, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_gid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<GroupId> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.gid")).boxed())
}

/// Return real, effective, and saved-set group identifiers.
///
/// Read all group-id classes for the calling process in one operation.
/// Saved-id availability follows host kernel semantics and can be unsupported on some targets.
///
/// # Platform
/// Unix and Windows.
/// Uses getresgid(2) on Unix and token mapping or `notSupported` on Windows.
///
/// # Errors
/// Returns ioInvalidData, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_group_ids(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<ProcessGroupIdsVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.groupIds")).boxed())
}

/// Return supplementary group identifiers.
///
/// Read the supplementary group list attached to the calling process.
/// Group ordering follows host kernel reporting order.
///
/// # Platform
/// Unix and Windows.
/// Uses getgroups(2) on Unix and token group enumeration on Windows.
///
/// # Errors
/// Returns ioInvalidData, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_groups(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<GroupId>> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.groups")).boxed())
}

/// Return the current process identifier.
///
/// Read the caller process id from the host process table.
/// The value is stable for the lifetime of the process.
///
/// # Platform
/// Unix and Windows.
/// Uses getpid(2) on Unix and GetCurrentProcessId on Windows.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `process.identity.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_pid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<ProcessId> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.pid")).boxed())
}

/// Return the parent process identifier.
///
/// Read the parent id relation as reported by the host kernel.
/// Parent visibility on Windows can be restricted by host policy.
///
/// # Platform
/// Unix and Windows.
/// Uses getppid(2) on Unix and process snapshot APIs on Windows.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `process.identity.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_ppid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<ProcessId> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.ppid")).boxed())
}

/// Set the effective group identifier only.
///
/// Change only the effective primary group while preserving the real group.
/// Privilege checks and saved-ID rules are enforced by the host kernel.
///
/// # Platform
/// Unix and Windows.
/// Uses setegid(2) on Unix and token adjustment on Windows where supported.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.write`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_set_egid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    groupid: GroupId,
) -> RuntimeResult<()> {
    let _ = groupid;
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.setEgid")).boxed())
}

/// Set the effective user identifier only.
///
/// Change only the effective user identity while preserving the real identity.
/// Privilege checks and saved-ID rules are enforced by the host kernel.
///
/// # Platform
/// Unix and Windows.
/// Uses seteuid(2) on Unix and token adjustment on Windows where supported.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.write`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_set_euid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    userid: UserId,
) -> RuntimeResult<()> {
    let _ = userid;
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.setEuid")).boxed())
}

/// Set the effective group identifier.
///
/// Set the process primary group identity.
/// Real and effective group update behavior follows host setgid semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses setgid(2) on Unix and token adjustment on Windows where supported.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.write`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_set_gid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    groupid: GroupId,
) -> RuntimeResult<()> {
    let _ = groupid;
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.setGid")).boxed())
}

/// Set real, effective, and saved-set group identifiers together.
///
/// Update all group-id classes in one host operation.
/// Privilege checks and immutable-id restrictions follow host kernel rules.
///
/// # Platform
/// Unix and Windows.
/// Uses setresgid(2) on Unix and token adjustment or `notSupported` on Windows.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.write`.
///
/// # Replay
/// External, recordable.
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
///
/// Replace the calling process supplementary group list.
/// This operation is typically restricted to privileged processes.
///
/// # Platform
/// Unix and Windows.
/// Uses setgroups(2) on Unix and token group adjustment on Windows where supported.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.write`.
///
/// # Replay
/// External, recordable.
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
///
/// Set the process user identity.
/// Real and effective identity update behavior follows host setuid semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses setuid(2) on Unix and token adjustment on Windows where supported.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.write`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_set_uid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    userid: UserId,
) -> RuntimeResult<()> {
    let _ = userid;
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.setUid")).boxed())
}

/// Set real, effective, and saved-set user identifiers together.
///
/// Update all user-id classes in one host operation.
/// Privilege checks and immutable-id restrictions follow host kernel rules.
///
/// # Platform
/// Unix and Windows.
/// Uses setresuid(2) on Unix and token adjustment or `notSupported` on Windows.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.write`.
///
/// # Replay
/// External, recordable.
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
///
/// Read the real user identity for the calling process.
/// Identity mapping on Windows is best-effort and may require token translation.
///
/// # Platform
/// Unix and Windows.
/// Uses getuid(2) on Unix and token SID translation on Windows.
///
/// # Errors
/// Returns ioInvalidData, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_uid(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<UserId> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.uid")).boxed())
}

/// Return real, effective, and saved-set user identifiers.
///
/// Read all user-id classes for the calling process in one operation.
/// Saved-id availability follows host kernel semantics and can be unsupported on some targets.
///
/// # Platform
/// Unix and Windows.
/// Uses getresuid(2) on Unix and token mapping or `notSupported` on Windows.
///
/// # Errors
/// Returns ioInvalidData, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_user_ids(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<ProcessUserIdsVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.ids.userIds")).boxed())
}

/// Change the root directory for path resolution.
///
/// Replace process root path resolution context with the provided directory.
/// Root-change semantics are host-defined and privilege-gated.
///
/// # Platform
/// Unix.
/// Uses chroot(2) or equivalent jail primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `security.restrict`.
///
/// # Replay
/// External, nonrecordable.
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
///
/// Install one host syscall filter for the current process.
/// Program bytecode and verifier rules are host-specific.
///
/// # Platform
/// Linux.
/// Uses seccomp filter install primitives.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `security.filter`.
///
/// # Replay
/// External, nonrecordable.
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
///
/// Update host name for the current UTS namespace or host context.
/// Name-length and privilege rules are enforced by the host kernel.
///
/// # Platform
/// Unix and Windows.
/// Uses sethostname(2) on Unix and host name APIs on Windows where permitted.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.namespace`.
///
/// # Replay
/// External, nonrecordable.
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
///
/// Switch network operation context to the specified network namespace path.
/// Namespace transition rules and privileges are host-defined.
///
/// # Platform
/// Linux.
/// Uses setns-style namespace switching with network namespace descriptors.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.namespace`.
///
/// # Replay
/// External, nonrecordable.
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
///
/// Join one specific namespace type from the target process.
/// Namespace join rules and privilege checks are enforced by the host kernel.
///
/// # Platform
/// Linux.
/// Uses setns(2) with namespace file descriptors.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.namespace`.
///
/// # Replay
/// External, nonrecordable.
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
///
/// Create isolated namespaces for the current process according to flag bits.
/// Namespace semantics and inheritance follow host kernel rules.
///
/// # Platform
/// Linux.
/// Uses unshare(2).
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.namespace`.
///
/// # Replay
/// External, nonrecordable.
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
///
/// Read soft and hard limits for one host resource selector.
/// Resource selector interpretation follows host kernel limit tables.
///
/// # Platform
/// Unix and Windows.
/// Uses getrlimit or prlimit on Unix and job-object limit queries on Windows where available.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.run`.
///
/// # Replay
/// External, recordable.
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
///
/// Update soft and hard limits for one host resource selector.
/// Privilege checks and hard-limit rules are enforced by the host kernel.
///
/// # Platform
/// Unix and Windows.
/// Uses setrlimit or prlimit on Unix and job-object limit updates on Windows where available.
///
/// # Errors
/// Returns invalidArgument, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.run`.
///
/// # Replay
/// External, recordable.
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
///
/// Read the active CPU affinity mask for the target process identifier.
/// Returned CPUs reflect host scheduler topology visibility.
///
/// # Platform
/// Unix and Windows.
/// Uses sched_getaffinity(2) on Unix and GetProcessAffinityMask on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.affinity`.
///
/// # Replay
/// External, recordable.
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
///
/// Read the scheduler priority value for the target process identifier.
/// Priority ranges and classes are host-defined.
///
/// # Platform
/// Unix and Windows.
/// Uses getpriority(2) on Unix and GetPriorityClass plus thread priority mapping on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.priority`.
///
/// # Replay
/// External, recordable.
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
///
/// Read one process scheduler policy class and its priority details.
/// Returned policy availability and numeric ranges are host-defined.
///
/// # Platform
/// Unix and Windows.
/// Uses sched_getscheduler plus sched_getparam on Unix and process scheduling class mapping on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.scheduler`.
///
/// # Replay
/// External, recordable.
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
///
/// Set the CPU affinity mask for the target process identifier.
/// Invalid CPU sets and privilege violations are rejected by the host scheduler.
///
/// # Platform
/// Unix and Windows.
/// Uses sched_setaffinity(2) on Unix and SetProcessAffinityMask on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.affinity`.
///
/// # Replay
/// External, recordable.
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
///
/// Set the scheduler priority value for the target process identifier.
/// Privilege checks and clamping are enforced by the host scheduler.
///
/// # Platform
/// Unix and Windows.
/// Uses setpriority(2) on Unix and SetPriorityClass or SetThreadPriority on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.priority`.
///
/// # Replay
/// External, recordable.
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
///
/// Set one process scheduler policy class with explicit priority and flags.
/// Privilege checks and policy-specific clamping are enforced by the host scheduler.
///
/// # Platform
/// Unix and Windows.
/// Uses sched_setscheduler plus sched_setparam on Unix and process scheduling class mapping on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.scheduler`.
///
/// # Replay
/// External, recordable.
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
///
/// Yield one scheduler timeslice voluntarily from the current execution context.
/// Yield ordering and wakeup behavior follow host scheduler semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses sched_yield(2) on Unix and SwitchToThread or Sleep(0) on Windows.
///
/// # Errors
/// Returns processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.scheduler`.
///
/// # Replay
/// External, recordable.
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
///
/// Read the process-group identifier currently assigned to the target process.
/// Visibility and lookup behavior follow host process table rules.
///
/// # Platform
/// Unix and Windows.
/// Uses getpgid(2) on Unix and runtime emulation or `notSupported` on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.session`.
///
/// # Replay
/// External, recordable.
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
///
/// Move the target process into the requested process group identifier.
/// Cross-session moves and permission checks follow host kernel job-control rules.
///
/// # Platform
/// Unix and Windows.
/// Uses setpgid(2) on Unix and runtime emulation or `notSupported` on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.session`.
///
/// # Replay
/// External, recordable.
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
///
/// Create a new session boundary and make the caller its session leader.
/// Repository and controlling-terminal semantics follow host job-control rules.
///
/// # Platform
/// Unix and Windows.
/// Uses setsid(2) on Unix and runtime emulation or `notSupported` on Windows.
///
/// # Errors
/// Returns processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.session`.
///
/// # Replay
/// External, recordable.
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
///
/// Deliver one signal value to the target process according to host signal semantics.
/// Delivery guarantees and supported signal numbers are host-defined.
///
/// # Platform
/// Unix and Windows.
/// Uses kill(2) on Unix and terminate or control-event APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.send`.
///
/// # Replay
/// External, recordable.
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
///
/// Return the active signal mask as an explicit signal set.
/// Mask semantics follow host thread-signal rules.
///
/// # Platform
/// Unix and Windows.
/// Uses sigprocmask or pthread_sigmask on Unix and host-equivalent APIs where available.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
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
///
/// Apply one set, block, or unblock operation to the active signal mask.
/// Mask transitions are atomic under host signal APIs.
///
/// # Platform
/// Unix and Windows.
/// Uses sigprocmask or pthread_sigmask on Unix and host-equivalent APIs where available.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
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
///
/// Wait for the next queued signal event for the subscription.
/// Delivery ordering and batching follow runtime and host signal queue semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses host signal delivery queues and wait primitives.
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
///
/// Register one runtime subscription handle for signal delivery.
/// Subscription mode and coalescing behavior follow runtime and host integration rules.
///
/// # Platform
/// Unix and Windows.
/// Uses host signal subscription state and queue integration.
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
///
/// Read a queued signal event when available and return immediately otherwise.
/// Empty queue behavior is reported through host-specific not-ready errors.
///
/// # Platform
/// Unix and Windows.
/// Uses host signal queue polling with nonblocking probes.
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
///
/// Check whether one requested signal is pending and return immediately.
/// Empty readiness is reported through host-specific not-ready errors.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking signal wait primitives and host-equivalent polling APIs where available.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
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
///
/// Unregister one signal subscription from runtime delivery.
/// Pending events may still be readable depending on host queueing behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses host signal subscription teardown semantics.
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
///
/// Block until one of the requested signals is observed and returned.
/// Selection and wakeup semantics follow host signal wait behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses sigwait-style primitives on Unix and host-equivalent wait APIs where available.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.signal.receive`.
///
/// # Replay
/// External, recordable.
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
///
/// Spawn one child process using the provided command, argv, envp, and spawn options.
/// Descriptor inheritance and process-group behavior follow host process-launch semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses posix_spawn or fork-plus-exec on Unix and CreateProcessW on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, processSpawnFailed, notSupported.
///
/// # Security
/// Requires `process.spawn`.
///
/// # Replay
/// External, recordable.
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
///
/// Spawn one child process and apply explicit stdio wiring and descriptor action scripts.
/// File-action ordering and inheritance behavior follow host spawn primitives.
///
/// # Platform
/// Unix and Windows.
/// Uses posix_spawn_file_actions on Unix and handle-inheritance setup on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, processSpawnFailed, notSupported.
///
/// # Security
/// Requires `process.spawn`.
///
/// # Replay
/// External, recordable.
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
///
/// Update the process-wide file mode creation mask used by future file-creation operations.
/// Mask interpretation follows POSIX mode bit rules.
///
/// # Platform
/// Unix and Windows.
/// Uses umask(2) on Unix and runtime compatibility behavior on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.identity.write`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_umask(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    mask: u32,
) -> RuntimeResult<u32> {
    let _ = mask;
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.umask.set")).boxed())
}

/// Wait for a process identifier.
///
/// Wait for one state transition for the specified process identifier.
/// Identifier matching and visibility follow host process table and job-control rules.
///
/// # Platform
/// Unix and Windows.
/// Uses waitpid or waitid on Unix and process-handle wait translation on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.wait`.
///
/// # Replay
/// External, recordable.
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
///
/// Poll one child for a state transition and return immediately when no transition is pending.
/// Pending absence is reported through `ioWouldBlock` without sleeping.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking waitpid or waitid on Unix and zero-timeout wait on Windows.
///
/// # Errors
/// Returns processNotFound, processPermissionDenied, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.wait`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_try_wait(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::ProcessHandle,
) -> RuntimeResult<ProcessWaitStatusVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.wait.tryWait")).boxed())
}

/// Wait for a child process handle.
///
/// Wait for one child state transition and return a normalized wait status payload.
/// Blocking and state-filter behavior is controlled by wait flags and host wait semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses waitpid or waitid on Unix and WaitForSingleObject plus status queries on Windows.
///
/// # Errors
/// Returns processNotFound, processPermissionDenied, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `process.wait`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_process_wait(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::ProcessHandle,
    flags: ProcessWaitFlags,
) -> RuntimeResult<ProcessWaitStatusVm> {
    let _ = (handle, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.process.wait.handle")).boxed())
}
