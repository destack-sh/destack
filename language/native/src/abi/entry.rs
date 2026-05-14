use crate::{NativeContext, NativeStatusCode, NativeValue};

/// Loaded native entry function.
pub type NativeEntry = unsafe extern "C" fn(
    context: *mut NativeContext,
    args: *const NativeValue,
    arg_count: usize,
    out: *mut NativeValue,
) -> NativeStatusCode;
