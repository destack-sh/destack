#![cfg_attr(windows, allow(dead_code, unused_imports))]

#[cfg(unix)]
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::process::{
    ExecAtFlags, GroupId, ProcessFdAction, ProcessFdActionKind, ProcessFdActionVm, ProcessFdFlags,
    ProcessGroupIds, ProcessId, ProcessLimit, ProcessLimitResource, ProcessSpawnOptions,
    ProcessSpawnOptionsVm, ProcessStdio, ProcessStdioKind, ProcessStdioVm, ProcessUserIds,
    ProcessWaitFlags, ProcessWaitKind, ProcessWaitStatus, Signal, SignalEvent, SignalFdFlags,
    SignalMaskHow, UserId, native as process_native, vm as process_vm,
};
use crate::platform::resource::{self, ResourceId};
use crate::platform::{
    NativeArray, NativeSlice, NativeStringRef, NativeStringSlice, VmArray, VmSlice, VmValueCodec,
    fs,
};
use crate::runtime::RuntimeCallContext;
use crate::tests::runtime::TestRuntime;

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

/// Monotonic suffix used to avoid cross-test collisions.
static NEXT_ENV_SUFFIX: AtomicU64 = AtomicU64::new(1);

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
    match path.encoding {
        fs::PathEncoding::Bytes => {
            let bytes = unsafe { path.bytes.0.as_slice()? };
            String::from_utf8(bytes.to_vec()).map_err(|_| {
                RuntimeError::from(crate::platform::PlatformError::invalid_argument_value(
                    "path",
                    "path bytes are not valid utf8",
                ))
                .boxed()
            })
        }
        fs::PathEncoding::Utf16 => {
            let utf16 = unsafe { path.utf16.0.as_slice()? };
            String::from_utf16(utf16).map_err(|_| {
                RuntimeError::from(crate::platform::PlatformError::invalid_argument_value(
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
    match path.encoding {
        fs::PathEncoding::Bytes => {
            let bytes = path.bytes.0.read_bytes(context)?;
            String::from_utf8(bytes).map_err(|_| {
                RuntimeError::from(crate::platform::PlatformError::invalid_argument_value(
                    "path",
                    "path bytes are not valid utf8",
                ))
                .boxed()
            })
        }
        fs::PathEncoding::Utf16 => {
            let utf16 = path.utf16.0.read_values(context)?;
            String::from_utf16(&utf16).map_err(|_| {
                RuntimeError::from(crate::platform::PlatformError::invalid_argument_value(
                    "path",
                    "path utf16 is not valid",
                ))
                .boxed()
            })
        }
    }
}

/// Encode a UTF-8 path string into native `OsPath`.
fn native_path_from_utf8(context: &RuntimeCallContext, value: &str) -> fs::OsPath {
    crate::platform::fs::core::os_path_from_utf8_string(context, value.to_string())
}

/// Encode a UTF-8 path string into VM `OsPath`.
fn vm_path_from_utf8(
    context: &mut vm::ExternalCallContext<'_>,
    value: &str,
) -> RuntimeResult<fs::OsPathVm> {
    #[cfg(unix)]
    {
        let bytes = fs::PathBytesAbi::<crate::platform::abi::VmAbi>(VmArray::from_bytes(
            context,
            value.as_bytes(),
        ));
        let utf16 = fs::PathUtf16Abi::<crate::platform::abi::VmAbi>(VmArray {
            data: vm::RawPointer::NULL,
            len: 0,
            capacity: 0,
            _marker: std::marker::PhantomData,
        });
        return Ok(fs::OsPathVm {
            encoding: fs::PathEncoding::Bytes,
            bytes,
            utf16,
        });
    }

    #[cfg(windows)]
    {
        let utf16_values = value.encode_utf16().collect::<Vec<_>>();
        let bytes = fs::PathBytesAbi::<crate::platform::abi::VmAbi>(VmArray {
            data: vm::RawPointer::NULL,
            len: 0,
            capacity: 0,
            _marker: std::marker::PhantomData,
        });
        let utf16 = fs::PathUtf16Abi::<crate::platform::abi::VmAbi>(VmArray::from_values(
            context,
            &utf16_values,
        )?);
        return Ok(fs::OsPathVm {
            encoding: fs::PathEncoding::Utf16,
            bytes,
            utf16,
        });
    }

    #[cfg(not(any(unix, windows)))]
    {
        let bytes = fs::PathBytesAbi::<crate::platform::abi::VmAbi>(VmArray::from_bytes(
            context,
            value.as_bytes(),
        ));
        let utf16 = fs::PathUtf16Abi::<crate::platform::abi::VmAbi>(VmArray {
            data: vm::RawPointer::NULL,
            len: 0,
            capacity: 0,
            _marker: std::marker::PhantomData,
        });
        Ok(fs::OsPathVm {
            encoding: fs::PathEncoding::Bytes,
            bytes,
            utf16,
        })
    }
}

/// Encode one VM `OsPath` value as an aggregate VM value.
fn vm_path_to_value(context: &mut vm::ExternalCallContext<'_>, path: fs::OsPathVm) -> vm::Value {
    let field_0 = vm::Value::uint(path.encoding as u8 as u64, 8);
    let field_1 = path.bytes.0.to_value(context);
    let field_2 = path.utf16.0.to_value(context);
    context.allocate_aggregate(vec![field_0, field_1, field_2])
}

/// Build one VM string slice from owned string values.
fn vm_string_slice(
    context: &mut vm::ExternalCallContext<'_>,
    values: &[String],
) -> RuntimeResult<VmSlice<vm::StringHandle>> {
    let handles = values
        .iter()
        .map(|value| vm::StringHandle::new(context.intern_string(value)))
        .collect::<Vec<_>>();
    VmSlice::from_values(context, &handles)
}

/// Build one VM stdio slice from test specs.
fn vm_process_stdio_slice(
    context: &mut vm::ExternalCallContext<'_>,
    values: &[ProcessStdioSpec],
) -> VmSlice<ProcessStdioVm> {
    if values.is_empty() {
        return VmSlice {
            data: vm::RawPointer::NULL,
            len: 0,
            _marker: std::marker::PhantomData,
        };
    }

    let mut encoded = Vec::with_capacity(values.len());
    for value in values {
        let file = resource::FileHandle(ResourceId(0));
        let pipe = resource::PipeHandle(ResourceId(0));
        let aggregate = context.allocate_aggregate(vec![
            value.kind.encode(),
            file.encode(),
            pipe.encode(),
            value.descriptor.encode(),
        ]);
        encoded.push(aggregate);
    }

    VmSlice {
        data: context.allocate_raw_values(encoded),
        len: values.len() as u32,
        _marker: std::marker::PhantomData,
    }
}

/// Build one VM fd-action slice from test specs.
fn vm_process_fd_action_slice(
    context: &mut vm::ExternalCallContext<'_>,
    values: &[ProcessFdActionSpec],
) -> RuntimeResult<VmSlice<ProcessFdActionVm>> {
    if values.is_empty() {
        return Ok(VmSlice {
            data: vm::RawPointer::NULL,
            len: 0,
            _marker: std::marker::PhantomData,
        });
    }

    let mut encoded = Vec::with_capacity(values.len());
    for value in values {
        let path = vm_path_from_utf8(context, &value.path)?;
        let path_value = vm_path_to_value(context, path);
        let aggregate = context.allocate_aggregate(vec![
            value.op.encode(),
            value.source.encode(),
            value.target.encode(),
            path_value,
            value.flags.encode(),
            value.mode.encode(),
        ]);
        encoded.push(aggregate);
    }

    Ok(VmSlice {
        data: context.allocate_raw_values(encoded),
        len: values.len() as u32,
        _marker: std::marker::PhantomData,
    })
}

/// Convert `notSupported` into `None` for portable assertions.
pub(crate) fn allow_not_supported<T>(result: RuntimeResult<T>) -> RuntimeResult<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) => {
            if let Some(platform) = error.platform_error() {
                if platform.code == PlatformErrorCode::NotSupported {
                    return Ok(None);
                }
            }

            Err(error)
        }
    }
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
    RuntimeError::from(crate::platform::PlatformError::io(format!(
        "{message}: {error}"
    )))
    .boxed()
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

        return Ok(ProcessGroupIds {
            real: GroupId(real as u32),
            effective: GroupId(effective as u32),
            saved: GroupId(saved as u32),
        });
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

    Ok(groups
        .into_iter()
        .map(|group| GroupId(group as u32))
        .collect::<Vec<_>>())
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

        return Ok(ProcessUserIds {
            real: UserId(real as u32),
            effective: UserId(effective as u32),
            saved: UserId(saved as u32),
        });
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
    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe fn errno_location() -> *mut libc::c_int {
        unsafe { libc::__errno_location() }
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
        soft: raw_limit.rlim_cur as u64,
        hard: raw_limit.rlim_max as u64,
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
        return (
            "/bin/sh".to_string(),
            vec!["-c".to_string(), format!("exit {code}")],
        );
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
        return (
            "/bin/sh".to_string(),
            vec!["-c".to_string(), format!("sleep {seconds}; exit {code}")],
        );
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
        RuntimeError::from(crate::platform::PlatformError::io(format!(
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
    call_context: &'call RuntimeCallContext,
    /// VM context when running VM bindings.
    vm_context: Option<*mut ()>,
}

impl<'call> ProcessHarnessContext<'call> {
    /// Return the VM context if available.
    #[allow(clippy::mut_from_ref)]
    fn vm_context_mut(&self) -> Option<&mut vm::ExternalCallContext<'_>> {
        self.vm_context
            .map(|context| unsafe { &mut *(context as *mut vm::ExternalCallContext<'_>) })
    }

    /// Read the process argument vector.
    pub(crate) fn args(&mut self) -> RuntimeResult<Vec<String>> {
        match self.vm_context_mut() {
            Some(context) => {
                let values = process_vm::destack_process_args(self.call_context, context)?;
                let handles = values.read_values(context)?;
                let mut decoded = Vec::with_capacity(handles.len());
                for handle in handles {
                    decoded.push(vm_string(context, handle)?);
                }
                Ok(decoded)
            }
            None => {
                let mut out = NativeStringSlice {
                    data: std::ptr::null(),
                    len: 0,
                };
                unsafe {
                    process_native::destack_process_args(self.call_context, &mut out)?;
                }
                let args = unsafe { out.as_slice()? };
                let mut decoded = Vec::with_capacity(args.len());
                for value in args {
                    decoded.push(native_string(*value)?);
                }
                Ok(decoded)
            }
        }
    }

    /// Read the current working directory.
    pub(crate) fn cwd(&mut self) -> RuntimeResult<String> {
        match self.vm_context_mut() {
            Some(context) => {
                let path = process_vm::destack_process_cwd(self.call_context, context)?;
                vm_path_to_utf8(context, path)
            }
            None => {
                let mut path = fs::OsPath {
                    encoding: fs::PathEncoding::Bytes,
                    bytes: fs::PathBytesAbi(NativeArray {
                        data: std::ptr::null_mut(),
                        len: 0,
                        capacity: 0,
                    }),
                    utf16: fs::PathUtf16Abi(NativeArray {
                        data: std::ptr::null_mut(),
                        len: 0,
                        capacity: 0,
                    }),
                };
                unsafe {
                    process_native::destack_process_cwd(self.call_context, &mut path)?;
                }
                native_path_to_utf8(path)
            }
        }
    }

    /// Set one UTF-8 environment variable.
    pub(crate) fn env_set(&mut self, name: &str, value: &str) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                let name_handle = vm::StringHandle::new(context.intern_string(name));
                let value_handle = vm::StringHandle::new(context.intern_string(value));
                process_vm::destack_process_env_set(
                    self.call_context,
                    context,
                    name_handle,
                    value_handle,
                )
            }
            None => {
                let name_ref = self.call_context.store_string(name);
                let value_ref = self.call_context.store_string(value);
                unsafe {
                    process_native::destack_process_env_set(self.call_context, name_ref, value_ref)
                }
            }
        }
    }

    /// Read one UTF-8 environment variable.
    pub(crate) fn env_get(&mut self, name: &str) -> RuntimeResult<String> {
        match self.vm_context_mut() {
            Some(context) => {
                let name_handle = vm::StringHandle::new(context.intern_string(name));
                let value =
                    process_vm::destack_process_env_get(self.call_context, context, name_handle)?;
                vm_string(context, value)
            }
            None => {
                let name_ref = self.call_context.store_string(name);
                let mut out = NativeStringRef {
                    data: std::ptr::null(),
                    len: 0,
                };
                unsafe {
                    process_native::destack_process_env_get(self.call_context, &mut out, name_ref)?;
                }
                native_string(out)
            }
        }
    }

    /// Delete one UTF-8 environment variable.
    pub(crate) fn env_delete(&mut self, name: &str) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                let name_handle = vm::StringHandle::new(context.intern_string(name));
                process_vm::destack_process_env_delete(self.call_context, context, name_handle)
            }
            None => {
                let name_ref = self.call_context.store_string(name);
                unsafe { process_native::destack_process_env_delete(self.call_context, name_ref) }
            }
        }
    }

    /// Set one byte-key environment variable.
    #[cfg(unix)]
    pub(crate) fn env_set_bytes(&mut self, name: &[u8], value: &[u8]) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                let name_ref = VmSlice::from_bytes(context, name);
                let value_ref = VmSlice::from_bytes(context, value);
                process_vm::destack_process_env_set_bytes(
                    self.call_context,
                    context,
                    name_ref,
                    value_ref,
                )
            }
            None => {
                let name_ref = self.call_context.store_slice(name.to_vec());
                let value_ref = self.call_context.store_slice(value.to_vec());
                unsafe {
                    process_native::destack_process_env_set_bytes(
                        self.call_context,
                        name_ref,
                        value_ref,
                    )
                }
            }
        }
    }

    /// Read one byte-key environment variable.
    #[cfg(unix)]
    pub(crate) fn env_get_bytes(&mut self, name: &[u8]) -> RuntimeResult<Vec<u8>> {
        match self.vm_context_mut() {
            Some(context) => {
                let name_ref = VmSlice::from_bytes(context, name);
                let out = process_vm::destack_process_env_get_bytes(
                    self.call_context,
                    context,
                    name_ref,
                )?;
                out.read_bytes(context)
            }
            None => {
                let name_ref = self.call_context.store_slice(name.to_vec());
                let mut out = NativeArray {
                    data: std::ptr::null_mut(),
                    len: 0,
                    capacity: 0,
                };
                unsafe {
                    process_native::destack_process_env_get_bytes(
                        self.call_context,
                        &mut out,
                        name_ref,
                    )?;
                }
                let out = unsafe { out.as_slice()? };
                Ok(out.to_vec())
            }
        }
    }

    /// Delete one byte-key environment variable.
    #[cfg(unix)]
    pub(crate) fn env_delete_bytes(&mut self, name: &[u8]) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                let name_ref = VmSlice::from_bytes(context, name);
                process_vm::destack_process_env_delete_bytes(self.call_context, context, name_ref)
            }
            None => {
                let name_ref = self.call_context.store_slice(name.to_vec());
                unsafe {
                    process_native::destack_process_env_delete_bytes(self.call_context, name_ref)
                }
            }
        }
    }

    /// Read process id.
    pub(crate) fn pid(&mut self) -> RuntimeResult<ProcessId> {
        match self.vm_context_mut() {
            Some(context) => process_vm::destack_process_pid(self.call_context, context),
            None => {
                let mut out = ProcessId(0);
                unsafe {
                    process_native::destack_process_pid(self.call_context, &mut out)?;
                }
                Ok(out)
            }
        }
    }

    /// Read user id.
    pub(crate) fn uid(&mut self) -> RuntimeResult<UserId> {
        match self.vm_context_mut() {
            Some(context) => process_vm::destack_process_uid(self.call_context, context),
            None => {
                let mut out = UserId(0);
                unsafe {
                    process_native::destack_process_uid(self.call_context, &mut out)?;
                }
                Ok(out)
            }
        }
    }

    /// Read group id.
    pub(crate) fn gid(&mut self) -> RuntimeResult<GroupId> {
        match self.vm_context_mut() {
            Some(context) => process_vm::destack_process_gid(self.call_context, context),
            None => {
                let mut out = GroupId(0);
                unsafe {
                    process_native::destack_process_gid(self.call_context, &mut out)?;
                }
                Ok(out)
            }
        }
    }

    /// Read full group id set.
    #[cfg(unix)]
    pub(crate) fn group_ids(&mut self) -> RuntimeResult<ProcessGroupIds> {
        match self.vm_context_mut() {
            Some(context) => process_vm::destack_process_group_ids(self.call_context, context),
            None => {
                let mut out = ProcessGroupIds {
                    real: GroupId(0),
                    effective: GroupId(0),
                    saved: GroupId(0),
                };
                unsafe {
                    process_native::destack_process_group_ids(self.call_context, &mut out)?;
                }
                Ok(out)
            }
        }
    }

    /// Read supplementary groups.
    #[cfg(unix)]
    pub(crate) fn groups(&mut self) -> RuntimeResult<Vec<GroupId>> {
        match self.vm_context_mut() {
            Some(context) => {
                let out = process_vm::destack_process_groups(self.call_context, context)?;
                out.read_values(context)
            }
            None => {
                let mut out = NativeSlice {
                    data: std::ptr::null_mut(),
                    len: 0,
                };
                unsafe {
                    process_native::destack_process_groups(self.call_context, &mut out)?;
                }
                let out = unsafe { out.as_slice()? };
                Ok(out.to_vec())
            }
        }
    }

    /// Read full user id set.
    #[cfg(unix)]
    pub(crate) fn user_ids(&mut self) -> RuntimeResult<ProcessUserIds> {
        match self.vm_context_mut() {
            Some(context) => process_vm::destack_process_user_ids(self.call_context, context),
            None => {
                let mut out = ProcessUserIds {
                    real: UserId(0),
                    effective: UserId(0),
                    saved: UserId(0),
                };
                unsafe {
                    process_native::destack_process_user_ids(self.call_context, &mut out)?;
                }
                Ok(out)
            }
        }
    }

    /// Read process group id for one pid.
    #[cfg(unix)]
    pub(crate) fn getpgid(&mut self, pid: ProcessId) -> RuntimeResult<ProcessId> {
        match self.vm_context_mut() {
            Some(context) => process_vm::destack_process_getpgid(self.call_context, context, pid),
            None => {
                let mut out = ProcessId(0);
                unsafe {
                    process_native::destack_process_getpgid(self.call_context, &mut out, pid)?;
                }
                Ok(out)
            }
        }
    }

    /// Send one signal to one pid.
    #[cfg(unix)]
    pub(crate) fn kill(&mut self, pid: ProcessId, signal: Signal) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                process_vm::destack_process_kill(self.call_context, context, pid, signal)
            }
            None => unsafe { process_native::destack_process_kill(self.call_context, pid, signal) },
        }
    }

    /// Wait for one pid state transition.
    pub(crate) fn wait_pid(
        &mut self,
        pid: ProcessId,
        flags: ProcessWaitFlags,
    ) -> RuntimeResult<ProcessWaitStatus> {
        match self.vm_context_mut() {
            Some(context) => {
                process_vm::destack_process_wait_pid(self.call_context, context, pid, flags)
            }
            None => {
                let mut out = ProcessWaitStatus {
                    pid: ProcessId(0),
                    kind: ProcessWaitKind::Running,
                    exit_code: 0,
                    signal: Signal(0),
                    core_dumped: false,
                };
                unsafe {
                    process_native::destack_process_wait_pid(
                        self.call_context,
                        &mut out,
                        pid,
                        flags,
                    )?;
                }
                Ok(out)
            }
        }
    }

    /// Read the current signal mask.
    #[cfg(unix)]
    pub(crate) fn signal_mask_read(&mut self) -> RuntimeResult<Vec<Signal>> {
        match self.vm_context_mut() {
            Some(context) => {
                let out = process_vm::destack_process_signal_mask_read(self.call_context, context)?;
                out.read_values(context)
            }
            None => {
                let mut out = NativeArray {
                    data: std::ptr::null_mut(),
                    len: 0,
                    capacity: 0,
                };
                unsafe {
                    process_native::destack_process_signal_mask_read(self.call_context, &mut out)?;
                }
                let out = unsafe { out.as_slice()? };
                Ok(out.to_vec())
            }
        }
    }

    /// Update the current signal mask.
    #[cfg(unix)]
    pub(crate) fn signal_mask_update(
        &mut self,
        how: SignalMaskHow,
        signals: &[Signal],
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                let signals = VmSlice::from_values(context, signals)?;
                process_vm::destack_process_signal_mask_update(
                    self.call_context,
                    context,
                    how,
                    signals,
                )
            }
            None => {
                let signals = self.call_context.store_slice(signals.to_vec());
                unsafe {
                    process_native::destack_process_signal_mask_update(
                        self.call_context,
                        how,
                        signals,
                    )
                }
            }
        }
    }

    /// Poll one pending signal from one explicit mask.
    #[cfg(unix)]
    pub(crate) fn signal_try_wait(&mut self, signals: &[Signal]) -> RuntimeResult<SignalEvent> {
        match self.vm_context_mut() {
            Some(context) => {
                let signals = VmSlice::from_values(context, signals)?;
                process_vm::destack_process_signal_try_wait(self.call_context, context, signals)
            }
            None => {
                let mut out = SignalEvent {
                    signal: Signal(0),
                    pid: ProcessId(0),
                };
                let signals = self.call_context.store_slice(signals.to_vec());
                unsafe {
                    process_native::destack_process_signal_try_wait(
                        self.call_context,
                        &mut out,
                        signals,
                    )?;
                }
                Ok(out)
            }
        }
    }

    /// Subscribe to one signal.
    #[cfg(unix)]
    pub(crate) fn signal_subscribe(
        &mut self,
        signal: Signal,
    ) -> RuntimeResult<resource::SignalHandle> {
        match self.vm_context_mut() {
            Some(context) => {
                process_vm::destack_process_signal_subscribe(self.call_context, context, signal)
            }
            None => {
                let mut out = resource::SignalHandle(ResourceId(0));
                unsafe {
                    process_native::destack_process_signal_subscribe(
                        self.call_context,
                        &mut out,
                        signal,
                    )?;
                }
                Ok(out)
            }
        }
    }

    /// Poll one subscribed signal handle.
    #[cfg(unix)]
    pub(crate) fn signal_try_receive(
        &mut self,
        handle: resource::SignalHandle,
    ) -> RuntimeResult<SignalEvent> {
        match self.vm_context_mut() {
            Some(context) => {
                process_vm::destack_process_signal_try_receive(self.call_context, context, handle)
            }
            None => {
                let mut out = SignalEvent {
                    signal: Signal(0),
                    pid: ProcessId(0),
                };
                unsafe {
                    process_native::destack_process_signal_try_receive(
                        self.call_context,
                        &mut out,
                        handle,
                    )?;
                }
                Ok(out)
            }
        }
    }

    /// Unsubscribe one signal handle.
    #[cfg(unix)]
    pub(crate) fn signal_unsubscribe(
        &mut self,
        handle: resource::SignalHandle,
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                process_vm::destack_process_signal_unsubscribe(self.call_context, context, handle)
            }
            None => unsafe {
                process_native::destack_process_signal_unsubscribe(self.call_context, handle)
            },
        }
    }

    /// Read one process limit.
    #[cfg(unix)]
    pub(crate) fn get_limit(
        &mut self,
        resource: ProcessLimitResource,
    ) -> RuntimeResult<ProcessLimit> {
        match self.vm_context_mut() {
            Some(context) => {
                process_vm::destack_process_get_limit(self.call_context, context, resource)
            }
            None => {
                let mut out = ProcessLimit { soft: 0, hard: 0 };
                unsafe {
                    process_native::destack_process_get_limit(
                        self.call_context,
                        &mut out,
                        resource,
                    )?;
                }
                Ok(out)
            }
        }
    }

    /// Set one process limit.
    #[cfg(unix)]
    pub(crate) fn set_limit(
        &mut self,
        resource: ProcessLimitResource,
        limit: ProcessLimit,
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                process_vm::destack_process_set_limit(self.call_context, context, resource, limit)
            }
            None => unsafe {
                process_native::destack_process_set_limit(self.call_context, resource, limit)
            },
        }
    }

    /// Read one process priority.
    #[cfg(unix)]
    pub(crate) fn get_priority(&mut self, pid: ProcessId) -> RuntimeResult<i32> {
        match self.vm_context_mut() {
            Some(context) => {
                process_vm::destack_process_get_priority(self.call_context, context, pid)
            }
            None => {
                let mut out = 0_i32;
                unsafe {
                    process_native::destack_process_get_priority(self.call_context, &mut out, pid)?;
                }
                Ok(out)
            }
        }
    }

    /// Set one process priority.
    #[cfg(unix)]
    pub(crate) fn set_priority(&mut self, pid: ProcessId, priority: i32) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                process_vm::destack_process_set_priority(self.call_context, context, pid, priority)
            }
            None => unsafe {
                process_native::destack_process_set_priority(self.call_context, pid, priority)
            },
        }
    }

    /// Spawn one child process with default descriptor behavior.
    #[cfg(unix)]
    pub(crate) fn spawn(
        &mut self,
        command: &str,
        arguments: &[String],
        environment: &[String],
        options: &ProcessSpawnOptionsSpec,
    ) -> RuntimeResult<resource::ProcessHandle> {
        match self.vm_context_mut() {
            Some(context) => {
                let command = vm_path_from_utf8(context, command)?;
                let arguments = vm_string_slice(context, arguments)?;
                let environment = vm_string_slice(context, environment)?;
                let options = ProcessSpawnOptionsVm {
                    cwd: vm_path_from_utf8(context, &options.cwd)?,
                    detached: options.detached,
                    reset_signals: options.reset_signals,
                    new_process_group: options.new_process_group,
                };

                process_vm::destack_process_spawn(
                    self.call_context,
                    context,
                    command,
                    arguments,
                    environment,
                    options,
                )
            }
            None => {
                let command = native_path_from_utf8(self.call_context, command);
                let arguments = self.call_context.store_string_slice(
                    arguments
                        .iter()
                        .map(|value| self.call_context.store_string(value))
                        .collect(),
                );
                let environment = self.call_context.store_string_slice(
                    environment
                        .iter()
                        .map(|value| self.call_context.store_string(value))
                        .collect(),
                );
                let options = ProcessSpawnOptions {
                    cwd: native_path_from_utf8(self.call_context, &options.cwd),
                    detached: options.detached,
                    reset_signals: options.reset_signals,
                    new_process_group: options.new_process_group,
                };

                let mut out = resource::ProcessHandle(ResourceId(0));
                unsafe {
                    process_native::destack_process_spawn(
                        self.call_context,
                        &mut out,
                        command,
                        arguments,
                        environment,
                        options,
                    )?;
                }
                Ok(out)
            }
        }
    }

    /// Spawn one child process with explicit stdio and fd actions.
    #[cfg(any(unix, windows))]
    pub(crate) fn spawn_with_actions(
        &mut self,
        command: &str,
        arguments: &[String],
        environment: &[String],
        options: &ProcessSpawnOptionsSpec,
        stdio: &[ProcessStdioSpec],
        actions: &[ProcessFdActionSpec],
    ) -> RuntimeResult<resource::ProcessHandle> {
        match self.vm_context_mut() {
            Some(context) => {
                let command = vm_path_from_utf8(context, command)?;
                let arguments = vm_string_slice(context, arguments)?;
                let environment = vm_string_slice(context, environment)?;
                let options = ProcessSpawnOptionsVm {
                    cwd: vm_path_from_utf8(context, &options.cwd)?,
                    detached: options.detached,
                    reset_signals: options.reset_signals,
                    new_process_group: options.new_process_group,
                };
                let stdio = vm_process_stdio_slice(context, stdio);
                let actions = vm_process_fd_action_slice(context, actions)?;

                process_vm::destack_process_spawn_with_actions(
                    self.call_context,
                    context,
                    command,
                    arguments,
                    environment,
                    options,
                    stdio,
                    actions,
                )
            }
            None => {
                let command = native_path_from_utf8(self.call_context, command);
                let arguments = self.call_context.store_string_slice(
                    arguments
                        .iter()
                        .map(|value| self.call_context.store_string(value))
                        .collect(),
                );
                let environment = self.call_context.store_string_slice(
                    environment
                        .iter()
                        .map(|value| self.call_context.store_string(value))
                        .collect(),
                );
                let options = ProcessSpawnOptions {
                    cwd: native_path_from_utf8(self.call_context, &options.cwd),
                    detached: options.detached,
                    reset_signals: options.reset_signals,
                    new_process_group: options.new_process_group,
                };

                let native_stdio = stdio
                    .iter()
                    .map(|value| ProcessStdio {
                        kind: value.kind,
                        file: resource::FileHandle(ResourceId(0)),
                        pipe: resource::PipeHandle(ResourceId(0)),
                        descriptor: value.descriptor,
                    })
                    .collect::<Vec<_>>();
                let stdio = self.call_context.store_slice(native_stdio);

                let native_actions = actions
                    .iter()
                    .map(|value| ProcessFdAction {
                        op: value.op,
                        source: value.source,
                        target: value.target,
                        path: native_path_from_utf8(self.call_context, &value.path),
                        flags: value.flags,
                        mode: value.mode,
                    })
                    .collect::<Vec<_>>();
                let actions = self.call_context.store_slice(native_actions);

                let mut out = resource::ProcessHandle(ResourceId(0));
                unsafe {
                    process_native::destack_process_spawn_with_actions(
                        self.call_context,
                        &mut out,
                        command,
                        arguments,
                        environment,
                        options,
                        stdio,
                        actions,
                    )?;
                }
                Ok(out)
            }
        }
    }

    /// Wait by process handle.
    #[cfg(any(unix, windows))]
    pub(crate) fn wait(
        &mut self,
        handle: resource::ProcessHandle,
        flags: ProcessWaitFlags,
    ) -> RuntimeResult<ProcessWaitStatus> {
        match self.vm_context_mut() {
            Some(context) => {
                process_vm::destack_process_wait(self.call_context, context, handle, flags)
            }
            None => {
                let mut out = ProcessWaitStatus {
                    pid: ProcessId(0),
                    kind: ProcessWaitKind::Running,
                    exit_code: 0,
                    signal: Signal(0),
                    core_dumped: false,
                };
                unsafe {
                    process_native::destack_process_wait(
                        self.call_context,
                        &mut out,
                        handle,
                        flags,
                    )?;
                }
                Ok(out)
            }
        }
    }

    /// Poll by process handle.
    #[cfg(unix)]
    pub(crate) fn try_wait(
        &mut self,
        handle: resource::ProcessHandle,
    ) -> RuntimeResult<ProcessWaitStatus> {
        match self.vm_context_mut() {
            Some(context) => {
                process_vm::destack_process_try_wait(self.call_context, context, handle)
            }
            None => {
                let mut out = ProcessWaitStatus {
                    pid: ProcessId(0),
                    kind: ProcessWaitKind::Running,
                    exit_code: 0,
                    signal: Signal(0),
                    core_dumped: false,
                };
                unsafe {
                    process_native::destack_process_try_wait(self.call_context, &mut out, handle)?;
                }
                Ok(out)
            }
        }
    }

    /// Open one process-fd style handle.
    #[cfg(any(unix, windows))]
    pub(crate) fn process_fd_open(
        &mut self,
        pid: ProcessId,
        flags: ProcessFdFlags,
    ) -> RuntimeResult<resource::ProcessFdHandle> {
        match self.vm_context_mut() {
            Some(context) => {
                process_vm::destack_process_process_fd_open(self.call_context, context, pid, flags)
            }
            None => {
                let mut out = resource::ProcessFdHandle(ResourceId(0));
                unsafe {
                    process_native::destack_process_process_fd_open(
                        self.call_context,
                        &mut out,
                        pid,
                        flags,
                    )?;
                }
                Ok(out)
            }
        }
    }

    /// Wait by process-fd style handle.
    #[cfg(any(unix, windows))]
    pub(crate) fn process_fd_wait(
        &mut self,
        handle: resource::ProcessFdHandle,
        timeout_ns: u64,
    ) -> RuntimeResult<ProcessWaitStatus> {
        match self.vm_context_mut() {
            Some(context) => process_vm::destack_process_process_fd_wait(
                self.call_context,
                context,
                handle,
                timeout_ns,
            ),
            None => {
                let mut out = ProcessWaitStatus {
                    pid: ProcessId(0),
                    kind: ProcessWaitKind::Running,
                    exit_code: 0,
                    signal: Signal(0),
                    core_dumped: false,
                };
                unsafe {
                    process_native::destack_process_process_fd_wait(
                        self.call_context,
                        &mut out,
                        handle,
                        timeout_ns,
                    )?;
                }
                Ok(out)
            }
        }
    }

    /// Close one process-fd style handle.
    #[cfg(any(unix, windows))]
    pub(crate) fn process_fd_close(
        &mut self,
        handle: resource::ProcessFdHandle,
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                process_vm::destack_process_process_fd_close(self.call_context, context, handle)
            }
            None => unsafe {
                process_native::destack_process_process_fd_close(self.call_context, handle)
            },
        }
    }

    /// Open one signal-fd style handle.
    #[cfg(unix)]
    pub(crate) fn signal_fd_open(
        &mut self,
        signals: &[Signal],
        flags: SignalFdFlags,
    ) -> RuntimeResult<resource::SignalFdHandle> {
        match self.vm_context_mut() {
            Some(context) => {
                let signals = VmSlice::from_values(context, signals)?;
                process_vm::destack_process_signal_fd_open(
                    self.call_context,
                    context,
                    signals,
                    flags,
                )
            }
            None => {
                let signals = self.call_context.store_slice(signals.to_vec());
                let mut out = resource::SignalFdHandle(ResourceId(0));
                unsafe {
                    process_native::destack_process_signal_fd_open(
                        self.call_context,
                        &mut out,
                        signals,
                        flags,
                    )?;
                }
                Ok(out)
            }
        }
    }

    /// Poll one signal-fd style handle.
    #[cfg(unix)]
    pub(crate) fn signal_fd_try_read(
        &mut self,
        handle: resource::SignalFdHandle,
    ) -> RuntimeResult<SignalEvent> {
        match self.vm_context_mut() {
            Some(context) => {
                process_vm::destack_process_signal_fd_try_read(self.call_context, context, handle)
            }
            None => {
                let mut out = SignalEvent {
                    signal: Signal(0),
                    pid: ProcessId(0),
                };
                unsafe {
                    process_native::destack_process_signal_fd_try_read(
                        self.call_context,
                        &mut out,
                        handle,
                    )?;
                }
                Ok(out)
            }
        }
    }

    /// Close one signal-fd style handle.
    #[cfg(unix)]
    pub(crate) fn signal_fd_close(
        &mut self,
        handle: resource::SignalFdHandle,
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                process_vm::destack_process_signal_fd_close(self.call_context, context, handle)
            }
            None => unsafe {
                process_native::destack_process_signal_fd_close(self.call_context, handle)
            },
        }
    }

    /// Execute one path in-place.
    #[cfg(unix)]
    pub(crate) fn exec(
        &mut self,
        command: &str,
        arguments: &[String],
        environment: &[String],
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                let command = vm_path_from_utf8(context, command)?;
                let arguments = vm_string_slice(context, arguments)?;
                let environment = vm_string_slice(context, environment)?;
                process_vm::destack_process_exec(
                    self.call_context,
                    context,
                    command,
                    arguments,
                    environment,
                )
            }
            None => {
                let command = native_path_from_utf8(self.call_context, command);
                let arguments = self.call_context.store_string_slice(
                    arguments
                        .iter()
                        .map(|value| self.call_context.store_string(value))
                        .collect(),
                );
                let environment = self.call_context.store_string_slice(
                    environment
                        .iter()
                        .map(|value| self.call_context.store_string(value))
                        .collect(),
                );
                unsafe {
                    process_native::destack_process_exec(
                        self.call_context,
                        command,
                        arguments,
                        environment,
                    )
                }
            }
        }
    }

    /// Execute one directory-relative path in-place.
    #[cfg(unix)]
    pub(crate) fn execat(
        &mut self,
        directory: resource::DirectoryHandle,
        path: &str,
        arguments: &[String],
        environment: &[String],
        flags: ExecAtFlags,
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                let path = vm_path_from_utf8(context, path)?;
                let arguments = vm_string_slice(context, arguments)?;
                let environment = vm_string_slice(context, environment)?;
                process_vm::destack_process_execat(
                    self.call_context,
                    context,
                    directory,
                    path,
                    arguments,
                    environment,
                    flags,
                )
            }
            None => {
                let path = native_path_from_utf8(self.call_context, path);
                let arguments = self.call_context.store_string_slice(
                    arguments
                        .iter()
                        .map(|value| self.call_context.store_string(value))
                        .collect(),
                );
                let environment = self.call_context.store_string_slice(
                    environment
                        .iter()
                        .map(|value| self.call_context.store_string(value))
                        .collect(),
                );
                unsafe {
                    process_native::destack_process_execat(
                        self.call_context,
                        directory,
                        path,
                        arguments,
                        environment,
                        flags,
                    )
                }
            }
        }
    }

    /// Execute one file handle in-place.
    #[cfg(unix)]
    pub(crate) fn fexec(
        &mut self,
        file: resource::FileHandle,
        arguments: &[String],
        environment: &[String],
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                let arguments = vm_string_slice(context, arguments)?;
                let environment = vm_string_slice(context, environment)?;
                process_vm::destack_process_fexec(
                    self.call_context,
                    context,
                    file,
                    arguments,
                    environment,
                )
            }
            None => {
                let arguments = self.call_context.store_string_slice(
                    arguments
                        .iter()
                        .map(|value| self.call_context.store_string(value))
                        .collect(),
                );
                let environment = self.call_context.store_string_slice(
                    environment
                        .iter()
                        .map(|value| self.call_context.store_string(value))
                        .collect(),
                );
                unsafe {
                    process_native::destack_process_fexec(
                        self.call_context,
                        file,
                        arguments,
                        environment,
                    )
                }
            }
        }
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
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
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
    pub(crate) fn run<F>(&self, callback: F)
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
    F: FnMut(&ProcessHarnessHandle),
{
    let native = ProcessHarnessHandle::Native(NativeProcessHarness::new());
    callback(&native);
    let vm = ProcessHarnessHandle::Vm(VmProcessHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(ProcessHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
