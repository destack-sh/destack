use std::mem::MaybeUninit;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::NativeAbiCodec;
use crate::platform::diagnostic::PlatformErrorCode;

/// Return whether one runtime error is an operation-level not-supported error.
pub(super) fn is_not_supported_error(error: &RuntimeError) -> bool {
    error
        .platform_error()
        .is_some_and(|platform| platform.code == PlatformErrorCode::NotSupported)
}

/// Read one optional camera control value.
pub(super) fn optional_control_value<T>(
    read: impl FnOnce(*mut T) -> RuntimeResult<()>,
) -> RuntimeResult<Option<T::Value>>
where
    T: Copy + NativeAbiCodec,
{
    let mut value = MaybeUninit::<T>::uninit();

    match read(value.as_mut_ptr()) {
        Ok(()) => {
            let value = unsafe { value.assume_init() };
            let value = unsafe { T::into_value(value)? };

            Ok(Some(value))
        }
        Err(error) if is_not_supported_error(error.as_ref()) => Ok(None),
        Err(error) => Err(error),
    }
}
