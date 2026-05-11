#![allow(clippy::missing_safety_doc)]
use std::ptr;
use std::sync::atomic::Ordering;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::resource::{ThreadEntryHandle, ThreadHandle};
use crate::platform::thread::resource::{ThreadLifecycleState, ThreadResource};
use crate::platform::thread::{ThreadOptions, core as core_thread};

use crate::runtime::BindingCallContext;

/// Build one thread detach error from a pthread return code.
fn thread_detach_error(code: libc::c_int) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        Some(code),
        Some("pthread_detach".to_string()),
        None,
        format!("pthread_detach failed: errno {code}"),
    ))
    .boxed()
}

/// Build one thread join error from a pthread return code.
fn thread_join_error(code: libc::c_int) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        Some(code),
        Some("pthread_join".to_string()),
        None,
        format!("pthread_join failed: errno {code}"),
    ))
    .boxed()
}

/// Detach one host thread.
pub(crate) unsafe fn destack_thread_detach(
    binding: &BindingCallContext,
    handle: ThreadHandle,
) -> RuntimeResult<()> {
    // resolve and validate the thread resource
    let resource = core_thread::resolve_thread_resource::<ThreadResource>(
        binding,
        handle.0,
        "handle",
        "thread handle",
    )?;

    // mark the handle as being detached
    core_thread::begin_thread_consume(&resource, ThreadLifecycleState::Detaching)?;

    // detach the native pthread handle
    let rc = unsafe { libc::pthread_detach(resource.native_handle) };
    if rc != 0 {
        core_thread::reset_thread_consume(&resource)?;
        return Err(thread_detach_error(rc));
    }

    // consume the detached handle after host success
    let _ = core_thread::take_thread_resource::<ThreadResource>(
        binding,
        handle.0,
        "handle",
        "thread handle",
    )?;

    Ok(())
}

/// Join one host thread.
pub(crate) unsafe fn destack_thread_join(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: ThreadHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and validate the thread resource
    let resource = core_thread::resolve_thread_resource::<ThreadResource>(
        binding,
        handle.0,
        "handle",
        "thread handle",
    )?;

    // mark the handle as being joined
    core_thread::begin_thread_consume(&resource, ThreadLifecycleState::Joining)?;

    // wait for native thread completion
    let rc = unsafe { libc::pthread_join(resource.native_handle, ptr::null_mut()) };
    if rc != 0 {
        core_thread::reset_thread_consume(&resource)?;
        return Err(thread_join_error(rc));
    }

    // consume the joined handle after host success
    let _ = core_thread::take_thread_resource::<ThreadResource>(
        binding,
        handle.0,
        "handle",
        "thread handle",
    )?;

    // load the published machine-word result
    let exit_code = resource.completion.exit_code.load(Ordering::Acquire);

    // write the machine-word result
    unsafe {
        *out = exit_code;
    }

    Ok(())
}

/// Spawn one host thread.
pub(crate) unsafe fn destack_thread_spawn(
    _binding: &BindingCallContext,
    _out: *mut ThreadHandle,
    _entry: ThreadEntryHandle,
    _argument: u64,
    _options: ThreadOptions,
) -> RuntimeResult<()> {
    Err(core_thread::thread_spawn_unavailable_error())
}
