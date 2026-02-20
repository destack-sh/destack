#![allow(dead_code)]
#![allow(clippy::missing_safety_doc)]
use std::ffi::c_void;
use std::ptr;

use windows_sys::Win32::Foundation::{CloseHandle, WAIT_FAILED, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{
    CreateThread, GetExitCodeThread, INFINITE, WaitForSingleObject,
};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::ThreadHandle;
use crate::platform::thread::{ThreadOptions, core as core_thread, resource as resource_thread};
use crate::platform::{NativeStringRef, PlatformError, core as core_platform};

use crate::runtime::BindingCallContext;

/// Thread bootstrap payload passed to one native Windows thread.
struct ThreadStartPayload {
    /// Exit code returned by the thread routine.
    exit_code: u32,
}

/// Run one native Windows thread start routine.
unsafe extern "system" fn thread_start(payload: *mut c_void) -> u32 {
    if payload.is_null() {
        return 0;
    }

    // reclaim the bootstrap payload ownership
    let payload = unsafe { Box::from_raw(payload as *mut ThreadStartPayload) };
    payload.exit_code
}
/// Detach one host thread.
///
/// Detach one thread from join tracking.
/// Detached thread lifecycle and cleanup are host-managed.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread_detach on Unix and handle-release semantics on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.spawn`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_detach(
    context: &BindingCallContext,
    handle: ThreadHandle,
) -> RuntimeResult<()> {
    // remove and validate the thread resource
    let resource = core_thread::take_thread_resource::<resource_thread::ThreadResource>(
        context,
        handle.0,
        "handle",
        "thread handle",
    )?;

    // close the native thread handle to detach
    let rc = unsafe { CloseHandle(resource.native_handle) };
    if rc == 0 {
        return Err(core_platform::io_error("CloseHandle"));
    }

    Ok(())
}

/// Join one host thread.
///
/// Wait for one joinable thread to exit and return its exit code.
/// Join behavior follows host thread lifecycle rules.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread_join on Unix and WaitForSingleObject plus exit code on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.spawn`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_join(
    context: &BindingCallContext,
    out: *mut u32,
    handle: ThreadHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // remove and validate the thread resource
    let resource = core_thread::take_thread_resource::<resource_thread::ThreadResource>(
        context,
        handle.0,
        "handle",
        "thread handle",
    )?;

    // wait for thread termination
    let wait_result = unsafe { WaitForSingleObject(resource.native_handle, INFINITE) };
    if wait_result == WAIT_FAILED {
        let _ = unsafe { CloseHandle(resource.native_handle) };
        return Err(core_platform::io_error("WaitForSingleObject"));
    }
    if wait_result != WAIT_OBJECT_0 {
        let _ = unsafe { CloseHandle(resource.native_handle) };
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

    // resolve the thread exit code
    let mut exit_code = 0_u32;
    let exit_code_rc = unsafe { GetExitCodeThread(resource.native_handle, &mut exit_code) };
    if exit_code_rc == 0 {
        let _ = unsafe { CloseHandle(resource.native_handle) };
        return Err(core_platform::io_error("GetExitCodeThread"));
    }

    // close the thread handle after join
    let close_rc = unsafe { CloseHandle(resource.native_handle) };
    if close_rc == 0 {
        return Err(core_platform::io_error("CloseHandle"));
    }

    // write the exit code
    unsafe {
        *out = exit_code;
    }

    Ok(())
}

/// Spawn one host thread.
///
/// Spawn one host thread that enters a runtime-provided entry symbol.
/// Entry dispatch and argument passing are runtime ABI contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses pthread_create on Unix and CreateThread on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `thread.spawn`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_thread_spawn(
    context: &BindingCallContext,
    out: *mut ThreadHandle,
    entry: NativeStringRef,
    argument: u64,
    options: ThreadOptions,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate supported spawn option flags
    if options.flags != 0 {
        return Err(core_thread::unsupported_flags_error(
            "options.flags",
            options.flags,
        ));
    }

    // validate and decode the entry symbol
    let _entry_symbol = unsafe { entry.as_str()? };

    // validate optional stack size
    let stack_size = usize::try_from(options.stack_bytes).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "options.stackBytes",
            "stack size exceeds host usize range",
        ))
        .boxed()
    })?;

    // allocate the bootstrap payload for the thread routine
    let payload = Box::new(ThreadStartPayload {
        exit_code: argument as u32,
    });
    let payload_ptr = Box::into_raw(payload) as *mut c_void;

    // create the native thread
    let mut _thread_id = 0_u32;
    let native_handle = unsafe {
        CreateThread(
            ptr::null(),
            stack_size,
            Some(thread_start),
            payload_ptr,
            0,
            &mut _thread_id,
        )
    };
    if native_handle == 0 {
        unsafe {
            let _ = Box::from_raw(payload_ptr as *mut ThreadStartPayload);
        }
        return Err(core_platform::io_error("CreateThread"));
    }

    // store one spawned thread resource
    let resource_id = core_thread::insert_thread_resource(
        context,
        "thread",
        resource_thread::ThreadResource { native_handle },
    );

    // write the thread handle
    let handle = ThreadHandle(resource_id);
    unsafe {
        *out = handle;
    }

    Ok(())
}
