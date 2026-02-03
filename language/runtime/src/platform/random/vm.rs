use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::random::RandomStream;
use crate::platform::{PlatformError, PlatformErrorCode, VmSlice};
use crate::random::RandomStreamId;
use crate::runtime::RuntimeCallContext;

/// Stub for destack.random.stream.
pub(super) fn destack_random_stream(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<RandomStream> {
    Ok(RandomStream(runtime.runtime().random.new_stream_id().get()))
}

/// Stub for destack.random.fillBytes.
pub(super) fn destack_random_fill_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    let mut bytes = vec![0u8; buffer.len as usize];
    runtime
        .runtime()
        .random
        .fill_stream_bytes(runtime.random_stream_id(), &mut bytes);
    buffer.write_bytes(context, &bytes)?;
    Ok(())
}

/// Stub for destack.random.nextU64.
pub(super) fn destack_random_next_u64(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<u64> {
    Ok(runtime
        .runtime()
        .random
        .next_stream_u64(runtime.random_stream_id()))
}

/// Stub for destack.random.nextU64From.
pub(super) fn destack_random_next_u64_from(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    stream: RandomStream,
) -> RuntimeResult<u64> {
    Ok(runtime
        .runtime()
        .random
        .next_stream_u64(RandomStreamId::new(stream.0)))
}

/// Stub for destack.random.secureBytes.
pub(super) fn destack_random_secure_bytes(
    _runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    let mut bytes = vec![0u8; buffer.len as usize];
    fill_bytes_secure(&mut bytes)?;
    buffer.write_bytes(context, &bytes)?;
    Ok(())
}

/// Stub for destack.random.fillBytesFrom.
pub(super) fn destack_random_fill_bytes_from(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    stream: RandomStream,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    let mut bytes = vec![0u8; buffer.len as usize];
    runtime
        .runtime()
        .random
        .fill_stream_bytes(RandomStreamId::new(stream.0), &mut bytes);
    buffer.write_bytes(context, &bytes)?;
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
