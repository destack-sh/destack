#![cfg_attr(windows, allow(dead_code, unused_imports))]

#[path = "harness.rs"]
mod harness;

#[cfg(unix)]
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeStringRef, VmAbi};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::core as core_fs;
use crate::platform::resource::{self, ResourceId};
use crate::platform::{PlatformError, VmArray, VmSlice, fs, process as process_platform};
use crate::runtime::BindingCallContext;
pub(crate) use crate::tests::platform::{
    assert_platform_error_code_with_privileged_policy,
    assert_platform_error_codes_with_privileged_policy,
};
use crate::tests::runtime::TestRuntime;
use process_platform::{
    GroupId, ProcessFdActionVm, ProcessGroupIds, ProcessId, ProcessLimit, ProcessLimitResource,
    ProcessStdioVm, ProcessUserIds, Signal, UserId,
};

/// Spawn options used by process harness helpers.
#[derive(Debug, Clone)]
pub(crate) struct ProcessSpawnOptionsSpec {
    /// Child working directory.
    pub cwd: String,
    /// Detached process mode.
    pub detached: bool,
    /// Reset signal dispositions.
    pub reset_signals: bool,
    /// Start a new process group.
    pub new_process_group: bool,
}

impl ProcessSpawnOptionsSpec {
    /// Build default spawn options for one cwd.
    pub(crate) fn inherit(cwd: String) -> Self {
        Self {
            cwd,
            detached: false,
            reset_signals: false,
            new_process_group: false,
        }
    }
}

/// Stdio descriptor spec for spawnWithActions tests.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProcessStdioSpec {
    /// Stdio kind selector.
    pub kind: ProcessStdioKind,
    /// Descriptor payload for `Descriptor` kind.
    pub descriptor: i32,
    /// Runtime resource id payload for `File` and `Pipe` kinds.
    pub resource_id: Option<ResourceId>,
}

/// Fd action spec for spawnWithActions tests.
#[derive(Debug, Clone)]
pub(crate) struct ProcessFdActionSpec {
    /// Action opcode.
    pub op: ProcessFdActionKind,
    /// Source descriptor.
    pub source: i32,
    /// Target descriptor.
    pub target: i32,
    /// Path payload for `Open` actions.
    pub path: String,
    /// Open flags for `Open` actions.
    pub flags: fs::OpenFlags,
    /// Open mode for `Open` actions.
    pub mode: fs::FileMode,
}

/// Stdio kind selector used by process test helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessStdioKind {
    /// Use an inherited stdio stream.
    Inherit,
    /// Route stdio to the null device.
    Null,
    /// Route stdio through an existing file descriptor.
    Descriptor,
    /// Route stdio through a runtime file handle.
    File,
    /// Route stdio through a runtime pipe handle.
    Pipe,
}

/// Fd action selector used by process test helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessFdActionKind {
    /// Close one descriptor in the child.
    Close,
    /// Duplicate one descriptor in the child.
    Dup2,
    /// Open one path as a descriptor in the child.
    Open,
}

/// Wait status selector used by process test helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessWaitKind {
    /// Child continued after stop.
    Continued,
    /// Child exited normally.
    Exited,
    /// Child is still running.
    Running,
    /// Child exited due to signal.
    Signaled,
    /// Child stopped by signal.
    Stopped,
}

/// Construct all process helper variants that are not exercised by current spawn lanes.
#[cfg(any(unix, windows))]
#[test]
fn test_process_helper_variants_constructible() {
    // construct file and pipe stdio helpers
    let file_stdio = ProcessStdioSpec {
        kind: ProcessStdioKind::File,
        descriptor: 0,
        resource_id: Some(ResourceId(7)),
    };
    let pipe_stdio = ProcessStdioSpec {
        kind: ProcessStdioKind::Pipe,
        descriptor: 0,
        resource_id: Some(ResourceId(9)),
    };

    // construct close fd-action helper
    let close_action = ProcessFdActionSpec {
        op: ProcessFdActionKind::Close,
        source: 3,
        target: -1,
        path: String::new(),
        flags: fs::OpenFlags(0),
        mode: fs::FileMode(0),
    };

    assert!(matches!(file_stdio.kind, ProcessStdioKind::File));
    assert!(matches!(pipe_stdio.kind, ProcessStdioKind::Pipe));
    assert!(matches!(close_action.op, ProcessFdActionKind::Close));
}

