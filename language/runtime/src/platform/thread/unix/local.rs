#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::resource::{ResourceKind, ThreadLocalKey};
use crate::platform::thread::{core as core_thread, resource as resource_thread};

use crate::runtime::BindingCallContext;

/// Build one TLS error from a pthread return code.
fn tls_error(syscall: &str, code: libc::c_int) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        Some(code),
        Some(syscall.to_string()),
        None,
        format!("{syscall} failed: errno {code}"),
    ))
    .boxed()
}
/// Create one thread-local key.
pub(crate) unsafe fn destack_thread_local_create(
    binding: &BindingCallContext,
    out: *mut ThreadLocalKey,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // create one native pthread TLS key
    let mut key: libc::pthread_key_t = 0;
    let rc = unsafe { libc::pthread_key_create(&mut key, None) };
    if rc != 0 {
        return Err(tls_error("pthread_key_create", rc));
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

    // release the native pthread TLS key
    let rc = unsafe { libc::pthread_key_delete(resource.key) };
    if rc != 0 {
        return Err(tls_error("pthread_key_delete", rc));
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
    let value = unsafe { libc::pthread_getspecific(resource.key) as u64 };
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
    // resolve the TLS key resource
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
    let value_ptr = value as *mut libc::c_void;
    let rc = unsafe { libc::pthread_setspecific(resource.key, value_ptr) };
    if rc != 0 {
        return Err(tls_error("pthread_setspecific", rc));
    }

    Ok(())
}
