#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::thread::{bindings_generated as bindings, core as core_thread};
use crate::platform::{NativeStringRef, PlatformError};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::resource;
use crate::platform::thread::ThreadOptions;
/// Detach one host thread.
///
/// Detach one thread from join tracking.
/// Detached thread lifecycle and cleanup are host-managed.
///
/// # Platform
/// Unix, Windows, and Wasi.
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
    context: &RuntimeCallContext,
    _handle: resource::ThreadHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.thread.spawn.detach")).boxed())
}

/// Join one host thread.
///
/// Wait for one joinable thread to exit and return its exit code.
/// Join behavior follows host thread lifecycle rules.
///
/// # Platform
/// Unix, Windows, and Wasi.
/// Uses pthread_join on Unix, WaitForSingleObject plus exit code on Windows, and wasi equivalents where available.
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
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: resource::ThreadHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.thread.spawn.join")).boxed())
}

/// Spawn one host thread.
///
/// Spawn one host thread that enters a runtime-provided entry symbol.
/// Entry dispatch and argument passing are runtime ABI contracts.
///
/// # Platform
/// Unix, Windows, and Wasi.
/// Uses pthread_create on Unix, CreateThread on Windows, and wasi thread support when available.
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
    context: &RuntimeCallContext,
    out: *mut resource::ThreadHandle,
    entry: NativeStringRef,
    argument: u64,
    options: ThreadOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, entry, argument, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.thread.spawn.spawn")).boxed())
}
