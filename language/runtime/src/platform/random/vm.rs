use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::random::{
    RandomStream, RandomStreamDomain, RandomStreamStateVm, SecureRandomInfoVm, SecureRandomSource,
};
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

/// Fill a slice with secure random bytes.
pub(super) fn destack_random_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = runtime;
    let mut bytes = vec![0u8; buffer.len as usize];
    fill_bytes_secure(&mut bytes)?;
    buffer.write_bytes(context, &bytes)?;
    Ok(())
}

/// Fill a slice with secure random bytes.
pub(super) fn destack_random_secure_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    destack_random_bytes(runtime, context, buffer)
}

/// Fill a slice with secure random bytes without blocking guarantees.
pub(super) fn destack_random_secure_bytes_try(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    destack_random_bytes(runtime, context, buffer)
}

/// Return metadata for the secure random source.
pub(super) fn destack_random_secure_info(
    _runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<SecureRandomInfoVm> {
    // allocate a stable backend label
    let backend_name = vm::StringHandle::new(context.intern_string("getrandom"));
    Ok(SecureRandomInfoVm {
        source: SecureRandomSource::Kernel,
        backend_name,
        may_block: false,
        is_cryptographic: true,
        is_seeded: true,
        is_fips_approved: false,
        entropy_bits_per_byte: 8.0,
    })
}

/// Export one stream state to a versioned byte payload.
pub(super) fn destack_random_stream_export(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    stream: RandomStream,
) -> RuntimeResult<RandomStreamStateVm> {
    let _ = stream;

    // NOTE #Incomplete: implement stream state export
    Err(RuntimeError::from(PlatformError::not_supported("random stream export")).boxed())
}

/// Import one stream state from a versioned byte payload.
pub(super) fn destack_random_stream_import(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    stream: RandomStream,
    state: RandomStreamStateVm,
) -> RuntimeResult<()> {
    let _ = stream;
    let _ = state;

    // NOTE #Incomplete: implement stream state import
    Err(RuntimeError::from(PlatformError::not_supported("random stream import")).boxed())
}

/// Allocate one stream in one runtime domain.
pub(super) fn destack_random_stream_in(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    domain: RandomStreamDomain,
) -> RuntimeResult<RandomStream> {
    let _ = domain;
    Ok(RandomStream(runtime.runtime().random.new_stream_id().get()))
}

/// Jump one stream forward by one deterministic step count.
pub(super) fn destack_random_stream_jump(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    stream: RandomStream,
    jump: u64,
) -> RuntimeResult<()> {
    // consume jump steps to preserve deterministic sequence state
    for _ in 0..jump {
        let _ = runtime
            .runtime()
            .random
            .next_stream_u64(RandomStreamId::new(stream.0));
    }

    Ok(())
}

/// Split one stream into one child stream identifier.
pub(super) fn destack_random_stream_split(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    parent: RandomStream,
) -> RuntimeResult<RandomStream> {
    let _ = parent;
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
