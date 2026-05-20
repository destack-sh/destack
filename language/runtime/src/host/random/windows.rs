use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::HostError;
use windows_sys::Win32::Security::Cryptography::{
    BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom,
};

/// Fill one buffer from the windows host entropy backend.
pub(crate) fn host_fill_bytes(buffer: &mut [u8]) -> RuntimeResult<()> {
    let length = u32::try_from(buffer.len()).map_err(|_| {
        RuntimeError::from(HostError::random(
            None,
            "BCryptGenRandom input length exceeds u32::MAX",
        ))
        .boxed()
    })?;

    // use system preferred rng with null algorithm handle
    // SAFETY: buffer is valid for length bytes and the system RNG mode permits a null handle
    let status = unsafe {
        BCryptGenRandom(
            std::ptr::null_mut(),
            buffer.as_mut_ptr(),
            length,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };
    if status < 0 {
        return Err(RuntimeError::from(HostError::random(
            None,
            format!("BCryptGenRandom failed with NTSTATUS 0x{status:08x}"),
        ))
        .boxed());
    }

    Ok(())
}

/// Try to fill one buffer from the windows host entropy backend without blocking.
pub(crate) fn host_try_fill_bytes(buffer: &mut [u8]) -> RuntimeResult<()> {
    host_fill_bytes(buffer)
}
