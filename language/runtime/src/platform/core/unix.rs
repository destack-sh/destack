#[cfg(any(target_os = "linux", target_os = "android"))]
use std::ffi::{CString, c_char, c_void};

use crate::diagnostic::RuntimeError;
use crate::platform::PlatformError;

/// Open one dynamic library by one file name.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) fn open_dynamic_library(name: &str) -> Option<*mut c_void> {
    let name = CString::new(name).ok()?;

    // open one shared object with eager symbol resolution
    let handle = unsafe { libc::dlopen(name.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
    if handle.is_null() {
        return None;
    }

    Some(handle)
}

/// Close one open dynamic library handle.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) fn close_dynamic_library(handle: *mut c_void) {
    if handle.is_null() {
        return;
    }

    // close one loaded shared object
    unsafe {
        libc::dlclose(handle);
    }
}

/// Load one typed symbol from one dynamic-library handle.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) fn load_dynamic_symbol<T>(handle: *mut c_void, name: &[u8]) -> Option<T>
where
    T: Copy,
{
    if name.is_empty() || *name.last()? != 0 {
        return None;
    }

    // resolve one symbol by one exact nul-terminated name
    let symbol = unsafe { libc::dlsym(handle, name.as_ptr().cast::<c_char>()) };
    if symbol.is_null() {
        return None;
    }

    // cast one raw symbol pointer into one typed function pointer
    union SymbolCast<T: Copy> {
        pointer: *mut c_void,
        symbol: T,
    }
    let value = unsafe { SymbolCast { pointer: symbol }.symbol };

    Some(value)
}

/// Load one typed symbol from one dynamic-library handle using one UTF-8 name.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) fn load_dynamic_symbol_named<T>(handle: *mut c_void, name: &str) -> Option<T>
where
    T: Copy,
{
    let name = CString::new(name).ok()?;
    let symbol = unsafe { libc::dlsym(handle, name.as_ptr()) };
    if symbol.is_null() {
        return None;
    }

    // cast one raw symbol pointer into one typed function pointer
    union SymbolCast<T: Copy> {
        pointer: *mut c_void,
        symbol: T,
    }
    let value = unsafe { SymbolCast { pointer: symbol }.symbol };

    Some(value)
}

/// Build an I/O runtime error from the last unix errno value.
pub(crate) fn io_error(syscall: &str, path: Option<&str>) -> Box<RuntimeError> {
    let errno = super::get_errno();
    io_error_with_errno(syscall, errno, path)
}

/// Build an I/O runtime error from an explicit unix errno value.
pub(crate) fn io_error_with_errno(
    syscall: &str,
    errno: i32,
    path: Option<&str>,
) -> Box<RuntimeError> {
    let message = format!("{syscall} failed: errno {errno}");
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        Some(errno),
        Some(syscall.to_string()),
        path.map(|path| path.to_string()),
        message,
    ))
    .boxed()
}

/// Build a network runtime error from the last unix errno value.
pub(crate) fn net_error(syscall: &str) -> Box<RuntimeError> {
    let errno = super::get_errno();
    net_error_with_errno(syscall, errno)
}

/// Build a network runtime error from an explicit unix errno value.
pub(crate) fn net_error_with_errno(syscall: &str, errno: i32) -> Box<RuntimeError> {
    let message = format!("{syscall} failed: errno {errno}");
    RuntimeError::from(PlatformError::net_with(
        None,
        None,
        Some(errno),
        Some(syscall.to_string()),
        None,
        None,
        message,
    ))
    .boxed()
}