/// Decoded wait-status payload used by process test helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProcessWaitStatusRecord {
    /// Process identifier associated with this wait status.
    pub pid: ProcessId,
    /// Wait result kind.
    pub kind: ProcessWaitKind,
    /// Exit status when `kind` is exited.
    pub exit_code: Option<i32>,
    /// Signal value when `kind` is signaled or stopped.
    pub signal: Option<Signal>,
    /// Core-dump flag for signaled exits.
    pub core_dumped: bool,
}

/// Monotonic suffix used to avoid cross-test collisions.
static NEXT_ENV_SUFFIX: AtomicU64 = AtomicU64::new(1);
/// Global lock that serializes process-state mutations across tests.
static PROCESS_TEST_LOCK: Mutex<()> = Mutex::new(());

/// Build a unique environment variable name for a test case.
pub(crate) fn unique_env_name(prefix: &str) -> String {
    let pid = std::process::id();
    let suffix = NEXT_ENV_SUFFIX.fetch_add(1, Ordering::Relaxed);
    format!("DESTACK_PROCESS_TEST_{prefix}_{pid}_{suffix}")
}

/// Build a unique temporary file path for process tests.
#[cfg(unix)]
pub(crate) fn unique_temp_file_path(prefix: &str) -> PathBuf {
    let name = unique_env_name(prefix);
    std::env::temp_dir().join(name)
}

/// Decode a native string reference into an owned string.
fn native_string(value: NativeStringRef) -> RuntimeResult<String> {
    let value = unsafe { value.as_str()? };
    Ok(value.to_string())
}

/// Decode a VM string handle into an owned string.
fn vm_string(
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<String> {
    let value_ref = context
        .string_ref(value)
        .map_err(|error| RuntimeError::from(error).boxed())?;
    Ok(value_ref.as_str().to_string())
}

/// Decode a native `OsPath` value into UTF-8.
fn native_path_to_utf8(path: fs::OsPath) -> RuntimeResult<String> {
    match path {
        fs::OsPath::OsPathBytes(path) => {
            let bytes = unsafe { path.bytes.0.as_slice()? };
            String::from_utf8(bytes.to_vec()).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "path",
                    "path bytes are not valid utf8",
                ))
                .boxed()
            })
        }
        fs::OsPath::OsPathUtf16(path) => {
            let utf16 = unsafe { path.utf16.0.as_slice()? };
            String::from_utf16(utf16).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "path",
                    "path utf16 is not valid",
                ))
                .boxed()
            })
        }
    }
}

/// Decode a VM `OsPath` value into UTF-8.
fn vm_path_to_utf8(
    context: &mut vm::ExternalCallContext<'_>,
    path: fs::OsPathVm,
) -> RuntimeResult<String> {
    match path {
        fs::OsPathVm::OsPathBytes(path) => {
            let bytes = path.bytes.0.read_bytes(&context.read())?;
            String::from_utf8(bytes).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "path",
                    "path bytes are not valid utf8",
                ))
                .boxed()
            })
        }
        fs::OsPathVm::OsPathUtf16(path) => {
            let utf16 = path.utf16.0.read_values(&context.read())?;
            String::from_utf16(&utf16).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "path",
                    "path utf16 is not valid",
                ))
                .boxed()
            })
        }
    }
}

/// Encode a UTF-8 path string into native `OsPath`.
fn native_path_from_utf8(binding: &BindingCallContext, value: &str) -> fs::OsPath {
    core_fs::os_path_from_utf8_string(binding, value.to_string())
}

