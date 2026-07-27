use crate::{FunctionId, Word};

use super::{NativeContext, NativeRuntimeStatusCode};

/// Call one host binding selected by its Program function.
pub type NativeBindingCall = unsafe extern "C" fn(
    context: *mut NativeContext,
    function: FunctionId,
    arguments: *const Word,
    result: *mut Word,
) -> NativeRuntimeStatusCode;
