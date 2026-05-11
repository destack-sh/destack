#![allow(clippy::missing_safety_doc)]
use std::sync::atomic::Ordering;

use windows_sys::Win32::Foundation::{CloseHandle, WAIT_FAILED, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{INFINITE, WaitForSingleObject};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::{ThreadEntryHandle, ThreadHandle};
use crate::platform::thread::resource::{ThreadLifecycleState, ThreadResource};
use crate::platform::thread::{ThreadOptions, core as core_thread};
use crate::platform::{PlatformError, core as core_platform};

use crate::runtime::BindingCallContext;
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

    // close the native thread handle to detach
    let rc = unsafe { CloseHandle(resource.native_handle) };
    if rc == 0 {
        core_thread::reset_thread_consume(&resource)?;
        return Err(core_platform::io_error("CloseHandle"));
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

    // wait for thread termination
    let wait_result = unsafe { WaitForSingleObject(resource.native_handle, INFINITE) };
    if wait_result == WAIT_FAILED {
        core_thread::reset_thread_consume(&resource)?;
        return Err(core_platform::io_error("WaitForSingleObject"));
    }
    if wait_result != WAIT_OBJECT_0 {
        core_thread::reset_thread_consume(&resource)?;
        return Err(RuntimeError::from(PlatformError::io_with(
            None,
            None,
            Some(wait_result as i32),
            Some("WaitForSingleObject".to_string()),
            None,
            format!("WaitForSingleObject failed: unexpected wait status {wait_result}"),
        ))
        .boxed());
    }

    // close the thread handle after join
    let close_rc = unsafe { CloseHandle(resource.native_handle) };
    if close_rc == 0 {
        core_thread::reset_thread_consume(&resource)?;
        return Err(core_platform::io_error("CloseHandle"));
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
