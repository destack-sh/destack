use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::core::process_args;
use crate::platform::process::{GroupId, ProcessId, Signal, UserId};
use crate::platform::{PlatformError, VmSlice};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Return the process arguments as a VM slice.
pub fn destack_process_args(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<VmSlice<vm::StringHandle>> {
    // build the argument slice
    build_process_args(context, process_args(runtime.platform()))
}

/// Return the current working directory.
pub fn destack_process_cwd(
    _runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<vm::StringHandle> {
    // read the current working directory
    let cwd = std::env::current_dir().map_err(|error| {
        RuntimeError::from(PlatformError::io(format!("failed to read cwd: {error}"))).boxed()
    })?;
    let cwd = cwd.to_string_lossy().to_string();

    // intern the path in the VM
    Ok(string_handle_from_string(context, cwd))
}

/// Change the current working directory.
pub fn destack_process_chdir(
    _runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: vm::StringHandle,
) -> RuntimeResult<()> {
    // resolve the path string
    let path_ref = context
        .string_ref(path)
        .map_err(Box::<RuntimeError>::from)?;

    // update the working directory
    std::env::set_current_dir(path_ref.as_str()).map_err(|error| {
        RuntimeError::from(PlatformError::io(format!("failed to change cwd: {error}"))).boxed()
    })?;

    Ok(())
}

/// Get an environment variable.
pub fn destack_process_env_get(
    _runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    name: vm::StringHandle,
) -> RuntimeResult<vm::StringHandle> {
    // resolve the variable name
    let name = {
        let name_ref = context
            .string_ref(name)
            .map_err(Box::<RuntimeError>::from)?;
        name_ref.as_str().to_string()
    };

    // read the variable and return it
    let value = match std::env::var(&name) {
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

    Ok(string_handle_from_string(context, value))
}

/// Set an environment variable.
pub fn destack_process_env_set(
    _runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    name: vm::StringHandle,
    value: vm::StringHandle,
) -> RuntimeResult<()> {
    // resolve the variable name and value
    let name_ref = context
        .string_ref(name)
        .map_err(Box::<RuntimeError>::from)?;
    let value_ref = context
        .string_ref(value)
        .map_err(Box::<RuntimeError>::from)?;
    let name = name_ref.as_str();
    let value = value_ref.as_str();

    // reject nul bytes per platform API
    if name.contains('\0') || value.contains('\0') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "name",
            "environment variable contains nul byte",
        ))
        .boxed());
    }

    // update the environment
    unsafe {
        std::env::set_var(name, value);
    }

    Ok(())
}

/// Delete an environment variable.
pub fn destack_process_env_delete(
    _runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    // resolve the variable name
    let name_ref = context
        .string_ref(name)
        .map_err(Box::<RuntimeError>::from)?;
    let name = name_ref.as_str();

    // reject nul bytes per platform API
    if name.contains('\0') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "name",
            "environment variable contains nul byte",
        ))
        .boxed());
    }

    // remove the environment entry
    unsafe {
        std::env::remove_var(name);
    }

    Ok(())
}

/// Return the process identifier.
pub fn destack_process_pid(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<ProcessId> {
    Ok(ProcessId(std::process::id()))
}

/// Return the parent process identifier.
pub fn destack_process_ppid(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<ProcessId> {
    Ok(ProcessId(process_ppid()?))
}

/// Return the user id.
pub fn destack_process_uid(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<UserId> {
    Ok(UserId(process_uid()?))
}

/// Return the group id.
pub fn destack_process_gid(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<GroupId> {
    Ok(GroupId(process_gid()?))
}

/// Update the process umask and return the previous value.
pub fn destack_process_umask(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    mask: u32,
) -> RuntimeResult<u32> {
    process_umask(mask)
}

/// Send a signal to a process.
pub fn destack_process_kill(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    pid: ProcessId,
    signal: Signal,
) -> RuntimeResult<()> {
    process_kill(pid.0, signal.0)
}

/// Exit the current process.
pub fn destack_process_exit(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    code: u32,
) -> RuntimeResult<()> {
    // exit the process immediately
    std::process::exit(code as i32);

    #[allow(unreachable_code)]
    Ok(())
}

/// Build a process args slice value for the VM.
fn build_process_args(
    context: &mut vm::RuntimeContext<'_>,
    args: &[String],
) -> RuntimeResult<VmSlice<vm::StringHandle>> {
    // collect argument values
    let length = args.len() as u32;
    let mut values = Vec::with_capacity(args.len());
    for arg in args {
        values.push(context.intern_string(arg));
    }

    // allocate the payload buffer
    let data_ptr = if values.is_empty() {
        vm::RawPointer::NULL
    } else {
        context.allocate_raw_values(values)
    };

    // build the slice representation
    Ok(VmSlice {
        data: data_ptr,
        len: length,
        _marker: std::marker::PhantomData,
    })
}

/// Intern a string into the VM and return a string handle.
fn string_handle_from_string(
    context: &mut vm::RuntimeContext<'_>,
    value: String,
) -> vm::StringHandle {
    let value = context.intern_string(&value);
    vm::StringHandle::new(value)
}

/// Resolve the parent process id.
fn process_ppid() -> RuntimeResult<u32> {
    // resolve the parent process id on unix platforms
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
    // resolve the user id on unix platforms
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
    // resolve the group id on unix platforms
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
    // update the umask on unix platforms
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

/// Send a signal to the process.
fn process_kill(pid: u32, signal: u32) -> RuntimeResult<()> {
    // signal the target process on unix platforms
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
