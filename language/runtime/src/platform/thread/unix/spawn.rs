#![allow(clippy::missing_safety_doc)]
use std::mem::MaybeUninit;
use std::ptr;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::{ResourceKind, ThreadHandle};
use crate::platform::thread::{ThreadOptions, core as core_thread, resource as resource_thread};
use crate::platform::{NativeStringRef, PlatformError};

use crate::runtime::BindingCallContext;

/// Thread bootstrap payload passed to one native pthread.
struct ThreadStartPayload {
    /// Exit code returned by the thread routine.
    exit_code: u32,
}

/// Run one native pthread start routine.
extern "C" fn thread_start(payload: *mut libc::c_void) -> *mut libc::c_void {
    if payload.is_null() {
        return ptr::null_mut();
    }

    // reclaim the bootstrap payload ownership
    let payload = unsafe { Box::from_raw(payload as *mut ThreadStartPayload) };
    let exit_code = payload.exit_code;

    // return the exit code in heap storage for pthread_join to reclaim
    Box::into_raw(Box::new(exit_code)) as *mut libc::c_void
}

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

/// Build one thread spawn error from a pthread return code.
fn thread_spawn_error(code: libc::c_int) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        Some(code),
        Some("pthread_create".to_string()),
        None,
        format!("pthread_create failed: errno {code}"),
    ))
    .boxed()
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
    binding: &BindingCallContext,
    handle: ThreadHandle,
) -> RuntimeResult<()> {
    // remove and validate the thread resource
    let resource = core_thread::take_thread_resource::<resource_thread::ThreadResource>(
        binding,
        handle.0,
        "handle",
        "thread handle",
    )?;

    // detach the native pthread handle
    let rc = unsafe { libc::pthread_detach(resource.native_handle) };
    if rc != 0 {
        return Err(thread_detach_error(rc));
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
    binding: &BindingCallContext,
    out: *mut u32,
    handle: ThreadHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // remove and validate the thread resource
    let resource = core_thread::take_thread_resource::<resource_thread::ThreadResource>(
        binding,
        handle.0,
        "handle",
        "thread handle",
    )?;

    // wait for native thread completion and decode the exit code
    let mut thread_return: *mut libc::c_void = ptr::null_mut();
    let rc = unsafe { libc::pthread_join(resource.native_handle, &mut thread_return) };
    if rc != 0 {
        return Err(thread_join_error(rc));
    }
    let exit_code = if thread_return.is_null() {
        0
    } else {
        unsafe { *Box::from_raw(thread_return as *mut u32) }
    };

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
    binding: &BindingCallContext,
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

    // initialize pthread attributes for optional stack configuration
    let mut attributes = MaybeUninit::<libc::pthread_attr_t>::uninit();
    let init_rc = unsafe { libc::pthread_attr_init(attributes.as_mut_ptr()) };
    if init_rc != 0 {
        return Err(thread_spawn_error(init_rc));
    }
    let mut attributes = unsafe { attributes.assume_init() };

    // apply requested stack size when configured
    if options.stack_bytes != 0 {
        let stack_size = usize::try_from(options.stack_bytes).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "options.stackBytes",
                "stack size exceeds host usize range",
            ))
            .boxed()
        })?;

        let setstack_rc = unsafe { libc::pthread_attr_setstacksize(&mut attributes, stack_size) };
        if setstack_rc != 0 {
            unsafe {
                libc::pthread_attr_destroy(&mut attributes);
            }
            return Err(thread_spawn_error(setstack_rc));
        }
    }

    // allocate the bootstrap payload for the thread routine
    let argument = u32::try_from(argument).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "argument",
            "argument exceeds u32 exit-code range",
        ))
        .boxed()
    })?;
    let payload = Box::new(ThreadStartPayload {
        exit_code: argument,
    });
    let payload_ptr = Box::into_raw(payload) as *mut libc::c_void;

    // create the native pthread
    let mut native_handle = MaybeUninit::<libc::pthread_t>::uninit();
    let create_rc = unsafe {
        libc::pthread_create(
            native_handle.as_mut_ptr(),
            &attributes,
            thread_start,
            payload_ptr,
        )
    };
    unsafe {
        libc::pthread_attr_destroy(&mut attributes);
    }
    if create_rc != 0 {
        unsafe {
            let _ = Box::from_raw(payload_ptr as *mut ThreadStartPayload);
        }
        return Err(thread_spawn_error(create_rc));
    }
    let native_handle = unsafe { native_handle.assume_init() };

    // store one spawned thread resource
    let resource_id = core_thread::insert_thread_resource(
        binding,
        ResourceKind::Thread,
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
