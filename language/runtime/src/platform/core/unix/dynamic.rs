#[cfg(unix)]
use std::ffi::{CStr, CString, c_char, c_void};

/// Open one dynamic library by one file name.
#[cfg(unix)]
pub(crate) fn open_dynamic_library(name: &str) -> Result<*mut c_void, String> {
    let name = CString::new(name)
        .map_err(|_| String::from("dynamic library name must not contain interior NUL bytes"))?;

    // clear any previous loader error before `dlopen`
    unsafe {
        libc::dlerror();
    }

    // open one shared object with eager symbol resolution
    let handle = unsafe { libc::dlopen(name.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
    if handle.is_null() {
        let error = dynamic_loader_error()
            .unwrap_or_else(|| String::from("dlopen failed with no loader diagnostic"));
        return Err(error);
    }

    Ok(handle)
}

/// Close one open dynamic library handle.
#[cfg(unix)]
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
#[cfg(unix)]
pub(crate) fn load_dynamic_symbol<T>(handle: *mut c_void, name: &[u8]) -> Result<T, String>
where
    T: Copy,
{
    if name.last().copied() != Some(0) {
        return Err(String::from(
            "dynamic symbol name must be one NUL-terminated byte string",
        ));
    }

    // clear any previous loader error before `dlsym`
    unsafe {
        libc::dlerror();
    }

    // resolve one symbol by one exact nul-terminated name
    let symbol = unsafe { libc::dlsym(handle, name.as_ptr().cast::<c_char>()) };
    if let Some(error) = dynamic_loader_error() {
        return Err(error);
    }

    if symbol.is_null() {
        return Err(String::from(
            "dlsym returned one null pointer without one loader diagnostic",
        ));
    }

    // cast one raw symbol pointer into one typed function pointer
    union SymbolCast<T: Copy> {
        pointer: *mut c_void,
        symbol: T,
    }
    let value = unsafe { SymbolCast { pointer: symbol }.symbol };

    Ok(value)
}

/// Load one typed symbol from one dynamic-library handle using one UTF-8 name.
#[cfg(target_os = "linux")]
pub(crate) fn load_dynamic_symbol_named<T>(handle: *mut c_void, name: &str) -> Result<T, String>
where
    T: Copy,
{
    let name = CString::new(name)
        .map_err(|_| String::from("dynamic symbol name must not contain interior NUL bytes"))?;

    // clear any previous loader error before `dlsym`
    unsafe {
        libc::dlerror();
    }

    let symbol = unsafe { libc::dlsym(handle, name.as_ptr()) };
    if let Some(error) = dynamic_loader_error() {
        return Err(error);
    }

    if symbol.is_null() {
        return Err(String::from(
            "dlsym returned one null pointer without one loader diagnostic",
        ));
    }

    // cast one raw symbol pointer into one typed function pointer
    union SymbolCast<T: Copy> {
        pointer: *mut c_void,
        symbol: T,
    }
    let value = unsafe { SymbolCast { pointer: symbol }.symbol };

    Ok(value)
}

/// Return one dynamic-loader error message when available.
#[cfg(unix)]
fn dynamic_loader_error() -> Option<String> {
    let pointer = unsafe { libc::dlerror() };
    if pointer.is_null() {
        return None;
    }

    let error = unsafe { CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned();
    if error.is_empty() {
        return None;
    }

    Some(error)
}
