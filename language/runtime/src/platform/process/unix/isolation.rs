#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeStringRef;
use crate::platform::process::core as core_process;
use crate::platform::{NativeArray, PlatformError};

use crate::runtime::BindingCallContext;

use crate::platform::fs;
use crate::platform::fs::core as core_fs;
use crate::platform::process::{
    ProcessId, ProcessNamespaceKind, ProcessUnshareFlags, SyscallFilterFlags,
};

/// Change the root directory for path resolution.
pub(crate) unsafe fn destack_process_chroot(
    _binding: &BindingCallContext,
    path: fs::OsPath,
) -> RuntimeResult<()> {
    let path = core_fs::os_path_to_utf8_string(path, "path")?;
    let path = core_process::cstring_from_str(&path, "path", "path contains nul byte")?;
    let result = unsafe { libc::chroot(path.as_ptr()) };
    if result != 0 {
        return Err(core_process::process_last_error(
            "chroot",
            "failed to change root directory",
        ));
    }

    Ok(())
}

/// Install one syscall filter program.
pub(crate) unsafe fn destack_process_install_syscall_filter(
    _binding: &BindingCallContext,
    program: NativeArray<u8>,
    flags: SyscallFilterFlags,
) -> RuntimeResult<()> {
    let program = unsafe { program.as_slice()? };
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let filter_size = std::mem::size_of::<libc::sock_filter>();
        if program.is_empty() || program.len() % filter_size != 0 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "program",
                "syscall filter program must be a non-empty sock_filter sequence",
            ))
            .boxed());
        }

        let mut filters = Vec::with_capacity(program.len() / filter_size);
        for chunk in program.chunks_exact(filter_size) {
            let filter =
                unsafe { std::ptr::read_unaligned(chunk.as_ptr() as *const libc::sock_filter) };
            filters.push(filter);
        }

        let mut program = libc::sock_fprog {
            len: filters.len() as u16,
            filter: filters.as_mut_ptr(),
        };

        let no_new_privs = unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) };
        if no_new_privs != 0 {
            return Err(core_process::process_last_error(
                "prctl(PR_SET_NO_NEW_PRIVS)",
                "failed to set no_new_privs before seccomp",
            ));
        }

        let result = unsafe {
            libc::syscall(
                libc::SYS_seccomp,
                libc::SECCOMP_SET_MODE_FILTER,
                flags.0 as libc::c_ulong,
                &mut program as *mut libc::sock_fprog,
            )
        };
        if result != 0 {
            return Err(core_process::process_last_error(
                "seccomp",
                "failed to install syscall filter",
            ));
        }

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (program, flags);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.isolation.installSyscallFilter",
        ))
        .boxed())
    }
}

/// Set process host name inside the active UTS namespace.
pub(crate) unsafe fn destack_process_set_host_name(
    _binding: &BindingCallContext,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let name = unsafe { name.as_str()? };
    if name.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "name",
            "host name must not be empty",
        ))
        .boxed());
    }

    let bytes = name.as_bytes();
    let length = i32::try_from(bytes.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "name",
            "host name is too long",
        ))
        .boxed()
    })?;
    let result = unsafe { libc::sethostname(bytes.as_ptr() as *const libc::c_char, length as _) };
    if result != 0 {
        return Err(core_process::process_last_error(
            "sethostname",
            "failed to set host name",
        ));
    }

    Ok(())
}

/// Set network namespace context for subsequent network operations.
pub(crate) unsafe fn destack_process_set_network_namespace(
    _binding: &BindingCallContext,
    path: fs::OsPath,
) -> RuntimeResult<()> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let path = core_fs::os_path_to_utf8_string(path, "path")?;
        process_setns_path(&path, libc::CLONE_NEWNET)
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = path;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.isolation.setNetworkNamespace",
        ))
        .boxed())
    }
}

