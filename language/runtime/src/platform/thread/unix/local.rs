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
    _context: &RuntimeCallContext,
    out: *mut resource::ThreadLocalKey,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let key = core_thread::thread_local_create()?;
    unsafe {
        *out = key;
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
    _context: &RuntimeCallContext,
    key: resource::ThreadLocalKey,
) -> RuntimeResult<()> {
    core_thread::thread_local_delete(key)
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
    _context: &RuntimeCallContext,
    out: *mut u64,
    key: resource::ThreadLocalKey,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = core_thread::thread_local_get(key)?;
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
    _context: &RuntimeCallContext,
    key: resource::ThreadLocalKey,
    argument_value: u64,
) -> RuntimeResult<()> {
    core_thread::thread_local_set(key, argument_value)
}
