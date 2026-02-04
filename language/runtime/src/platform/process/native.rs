use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::{GroupId, ProcessId, Signal, UserId};
use crate::platform::{NativeStringRef, NativeStringSlice, PlatformError};
use crate::runtime::RuntimeCallContext;

/// Return the process args for native code.
pub unsafe fn destack_process_args(
    context: &RuntimeCallContext,
    out: *mut NativeStringSlice,
) -> RuntimeResult<()> {
    let slice = NativeStringSlice::from_slice(context.platform().args_refs());
    unsafe {
        *out = slice;
    }
    Ok(())
}

/// Return the current working directory for native code.
pub unsafe fn destack_process_cwd(
    context: &RuntimeCallContext,
    out: *mut NativeStringRef,
) -> RuntimeResult<()> {
    let cwd = std::env::current_dir().map_err(|error| {
        RuntimeError::from(PlatformError::io(format!("failed to read cwd: {error}"))).boxed()
    })?;
    let cwd = cwd.to_string_lossy().to_string();
    unsafe {
        *out = context.store_string(&cwd);
    }
    Ok(())
}

/// Change the current working directory for native code.
pub unsafe fn destack_process_chdir(
    _context: &RuntimeCallContext,
    path: NativeStringRef,
) -> RuntimeResult<()> {
    let path = unsafe { path.as_str()? };
    std::env::set_current_dir(path).map_err(|error| {
        RuntimeError::from(PlatformError::io(format!("failed to change cwd: {error}"))).boxed()
    })?;
    Ok(())
}

/// Get an environment variable for native code.
pub unsafe fn destack_process_env_get(
    context: &RuntimeCallContext,
    out: *mut NativeStringRef,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let name = unsafe { name.as_str()? };
    let value = match std::env::var(name) {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "name",
                "environment variable not found",
            ))
            .boxed());
        }
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "name",
                "environment variable is not valid unicode",
            ))
            .boxed());
        }
    };

    unsafe {
        *out = context.store_string(&value);
    }

    Ok(())
}

/// Set an environment variable for native code.
pub unsafe fn destack_process_env_set(
    _context: &RuntimeCallContext,
    name: NativeStringRef,
    value: NativeStringRef,
) -> RuntimeResult<()> {
    let name = unsafe { name.as_str()? };
    let value = unsafe { value.as_str()? };
    if name.contains('\0') || value.contains('\0') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "name",
            "environment variable contains nul byte",
        ))
        .boxed());
    }

    unsafe {
        std::env::set_var(name, value);
    }
    Ok(())
}

/// Delete an environment variable for native code.
pub unsafe fn destack_process_env_delete(
    _context: &RuntimeCallContext,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let name = unsafe { name.as_str()? };
    if name.contains('\0') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "name",
            "environment variable contains nul byte",
        ))
        .boxed());
    }

    unsafe {
        std::env::remove_var(name);
    }
    Ok(())
}

/// Return the process identifier for native code.
pub unsafe fn destack_process_pid(
    _context: &RuntimeCallContext,
    out: *mut ProcessId,
) -> RuntimeResult<()> {
    unsafe {
        *out = ProcessId(std::process::id());
    }

    Ok(())
}

/// Return the parent process identifier for native code.
pub unsafe fn destack_process_ppid(
    _context: &RuntimeCallContext,
    out: *mut ProcessId,
) -> RuntimeResult<()> {
    let ppid = process_ppid()?;
    unsafe {
        *out = ProcessId(ppid);
    }

    Ok(())
}

/// Return the user id for native code.
pub unsafe fn destack_process_uid(
    _context: &RuntimeCallContext,
    out: *mut UserId,
) -> RuntimeResult<()> {
    let uid = process_uid()?;
    unsafe {
        *out = UserId(uid);
    }

    Ok(())
}

/// Return the group id for native code.
pub unsafe fn destack_process_gid(
    _context: &RuntimeCallContext,
    out: *mut GroupId,
) -> RuntimeResult<()> {
    let gid = process_gid()?;
    unsafe {
        *out = GroupId(gid);
    }

    Ok(())
}

/// Update the process umask and return the previous value.
pub unsafe fn destack_process_umask(
    _context: &RuntimeCallContext,
    out: *mut u32,
    mask: u32,
) -> RuntimeResult<()> {
    let previous = process_umask(mask)?;
    unsafe {
        *out = previous;
    }

    Ok(())
}

/// Send a signal to a process.
pub unsafe fn destack_process_kill(
    _context: &RuntimeCallContext,
    pid: ProcessId,
    signal: Signal,
) -> RuntimeResult<()> {
    process_kill(pid.0, signal.0)?;
    Ok(())
}

/// Exit the current process.
pub unsafe fn destack_process_exit(_context: &RuntimeCallContext, code: u32) -> RuntimeResult<()> {
    std::process::exit(code as i32);

    #[allow(unreachable_code)]
    Ok(())
}

/// Resolve the parent process id.
fn process_ppid() -> RuntimeResult<u32> {
    #[cfg(unix)]
    {
        Ok(unsafe { libc::getppid() as u32 })
    }
    #[cfg(not(unix))]
    {
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.ppid")).boxed())
    }
}

/// Resolve the user id.
fn process_uid() -> RuntimeResult<u32> {
    #[cfg(unix)]
    {
        Ok(unsafe { libc::getuid() as u32 })
    }
    #[cfg(not(unix))]
    {
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.uid")).boxed())
    }
}

/// Resolve the group id.
fn process_gid() -> RuntimeResult<u32> {
    #[cfg(unix)]
    {
        Ok(unsafe { libc::getgid() as u32 })
    }
    #[cfg(not(unix))]
    {
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.gid")).boxed())
    }
}

/// Update the process umask and return the previous value.
fn process_umask(mask: u32) -> RuntimeResult<u32> {
    #[cfg(unix)]
    {
        Ok(unsafe { libc::umask(mask as libc::mode_t) as u32 })
    }
    #[cfg(not(unix))]
    {
        let _ = mask;
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.umask")).boxed())
    }
}

/// Send a signal to the given process.
fn process_kill(pid: u32, signal: u32) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(pid as libc::pid_t, signal as libc::c_int) };
        if result < 0 {
            return Err(RuntimeError::from(PlatformError::io(format!(
                "failed to signal process {pid}"
            )))
            .boxed());
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = (pid, signal);
        Err(RuntimeError::from(PlatformError::not_supported("destack.process.kill")).boxed())
    }
}
