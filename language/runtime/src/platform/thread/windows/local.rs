#![allow(dead_code)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::ThreadLocalKey;
use crate::platform::thread::{core as core_thread, resource as resource_thread};
use crate::platform::{PlatformError, core as core_platform};
use windows_sys::Win32::Foundation::{ERROR_SUCCESS, GetLastError, SetLastError};
use windows_sys::Win32::System::Threading::{
    TLS_OUT_OF_INDEXES, TlsAlloc, TlsFree, TlsGetValue, TlsSetValue,
};

use crate::runtime::RuntimeCallContext;
/// Create one thread-local key.
///
/// Allocate one runtime thread-local storage key.
/// Key lifetime is explicit and must be released with delete.
///
/// # Platform
/// Unix, Windows, and Wasi.
/// Uses pthread TLS keys on Unix, TlsAlloc on Windows, and wasi TLS support where available.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.local`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_local_create(
    context: &RuntimeCallContext,
    out: *mut ThreadLocalKey,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // create one native windows TLS key
    let key = unsafe { TlsAlloc() };
    if key == TLS_OUT_OF_INDEXES {
        return Err(core_platform::io_error("TlsAlloc"));
    }

    // allocate one thread-local key resource
    let resource_id = core_thread::insert_thread_resource(
        context,
        "thread.local",
        resource_thread::ThreadLocalResource { key },
    );
    unsafe {
        *out = ThreadLocalKey(resource_id);
    }

    Ok(())
}

/// Delete one thread-local key.
///
/// Release one thread-local key and associated host resources.
/// Existing per-thread values become invalid after deletion.
///
/// # Platform
/// Unix, Windows, and Wasi.
/// Uses pthread TLS key deletion on Unix, TlsFree on Windows, and wasi TLS support where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.local`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_local_delete(
    context: &RuntimeCallContext,
    key: ThreadLocalKey,
) -> RuntimeResult<()> {
    // remove the thread-local key resource
    let resource = core_thread::take_thread_resource::<resource_thread::ThreadLocalResource>(
        context,
        key.0,
        "key",
        "thread local key",
    )?;

    // release the native windows TLS key
    let rc = unsafe { TlsFree(resource.key) };
    if rc == 0 {
        return Err(core_platform::io_error("TlsFree"));
    }

    Ok(())
}

/// Read one thread-local value.
///
/// Read one machine-word value from one thread-local key.
/// Value interpretation is caller-defined and ABI-dependent.
///
/// # Platform
/// Unix, Windows, and Wasi.
/// Uses pthread TLS storage on Unix, TlsGetValue on Windows, and wasi TLS support where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.local`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_local_get(
    context: &RuntimeCallContext,
    out: *mut u64,
    key: ThreadLocalKey,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate that the key exists
    let resource = core_thread::resolve_thread_resource::<resource_thread::ThreadLocalResource>(
        context,
        key.0,
        "key",
        "thread local key",
    )?;

    // read this thread-local value
    unsafe { SetLastError(ERROR_SUCCESS) };
    let value_ptr = unsafe { TlsGetValue(resource.key) };
    let error_code = unsafe { GetLastError() };
    if value_ptr.is_null() && error_code != ERROR_SUCCESS {
        return Err(core_platform::io_error_with_code(
            "TlsGetValue",
            error_code as i32,
        ));
    }

    let value = value_ptr as usize as u64;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Store one thread-local value.
///
/// Write one machine-word value into one thread-local key.
/// Value interpretation is caller-defined and ABI-dependent.
///
/// # Platform
/// Unix, Windows, and Wasi.
/// Uses pthread TLS storage on Unix, TlsSetValue on Windows, and wasi TLS support where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.local`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_local_set(
    context: &RuntimeCallContext,
    key: ThreadLocalKey,
    argument_value: u64,
) -> RuntimeResult<()> {
    // validate that the key exists
    let resource = core_thread::resolve_thread_resource::<resource_thread::ThreadLocalResource>(
        context,
        key.0,
        "key",
        "thread local key",
    )?;

    // write this thread-local value
    let value_ptr = argument_value as usize as *mut std::ffi::c_void;
    let rc = unsafe { TlsSetValue(resource.key, value_ptr) };
    if rc == 0 {
        return Err(core_platform::io_error("TlsSetValue"));
    }

    Ok(())
}