/// Encode a UTF-8 path string into VM `OsPath`.
fn vm_path_from_utf8(
    context: &mut vm::ExternalCallContext<'_>,
    value: &str,
) -> RuntimeResult<fs::OsPathVm> {
    #[cfg(unix)]
    {
        let bytes = fs::PathBytesAbi::<VmAbi>(
            VmArray::from_bytes(&mut context.write(), value.as_bytes())
                .expect("vm test byte array should allocate"),
        );
        let kind = vm::StringHandle::new(
            context
                .intern_string("bytes")
                .expect("vm test string should intern"),
        );
        Ok(fs::OsPathVm::OsPathBytes(fs::OsPathBytesVm { kind, bytes }))
    }

    #[cfg(windows)]
    {
        let utf16_values = value.encode_utf16().collect::<Vec<_>>();
        let utf16 =
            fs::PathUtf16Abi::<VmAbi>(VmArray::from_values(&mut context.write(), &utf16_values)?);
        let kind = vm::StringHandle::new(
            context
                .intern_string("utf16")
                .expect("vm test string should intern"),
        );
        Ok(fs::OsPathVm::OsPathUtf16(fs::OsPathUtf16Vm { kind, utf16 }))
    }

    #[cfg(not(any(unix, windows)))]
    {
        let bytes = fs::PathBytesAbi::<VmAbi>(
            VmArray::from_bytes(&mut context.write(), value.as_bytes())
                .expect("vm test byte array should allocate"),
        );
        let kind = vm::StringHandle::new(
            context
                .intern_string("bytes")
                .expect("vm test string should intern"),
        );
        Ok(fs::OsPathVm::OsPathBytes(fs::OsPathBytesVm { kind, bytes }))
    }
}

/// Build one VM string slice from owned string values.
fn vm_string_slice(
    context: &mut vm::ExternalCallContext<'_>,
    values: &[String],
) -> RuntimeResult<VmSlice<vm::StringHandle>> {
    let handles = values
        .iter()
        .map(|value| {
            vm::StringHandle::new(
                context
                    .intern_string(value)
                    .expect("vm test string should intern"),
            )
        })
        .collect::<Vec<_>>();
    VmSlice::from_values(&mut context.write(), &handles)
}

/// Build one VM stdio slice from test specs.
fn vm_process_stdio_slice(
    context: &mut vm::ExternalCallContext<'_>,
    values: &[ProcessStdioSpec],
) -> RuntimeResult<VmSlice<ProcessStdioVm>> {
    if values.is_empty() {
        return Ok(VmSlice {
            data: vm::Value::VOID,
            len: 0,
            _marker: std::marker::PhantomData,
        });
    }

    let mut encoded = Vec::with_capacity(values.len());
    for value in values {
        let encoded_value = match value.kind {
            ProcessStdioKind::Descriptor => {
                ProcessStdioVm::ProcessStdioDescriptor(process_platform::ProcessStdioDescriptorVm {
                    kind: vm::StringHandle::new(
                        context
                            .intern_string("descriptor")
                            .expect("vm test string should intern"),
                    ),
                    descriptor: value.descriptor,
                })
            }
            ProcessStdioKind::File => {
                let resource_id = value
                    .resource_id
                    .expect("file stdio should provide one resource id");
                ProcessStdioVm::ProcessStdioFile(process_platform::ProcessStdioFileVm {
                    kind: vm::StringHandle::new(
                        context
                            .intern_string("file")
                            .expect("vm test string should intern"),
                    ),
                    file: resource::FileHandle(resource_id),
                })
            }
            ProcessStdioKind::Inherit => {
                ProcessStdioVm::ProcessStdioInherit(process_platform::ProcessStdioInheritVm {
                    kind: vm::StringHandle::new(
                        context
                            .intern_string("inherit")
                            .expect("vm test string should intern"),
                    ),
                })
            }
            ProcessStdioKind::Null => {
                ProcessStdioVm::ProcessStdioNull(process_platform::ProcessStdioNullVm {
                    kind: vm::StringHandle::new(
                        context
                            .intern_string("null")
                            .expect("vm test string should intern"),
                    ),
                })
            }
            ProcessStdioKind::Pipe => {
                let resource_id = value
                    .resource_id
                    .expect("pipe stdio should provide one resource id");
                ProcessStdioVm::ProcessStdioPipe(process_platform::ProcessStdioPipeVm {
                    kind: vm::StringHandle::new(
                        context
                            .intern_string("pipe")
                            .expect("vm test string should intern"),
                    ),
                    pipe: resource::PipeHandle(resource_id),
                })
            }
        };
        encoded.push(encoded_value);
    }

    VmSlice::from_values(&mut context.write(), &encoded)
}

