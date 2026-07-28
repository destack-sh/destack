use crate::{FunctionId, Word};

use super::{NativeContext, NativeRuntimeStatusCode};

/// Call one runtime binding selected by its Program function.
pub type NativeBindingCall = unsafe extern "C" fn(
    context: *mut NativeContext,
    function: FunctionId,
    arguments: *const Word,
    argument_count: usize,
    result: *mut Word,
    result_count: usize,
) -> NativeRuntimeStatusCode;
