use super::{NativeContext, NativeExitCode, NativeValue};

/// Native entry function.
pub type NativeEntry = unsafe extern "C" fn(
    context: *mut NativeContext,
    args: *const NativeValue,
    arg_count: usize,
    out: *mut NativeValue,
) -> NativeExitCode;
