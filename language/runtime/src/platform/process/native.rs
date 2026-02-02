use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::bindings::{BindingDescriptor, native_call};
use crate::platform::process::{
    GroupId, ProcessId, Signal, UserId, bindings_generated as bindings,
};
use crate::platform::{NativeStringRef, NativeStringSlice, PlatformError, RuntimeStatus};

/// Return the process args for native code.
#[unsafe(export_name = "destack.process.args")]
pub unsafe extern "C" fn destack_process_args(out: *mut NativeStringSlice) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::ARGS)?;

        // reject null output pointers
        if out.is_null() {
            return Err(RuntimeError::platform(PlatformError::null_pointer("out")).boxed());
        }

        // write the output slice
        let slice = NativeStringSlice::from_slice(context.platform().args_refs());
        unsafe {
            *out = slice;
        }

        Ok(())
    })
}

/// Return the current working directory for native code.
#[unsafe(export_name = "destack.process.cwd")]
pub unsafe extern "C" fn destack_process_cwd(out: *mut NativeStringRef) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::CWD)?;

        if out.is_null() {
            return Err(RuntimeError::platform(PlatformError::null_pointer("out")).boxed());
        }

        let cwd = std::env::current_dir().map_err(|error| {
            RuntimeError::platform(PlatformError::io(format!("failed to read cwd: {error}")))
                .boxed()
        })?;
        let cwd = cwd.to_string_lossy().to_string();
        unsafe {
            *out = context.store_string(&cwd);
        }

        Ok(())
    })
}

/// Change the current working directory for native code.
#[unsafe(export_name = "destack.process.chdir")]
pub unsafe extern "C" fn destack_process_chdir(path: NativeStringRef) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::CHDIR)?;
        let path = unsafe { path.as_str()? };
        std::env::set_current_dir(path).map_err(|error| {
            RuntimeError::platform(PlatformError::io(format!("failed to change cwd: {error}")))
                .boxed()
        })?;
        Ok(())
    })
}

/// Get an environment variable for native code.
#[unsafe(export_name = "destack.process.envGet")]
pub unsafe extern "C" fn destack_process_env_get(
    out: *mut NativeStringRef,
    name: NativeStringRef,
) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::ENV_GET)?;

        if out.is_null() {
            return Err(RuntimeError::platform(PlatformError::null_pointer("out")).boxed());
        }

        let name = unsafe { name.as_str()? };
        let value = match std::env::var(name) {
            Ok(value) => value,
            Err(std::env::VarError::NotPresent) => {
                return Err(
                    RuntimeError::platform(PlatformError::invalid_argument_value(
                        "name",
                        "environment variable not found",
                    ))
                    .boxed(),
                );
            }
            Err(std::env::VarError::NotUnicode(_)) => {
                return Err(
                    RuntimeError::platform(PlatformError::invalid_argument_value(
                        "name",
                        "environment variable is not valid unicode",
                    ))
                    .boxed(),
                );
            }
        };

        unsafe {
            *out = context.store_string(&value);
        }

        Ok(())
    })
}

/// Set an environment variable for native code.
#[unsafe(export_name = "destack.process.envSet")]
pub unsafe extern "C" fn destack_process_env_set(
    name: NativeStringRef,
    value: NativeStringRef,
) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::ENV_SET)?;

        let name = unsafe { name.as_str()? };
        let value = unsafe { value.as_str()? };
        if name.contains('\0') || value.contains('\0') {
            return Err(
                RuntimeError::platform(PlatformError::invalid_argument_value(
                    "name",
                    "environment variable contains nul byte",
                ))
                .boxed(),
            );
        }
        unsafe {
            std::env::set_var(name, value);
        }
        Ok(())
    })
}

/// Delete an environment variable for native code.
#[unsafe(export_name = "destack.process.envDelete")]
pub unsafe extern "C" fn destack_process_env_delete(name: NativeStringRef) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::ENV_DELETE)?;

        let name = unsafe { name.as_str()? };
        if name.contains('\0') {
            return Err(
                RuntimeError::platform(PlatformError::invalid_argument_value(
                    "name",
                    "environment variable contains nul byte",
                ))
                .boxed(),
            );
        }
        unsafe {
            std::env::remove_var(name);
        }
        Ok(())
    })
}

