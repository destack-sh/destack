use crate::{TypeId, Word};

use super::{NativeContext, NativeExitCode, NativeRuntimeStatusCode, NativeTrapCode};

/// Cooperate with the runtime at one native safepoint.
pub type NativeSafepoint =
    unsafe extern "C" fn(context: *mut NativeContext, safepoint: u32) -> NativeRuntimeStatusCode;

/// Stop execution for host inspection.
pub type NativeStop =
    unsafe extern "C" fn(context: *mut NativeContext, safepoint: u32) -> NativeExitCode;

/// Deoptimize native execution into interpreter state.
pub type NativeDeopt =
    unsafe extern "C" fn(context: *mut NativeContext, safepoint: u32) -> NativeExitCode;

/// Report one native trap.
pub type NativeTrapExit =
    unsafe extern "C" fn(context: *mut NativeContext, trap: NativeTrapCode) -> NativeExitCode;

/// Report one payloadless language panic.
pub type NativePanic = unsafe extern "C" fn(context: *mut NativeContext) -> NativeExitCode;

/// Copy one typed language panic payload into runtime ownership.
pub type NativePanicValue = unsafe extern "C" fn(
    context: *mut NativeContext,
    ty: TypeId,
    words: *const Word,
) -> NativeExitCode;

/// Continue the active language unwind.
pub type NativeUnwindResume = unsafe extern "C" fn(context: *mut NativeContext) -> NativeExitCode;
