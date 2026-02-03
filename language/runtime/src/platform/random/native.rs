use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::random::RandomStream;
use crate::platform::{NativeSlice, PlatformError, PlatformErrorCode};
use crate::random::RandomStreamId;
use crate::runtime::RuntimeCallContext;

/// Return a deterministic random stream identifier for native code.
pub unsafe fn destack_random_stream(
    context: &RuntimeCallContext,
    out: *mut RandomStream,
) -> RuntimeResult<()> {
    let stream_id = context.runtime().random.new_stream_id();
    unsafe {
        *out = RandomStream(stream_id.get());
    }
    Ok(())
}

/// Return a deterministic random u64 for native code.
pub unsafe fn destack_random_next_u64(
    context: &RuntimeCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    let value = context
        .runtime()
        .random
        .next_stream_u64(context.random_stream_id());
    unsafe {
        *out = value;
    }
    Ok(())
}

/// Return a deterministic random u64 from a stream for native code.
pub unsafe fn destack_random_next_u64_from(
    context: &RuntimeCallContext,
    out: *mut u64,
    stream: RandomStream,
) -> RuntimeResult<()> {
    let value = context
        .runtime()
        .random
        .next_stream_u64(RandomStreamId::new(stream.0));
    unsafe {
        *out = value;
    }
    Ok(())
}

/// Fill a byte slice with deterministic random data for native code.
pub unsafe fn destack_random_fill_bytes(
    context: &RuntimeCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let slice = unsafe { buffer.as_mut_slice()? };
    context
        .runtime()
        .random
        .fill_stream_bytes(context.random_stream_id(), slice);
    Ok(())
}

/// Fill a byte slice with deterministic random data from a stream for native code.
pub unsafe fn destack_random_fill_bytes_from(
    context: &RuntimeCallContext,
    stream: RandomStream,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let slice = unsafe { buffer.as_mut_slice()? };
    context
        .runtime()
        .random
        .fill_stream_bytes(RandomStreamId::new(stream.0), slice);
    Ok(())
}

/// Fill a byte slice with secure random data for native code.
pub unsafe fn destack_random_secure_bytes(
    _context: &RuntimeCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let slice = unsafe { buffer.as_mut_slice()? };
    fill_bytes_secure(slice)?;
    Ok(())
}

/// Fill a buffer with secure random bytes from the OS.
fn fill_bytes_secure(buffer: &mut [u8]) -> RuntimeResult<()> {
    getrandom::fill(buffer).map_err(|error| {
        RuntimeError::from(PlatformError::random(
            Some(PlatformErrorCode::RandomUnavailable),
            format!("secure random failed: {error}"),
        ))
        .boxed()
    })?;

    Ok(())
}
