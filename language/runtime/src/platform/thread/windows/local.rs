#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::{ResourceKind, ThreadLocalKey};
use crate::platform::thread::{core as core_thread, resource as resource_thread};
use crate::platform::{PlatformError, core as core_platform};
use windows_sys::Win32::Foundation::{ERROR_SUCCESS, GetLastError, SetLastError};
use windows_sys::Win32::System::Threading::{
    TLS_OUT_OF_INDEXES, TlsAlloc, TlsFree, TlsGetValue, TlsSetValue,
};

use crate::runtime::BindingCallContext;
/// Create one thread-local key.
pub(crate) unsafe fn destack_thread_local_create(
    binding: &BindingCallContext,
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
        binding,
        ResourceKind::ThreadLocal,
        "thread.local",
        resource_thread::ThreadLocalResource { key },
    );
    unsafe {
        *out = ThreadLocalKey(resource_id);
    }

    Ok(())
}

/// Delete one thread-local key.
pub(crate) unsafe fn destack_thread_local_delete(
    binding: &BindingCallContext,
    key: ThreadLocalKey,
) -> RuntimeResult<()> {
    // remove the thread-local key resource
    let resource = core_thread::take_thread_resource::<resource_thread::ThreadLocalResource>(
        binding,
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
pub(crate) unsafe fn destack_thread_local_get(
    binding: &BindingCallContext,
    out: *mut u64,
    key: ThreadLocalKey,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate that the key exists
    let resource = core_thread::resolve_thread_resource::<resource_thread::ThreadLocalResource>(
        binding,
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

    let value = value_ptr as u64;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Store one thread-local value.
pub(crate) unsafe fn destack_thread_local_set(
    binding: &BindingCallContext,
    key: ThreadLocalKey,
    argument_value: u64,
) -> RuntimeResult<()> {
    // validate that the key exists
    let resource = core_thread::resolve_thread_resource::<resource_thread::ThreadLocalResource>(
        binding,
        key.0,
        "key",
        "thread local key",
    )?;

    // write this thread-local value
    let value = usize::try_from(argument_value).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "argument_value",
            "value exceeds host pointer width",
        ))
        .boxed()
    })?;
    let value_ptr = value as *mut std::ffi::c_void;
    let rc = unsafe { TlsSetValue(resource.key, value_ptr) };
    if rc == 0 {
        return Err(core_platform::io_error("TlsSetValue"));
    }

    Ok(())
}
