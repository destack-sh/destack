#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::abi::NativeStringSlice;

use crate::runtime::BindingCallContext;

/// Return the process argument vector.
pub(crate) unsafe fn destack_process_args(
    binding: &BindingCallContext,
    out: *mut NativeStringSlice,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // collect argument strings into call-local storage
    let args = binding.process_args();
    let mut values = Vec::with_capacity(args.len());
    for argument in args {
        values.push(binding.store_string(argument));
    }

    // write the encoded string slice
    unsafe {
        *out = binding.store_string_slice(values);
    }

    Ok(())
}
