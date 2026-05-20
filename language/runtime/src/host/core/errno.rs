/// Return the errno pointer for linux and dragonfly.
#[cfg(any(target_os = "linux", target_os = "dragonfly"))]
pub(crate) fn errno_location() -> *mut libc::c_int {
    // SAFETY: libc returns the thread-local errno storage for the current thread
    unsafe { libc::__errno_location() }
}

/// Return the errno pointer for apple and freebsd targets.
#[cfg(any(target_os = "macos", target_os = "freebsd"))]
pub(crate) fn errno_location() -> *mut libc::c_int {
    // SAFETY: libc returns the thread-local errno storage for the current thread
    unsafe { libc::__error() }
}

/// Return the errno pointer for openbsd and netbsd.
#[cfg(any(target_os = "openbsd", target_os = "netbsd"))]
pub(crate) fn errno_location() -> *mut libc::c_int {
    // SAFETY: libc returns the thread-local errno storage for the current thread
    unsafe { libc::__errno() }
}

/// Read the current errno value.
#[cfg(unix)]
pub(crate) fn get_errno() -> libc::c_int {
    // SAFETY: errno_location returns a valid thread-local pointer on supported unix targets
    unsafe { *errno_location() }
}