/// Enter one namespace owned by another process.
pub(crate) unsafe fn destack_process_setns(
    _binding: &BindingCallContext,
    pid: ProcessId,
    namespace: ProcessNamespaceKind,
) -> RuntimeResult<()> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let pid = core_process::process_pid_to_unix_target(pid.0, "pid")? as u32;
        let name = namespace_name(namespace);
        let namespace_path = format!("/proc/{pid}/ns/{name}");
        let namespace_flag = namespace_flag(namespace);
        process_setns_path(&namespace_path, namespace_flag)
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (pid, namespace);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.isolation.setns",
        ))
        .boxed())
    }
}

/// Unshare one or more namespaces.
pub(crate) unsafe fn destack_process_unshare(
    _binding: &BindingCallContext,
    flags: ProcessUnshareFlags,
) -> RuntimeResult<()> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let flags = i32::try_from(flags.0).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "flags",
                "unshare flags exceed supported width",
            ))
            .boxed()
        })?;
        let result = unsafe { libc::unshare(flags) };
        if result != 0 {
            return Err(core_process::process_last_error(
                "unshare",
                "failed to unshare namespaces",
            ));
        }

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = flags;
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.process.isolation.unshare",
        ))
        .boxed())
    }
}

/// Apply setns using one namespace descriptor path.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn process_setns_path(path: &str, namespace_flag: i32) -> RuntimeResult<()> {
    let path = core_process::cstring_from_str(path, "path", "path contains nul byte")?;

    let descriptor = unsafe { libc::open(path.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC) };
    if descriptor < 0 {
        return Err(core_process::process_last_error(
            "open",
            "failed to open namespace descriptor",
        ));
    }

    let setns_result = unsafe { libc::setns(descriptor, namespace_flag) };
    let setns_errno = if setns_result != 0 {
        std::io::Error::last_os_error()
            .raw_os_error()
            .unwrap_or(libc::EINVAL)
    } else {
        0
    };

    let close_result = unsafe { libc::close(descriptor) };
    let close_errno = if close_result != 0 {
        std::io::Error::last_os_error()
            .raw_os_error()
            .unwrap_or(libc::EINVAL)
    } else {
        0
    };

    if setns_errno != 0 {
        return Err(core_process::process_errno_error(
            setns_errno,
            "setns",
            "failed to switch namespace",
        ));
    }
    if close_errno != 0 {
        return Err(core_process::process_errno_error(
            close_errno,
            "close",
            "failed to close namespace descriptor",
        ));
    }

    Ok(())
}

/// Map namespace kind to procfs namespace name.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn namespace_name(namespace: ProcessNamespaceKind) -> &'static str {
    match namespace {
        ProcessNamespaceKind::Mount => "mnt",
        ProcessNamespaceKind::User => "user",
        ProcessNamespaceKind::Pid => "pid",
        ProcessNamespaceKind::Network => "net",
        ProcessNamespaceKind::Ipc => "ipc",
        ProcessNamespaceKind::Uts => "uts",
        ProcessNamespaceKind::Cgroup => "cgroup",
        ProcessNamespaceKind::Time => "time",
    }
}

/// Map namespace kind to setns clone flag.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn namespace_flag(namespace: ProcessNamespaceKind) -> i32 {
    match namespace {
        ProcessNamespaceKind::Mount => libc::CLONE_NEWNS,
        ProcessNamespaceKind::User => libc::CLONE_NEWUSER,
        ProcessNamespaceKind::Pid => libc::CLONE_NEWPID,
        ProcessNamespaceKind::Network => libc::CLONE_NEWNET,
        ProcessNamespaceKind::Ipc => libc::CLONE_NEWIPC,
        ProcessNamespaceKind::Uts => libc::CLONE_NEWUTS,
        ProcessNamespaceKind::Cgroup => libc::CLONE_NEWCGROUP,
        ProcessNamespaceKind::Time => time_namespace_flag(),
    }
}

/// Return the host time-namespace clone flag.
#[cfg(target_os = "linux")]
fn time_namespace_flag() -> i32 {
    libc::CLONE_NEWTIME
}

/// Return the host time-namespace clone flag.
#[cfg(target_os = "android")]
fn time_namespace_flag() -> i32 {
    0x0000_0080
}