/// Build one VM fd-action slice from test specs.
fn vm_process_fd_action_slice(
    context: &mut vm::ExternalCallContext<'_>,
    values: &[ProcessFdActionSpec],
) -> RuntimeResult<VmSlice<ProcessFdActionVm>> {
    if values.is_empty() {
        return Ok(VmSlice {
            data: vm::Value::VOID,
            len: 0,
            _marker: std::marker::PhantomData,
        });
    }

    let mut encoded = Vec::with_capacity(values.len());
    for value in values {
        let encoded_value = match value.op {
            ProcessFdActionKind::Close => {
                ProcessFdActionVm::ProcessFdActionClose(process_platform::ProcessFdActionCloseVm {
                    kind: vm::StringHandle::new(
                        context
                            .intern_string("close")
                            .expect("vm test string should intern"),
                    ),
                    descriptor: value.source,
                })
            }
            ProcessFdActionKind::Dup2 => {
                ProcessFdActionVm::ProcessFdActionDup2(process_platform::ProcessFdActionDup2Vm {
                    kind: vm::StringHandle::new(
                        context
                            .intern_string("dup2")
                            .expect("vm test string should intern"),
                    ),
                    source: value.source,
                    target: value.target,
                })
            }
            ProcessFdActionKind::Open => {
                let path = vm_path_from_utf8(context, &value.path)?;
                ProcessFdActionVm::ProcessFdActionOpen(process_platform::ProcessFdActionOpenVm {
                    kind: vm::StringHandle::new(
                        context
                            .intern_string("open")
                            .expect("vm test string should intern"),
                    ),
                    target: value.target,
                    path,
                    flags: value.flags,
                    mode: value.mode,
                })
            }
        };
        encoded.push(encoded_value);
    }

    VmSlice::from_values(&mut context.write(), &encoded)
}

/// Return true when the runtime error is `ioWouldBlock`.
#[cfg(unix)]
pub(crate) fn is_would_block(error: &RuntimeError) -> bool {
    let Some(platform_error) = error.platform_error() else {
        return false;
    };

    platform_error.code == PlatformErrorCode::IoWouldBlock
}

/// Build an io runtime error from the current unix errno.
#[cfg(unix)]
fn unix_io_error(message: &str) -> Box<RuntimeError> {
    let error = std::io::Error::last_os_error();
    RuntimeError::from(PlatformError::io(format!("{message}: {error}"))).boxed()
}

/// Read group ids through direct unix syscalls.
#[cfg(unix)]
pub(crate) fn syscall_group_ids() -> RuntimeResult<ProcessGroupIds> {
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        let mut real: libc::gid_t = 0;
        let mut effective: libc::gid_t = 0;
        let mut saved: libc::gid_t = 0;
        let result = unsafe { libc::getresgid(&mut real, &mut effective, &mut saved) };
        if result != 0 {
            return Err(unix_io_error("failed to read group ids"));
        }

        Ok(ProcessGroupIds {
            real: GroupId(real as u32),
            effective: GroupId(effective as u32),
            saved: GroupId(saved as u32),
        })
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        let real = unsafe { libc::getgid() as u32 };
        let effective = unsafe { libc::getegid() as u32 };

        Ok(ProcessGroupIds {
            real: GroupId(real),
            effective: GroupId(effective),
            saved: GroupId(effective),
        })
    }
}