/// Return the process identifier for native code.
#[unsafe(export_name = "destack.process.pid")]
pub unsafe extern "C" fn destack_process_pid(out: *mut ProcessId) -> RuntimeStatus {
    write_process_id(bindings::PID, out, std::process::id())
}

/// Return the parent process identifier for native code.
#[unsafe(export_name = "destack.process.ppid")]
pub unsafe extern "C" fn destack_process_ppid(out: *mut ProcessId) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::PPID)?;

        if out.is_null() {
            return Err(RuntimeError::platform(PlatformError::null_pointer("out")).boxed());
        }

        let ppid = process_ppid()?;
        unsafe {
            *out = ProcessId(ppid);
        }

        Ok(())
    })
}

/// Return the user id for native code.
#[unsafe(export_name = "destack.process.uid")]
pub unsafe extern "C" fn destack_process_uid(out: *mut UserId) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::UID)?;

        if out.is_null() {
            return Err(RuntimeError::platform(PlatformError::null_pointer("out")).boxed());
        }

        let uid = process_uid()?;
        unsafe {
            *out = UserId(uid);
        }

        Ok(())
    })
}

/// Return the group id for native code.
#[unsafe(export_name = "destack.process.gid")]
pub unsafe extern "C" fn destack_process_gid(out: *mut GroupId) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::GID)?;

        if out.is_null() {
            return Err(RuntimeError::platform(PlatformError::null_pointer("out")).boxed());
        }

        let gid = process_gid()?;
        unsafe {
            *out = GroupId(gid);
        }

        Ok(())
    })
}

/// Update the process umask and return the previous value.
#[unsafe(export_name = "destack.process.umask")]
pub unsafe extern "C" fn destack_process_umask(out: *mut u32, mask: u32) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::UMASK)?;

        if out.is_null() {
            return Err(RuntimeError::platform(PlatformError::null_pointer("out")).boxed());
        }

        let previous = process_umask(mask)?;
        unsafe {
            *out = previous;
        }

        Ok(())
    })
}

/// Send a signal to a process.
#[unsafe(export_name = "destack.process.kill")]
pub unsafe extern "C" fn destack_process_kill(pid: ProcessId, signal: Signal) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::KILL)?;
        process_kill(pid.0, signal.0)?;
        Ok(())
    })
}

/// Exit the current process.
#[unsafe(export_name = "destack.process.exit")]
pub unsafe extern "C" fn destack_process_exit(code: u32) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::EXIT)?;
        std::process::exit(code as i32);

        #[allow(unreachable_code)]
        Ok(())
    })
}

/// Write a process id to the given output pointer.
fn write_process_id(binding: BindingDescriptor, out: *mut ProcessId, pid: u32) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(binding)?;

        if out.is_null() {
            return Err(RuntimeError::platform(PlatformError::null_pointer("out")).boxed());
        }

        unsafe {
            *out = ProcessId(pid);
        }

        Ok(())
    })
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
        Err(RuntimeError::platform(PlatformError::not_supported("destack.process.ppid")).boxed())
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
        Err(RuntimeError::platform(PlatformError::not_supported("destack.process.uid")).boxed())
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
        Err(RuntimeError::platform(PlatformError::not_supported("destack.process.gid")).boxed())
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
        Err(RuntimeError::platform(PlatformError::not_supported("destack.process.umask")).boxed())
    }
}

/// Send a signal to the given process.
fn process_kill(pid: u32, signal: u32) -> RuntimeResult<()> {
    // signal the target process on unix platforms
    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(pid as libc::pid_t, signal as libc::c_int) };
        if result < 0 {
            return Err(RuntimeError::platform(PlatformError::io(format!(
                "failed to signal process {pid}"
            )))
            .boxed());
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        Err(RuntimeError::platform(PlatformError::not_supported("destack.process.kill")).boxed())
    }
}
