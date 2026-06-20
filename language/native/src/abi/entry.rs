use crate::{NativeContext, NativeContinuation, NativeExitCode, NativeValue};

/// Native entry function.
pub type NativeEntry = unsafe extern "C" fn(
    context: *mut NativeContext,
    args: *const NativeValue,
    arg_count: usize,
    out: *mut NativeValue,
) -> NativeExitCode;

/// Native continuation resume function.
pub type NativeResumeEntry = unsafe extern "C" fn(
    context: *mut NativeContext,
    continuation: NativeContinuation,
    received: NativeValue,
    out: *mut NativeValue,
) -> NativeExitCode;