/// Read supplementary groups through direct unix syscalls.
#[cfg(unix)]
pub(crate) fn syscall_groups() -> RuntimeResult<Vec<GroupId>> {
    let count = unsafe { libc::getgroups(0, std::ptr::null_mut()) };
    if count < 0 {
        return Err(unix_io_error("failed to query supplementary group count"));
    }

    if count == 0 {
        return Ok(Vec::new());
    }

    let mut groups = vec![0 as libc::gid_t; count as usize];
    let result = unsafe { libc::getgroups(count, groups.as_mut_ptr()) };
    if result < 0 {
        return Err(unix_io_error("failed to read supplementary groups"));
    }

    Ok(groups.into_iter().map(GroupId).collect::<Vec<_>>())
}

/// Read user ids through direct unix syscalls.
#[cfg(unix)]
pub(crate) fn syscall_user_ids() -> RuntimeResult<ProcessUserIds> {
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        let mut real: libc::uid_t = 0;
        let mut effective: libc::uid_t = 0;
        let mut saved: libc::uid_t = 0;
        let result = unsafe { libc::getresuid(&mut real, &mut effective, &mut saved) };
        if result != 0 {
            return Err(unix_io_error("failed to read user ids"));
        }

        Ok(ProcessUserIds {
            real: UserId(real as u32),
            effective: UserId(effective as u32),
            saved: UserId(saved as u32),
        })
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        let real = unsafe { libc::getuid() as u32 };
        let effective = unsafe { libc::geteuid() as u32 };

        Ok(ProcessUserIds {
            real: UserId(real),
            effective: UserId(effective),
            saved: UserId(effective),
        })
    }
}

/// Read process group id through direct unix syscalls.
#[cfg(unix)]
pub(crate) fn syscall_getpgid(pid: ProcessId) -> RuntimeResult<ProcessId> {
    let result = unsafe { libc::getpgid(pid.0 as libc::pid_t) };
    if result < 0 {
        return Err(unix_io_error("failed to read process group id"));
    }

    Ok(ProcessId(result as u32))
}

/// Read process priority through direct unix syscalls.
#[cfg(unix)]
pub(crate) fn syscall_get_priority(pid: ProcessId) -> RuntimeResult<i32> {
    #[cfg(target_os = "linux")]
    unsafe fn errno_location() -> *mut libc::c_int {
        unsafe { libc::__errno_location() }
    }

    #[cfg(target_os = "android")]
    unsafe fn errno_location() -> *mut libc::c_int {
        unsafe { libc::__errno() }
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    unsafe fn errno_location() -> *mut libc::c_int {
        unsafe { libc::__error() }
    }

    unsafe {
        *errno_location() = 0;
    }

    let value = unsafe { libc::getpriority(libc::PRIO_PROCESS, pid.0 as libc::id_t) };
    if value == -1 {
        let errno = unsafe { *errno_location() };
        if errno != 0 {
            return Err(unix_io_error("failed to read priority"));
        }
    }

    Ok(value)
}

/// Read one process limit through direct unix syscalls.
#[cfg(unix)]
pub(crate) fn syscall_get_limit(resource: ProcessLimitResource) -> RuntimeResult<ProcessLimit> {
    let mut raw_limit = unsafe { std::mem::zeroed::<libc::rlimit>() };
    let result = unsafe { libc::getrlimit(resource.0 as _, &mut raw_limit) };
    if result != 0 {
        return Err(unix_io_error("failed to read process limit"));
    }

    Ok(ProcessLimit {
        soft: raw_limit.rlim_cur,
        hard: raw_limit.rlim_max,
    })
}

/// Fork a child that exits immediately with the provided status code.
#[cfg(unix)]
pub(crate) fn fork_child_exit(code: i32) -> RuntimeResult<ProcessId> {
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        return Err(unix_io_error("failed to fork child process"));
    }

    if pid == 0 {
        unsafe {
            libc::_exit(code);
        }
    }

    Ok(ProcessId(pid as u32))
}

