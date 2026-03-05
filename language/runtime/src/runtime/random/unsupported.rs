use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;

/// Return unsupported for host entropy on unsupported platforms.
pub(crate) fn host_fill_bytes(_buffer: &mut [u8]) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("host entropy")).boxed())
}

/// Return unsupported for nonblocking host entropy on unsupported platforms.
pub(crate) fn host_try_fill_bytes(_buffer: &mut [u8]) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("host entropy")).boxed())
}
