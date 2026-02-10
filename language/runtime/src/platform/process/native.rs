use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::core::{
    process_args, process_chdir, process_cwd, process_env_delete, process_env_get, process_env_set,
    process_exit, process_gid, process_kill, process_pid, process_ppid, process_uid, process_umask,
};
use crate::platform::process::{GroupId, ProcessId, Signal, UserId};
use crate::platform::{NativeStringRef, NativeStringSlice, PlatformError};
use crate::runtime::RuntimeCallContext;

/// Return the process args for native code.
pub unsafe fn destack_process_args(
    context: &RuntimeCallContext,
    out: *mut NativeStringSlice,
) -> RuntimeResult<()> {
    let args = process_args(context.platform());
    let mut stored = Vec::with_capacity(args.len());
    for value in args {
        stored.push(context.store_string(value));
    }
    let slice = context.store_string_slice(stored);
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
    let cwd = process_cwd()?;
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
    process_chdir(path)?;
    Ok(())
}

/// Get an environment variable for native code.
pub unsafe fn destack_process_env_get(
    context: &RuntimeCallContext,
    out: *mut NativeStringRef,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let name = unsafe { name.as_str()? };
    let value = process_env_get(name)?.ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "name",
            "environment variable not found",
        ))
        .boxed()
    })?;

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
    process_env_set(name, value)
}

/// Delete an environment variable for native code.
pub unsafe fn destack_process_env_delete(
    _context: &RuntimeCallContext,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let name = unsafe { name.as_str()? };
    process_env_delete(name)
}

/// Return the process identifier for native code.
pub unsafe fn destack_process_pid(
    _context: &RuntimeCallContext,
    out: *mut ProcessId,
) -> RuntimeResult<()> {
    let pid = process_pid()?;
    unsafe {
        *out = ProcessId(pid);
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
    process_exit(code)
}