/// Fork a child that sleeps and then exits with the provided status code.
#[cfg(unix)]
pub(crate) fn fork_child_sleep_then_exit(seconds: u32, code: i32) -> RuntimeResult<ProcessId> {
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        return Err(unix_io_error("failed to fork child process"));
    }

    if pid == 0 {
        unsafe {
            libc::sleep(seconds);
            libc::_exit(code);
        }
    }

    Ok(ProcessId(pid as u32))
}

/// Build a shell command that exits with the provided status code.
#[cfg(any(unix, windows))]
pub(crate) fn shell_exit_command(code: i32) -> (String, Vec<String>) {
    #[cfg(unix)]
    {
        (
            "/bin/sh".to_string(),
            vec!["-c".to_string(), format!("exit {code}")],
        )
    }

    #[cfg(windows)]
    {
        (
            "cmd".to_string(),
            vec!["/C".to_string(), format!("exit /B {code}")],
        )
    }
}

/// Build a shell command that sleeps briefly and exits with the provided status code.
#[cfg(any(unix, windows))]
pub(crate) fn shell_sleep_then_exit_command(seconds: u32, code: i32) -> (String, Vec<String>) {
    #[cfg(unix)]
    {
        (
            "/bin/sh".to_string(),
            vec!["-c".to_string(), format!("sleep {seconds}; exit {code}")],
        )
    }

    #[cfg(windows)]
    {
        (
            "cmd".to_string(),
            vec![
                "/C".to_string(),
                format!("ping -n {} 127.0.0.1 >NUL & exit /B {code}", seconds + 1),
            ],
        )
    }
}

/// Spawn one shell command and return its process id.
#[cfg(any(unix, windows))]
pub(crate) fn spawn_shell(command: String, arguments: Vec<String>) -> RuntimeResult<ProcessId> {
    let mut child = Command::new(command);
    child.args(arguments);
    let child = child.spawn().map_err(|error| {
        RuntimeError::from(PlatformError::io(format!(
            "failed to spawn shell process: {error}",
        )))
        .boxed()
    })?;

    let pid = ProcessId(child.id());
    std::mem::drop(child);
    Ok(pid)
}

/// Process harness context used by tests.
pub(crate) struct ProcessHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(super) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(super) vm_context: Option<*mut ()>,
}

impl<'call> ProcessHarnessContext<'call> {
    /// Return the runtime call context for native harness operations.
    #[cfg(windows)]
    pub(crate) fn call_context(&self) -> &'call BindingCallContext {
        self.call_context
    }
}

/// Native process harness backed by native bindings.
pub(crate) struct NativeProcessHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeProcessHarness {
    /// Create a new native process harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM process harness backed by VM bindings.
pub(crate) struct VmProcessHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmProcessHarness {
    /// Create a new VM process harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum ProcessHarnessHandle {
    /// Native process harness.
    Native(NativeProcessHarness),
    /// VM process harness.
    Vm(VmProcessHarness),
}

impl ProcessHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&mut self, callback: F) -> R
    where
        F: for<'call> FnOnce(ProcessHarnessContext<'call>) -> R,
    {
        match self {
            ProcessHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(ProcessHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            ProcessHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(ProcessHarnessContext {
                            call_context,
                            vm_context: Some(vm_context),
                        })
                    })
            }
        }
    }

    /// Run one callback that returns a runtime result.
    pub(crate) fn run<F>(&mut self, callback: F)
    where
        F: for<'call> FnOnce(ProcessHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("process harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&mut ProcessHarnessHandle),
{
    let mut native = ProcessHarnessHandle::Native(NativeProcessHarness::new());
    callback(&mut native);
    let mut vm = ProcessHarnessHandle::Vm(VmProcessHarness::new());
    callback(&mut vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(ProcessHarnessContext<'call>) -> RuntimeResult<()>,
{
    let _guard = PROCESS_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
