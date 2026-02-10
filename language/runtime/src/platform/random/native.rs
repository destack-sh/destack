use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::random::{
    RandomStream, RandomStreamDomain, RandomStreamState, SecureRandomInfo, SecureRandomSource,
};
use crate::platform::{NativeSlice, NativeStringRef, PlatformError, PlatformErrorCode};
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

/// Fill a byte slice with secure random data for native code.
pub unsafe fn destack_random_bytes(
    context: &RuntimeCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = context;
    let slice = unsafe { buffer.as_mut_slice()? };
    fill_bytes_secure(slice)?;
    Ok(())
}

/// Fill a byte slice with secure random data without blocking guarantees.
pub unsafe fn destack_random_secure_bytes_try(
    context: &RuntimeCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = context;
    let slice = unsafe { buffer.as_mut_slice()? };
    fill_bytes_secure(slice)?;
    Ok(())
}

/// Return metadata for the secure random source.
pub unsafe fn destack_random_secure_info(
    context: &RuntimeCallContext,
    out: *mut SecureRandomInfo,
) -> RuntimeResult<()> {
    let _ = context;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // set a stable backend description
    unsafe {
        *out = SecureRandomInfo {
            source: SecureRandomSource::Kernel,
            backend_name: NativeStringRef::from("getrandom"),
            may_block: false,
            is_cryptographic: true,
            is_seeded: true,
            is_fips_approved: false,
            entropy_bits_per_byte: 8.0,
        };
    }

    Ok(())
}

/// Export one stream state as a versioned payload.
pub unsafe fn destack_random_stream_export(
    context: &RuntimeCallContext,
    out: *mut RandomStreamState,
    stream: RandomStream,
) -> RuntimeResult<()> {
    let _ = context;
    let _ = stream;
    let _ = out;

    // NOTE #Incomplete: implement stream state export
    Err(RuntimeError::from(PlatformError::not_supported("random stream export")).boxed())
}

/// Import one stream state from a versioned payload.
pub unsafe fn destack_random_stream_import(
    context: &RuntimeCallContext,
    stream: RandomStream,
    state: RandomStreamState,
) -> RuntimeResult<()> {
    let _ = context;
    let _ = stream;
    let _ = state;

    // NOTE #Incomplete: implement stream state import
    Err(RuntimeError::from(PlatformError::not_supported("random stream import")).boxed())
}

/// Allocate one stream in the requested runtime domain.
pub unsafe fn destack_random_stream_in(
    context: &RuntimeCallContext,
    out: *mut RandomStream,
    domain: RandomStreamDomain,
) -> RuntimeResult<()> {
    let _ = domain;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // allocate a stream id using runtime state
    let stream_id = context.runtime().random.new_stream_id();
    unsafe {
        *out = RandomStream(stream_id.get());
    }

    Ok(())
}

/// Jump one stream forward by a deterministic step count.
pub unsafe fn destack_random_stream_jump(
    context: &RuntimeCallContext,
    stream: RandomStream,
    jump: u64,
) -> RuntimeResult<()> {
    // consume jump steps to preserve deterministic sequence state
    for _ in 0..jump {
        let _ = context
            .runtime()
            .random
            .next_stream_u64(RandomStreamId::new(stream.0));
    }

    Ok(())
}

/// Split one stream into one child stream identifier.
pub unsafe fn destack_random_stream_split(
    context: &RuntimeCallContext,
    out: *mut RandomStream,
    parent: RandomStream,
) -> RuntimeResult<()> {
    let _ = parent;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // allocate a child stream id in runtime state
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

/// Fill a byte slice with secure random data without blocking guarantees.
pub unsafe fn destack_random_bytes_try(
    context: &RuntimeCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { destack_random_secure_bytes_try(context, buffer) }
}

/// Return metadata for the secure random source.
pub unsafe fn destack_random_info(
    context: &RuntimeCallContext,
    out: *mut SecureRandomInfo,
) -> RuntimeResult<()> {
    unsafe { destack_random_secure_info(context, out) }
}

/// Export one stream state as a versioned payload.
pub unsafe fn destack_random_export(
    context: &RuntimeCallContext,
    out: *mut RandomStreamState,
    stream: RandomStream,
) -> RuntimeResult<()> {
    unsafe { destack_random_stream_export(context, out, stream) }
}

/// Import one stream state from a versioned payload.
pub unsafe fn destack_random_import(
    context: &RuntimeCallContext,
    stream: RandomStream,
    state: RandomStreamState,
) -> RuntimeResult<()> {
    unsafe { destack_random_stream_import(context, stream, state) }
}

/// Allocate one stream in the requested runtime domain.
pub unsafe fn destack_random_in(
    context: &RuntimeCallContext,
    out: *mut RandomStream,
    domain: RandomStreamDomain,
) -> RuntimeResult<()> {
    unsafe { destack_random_stream_in(context, out, domain) }
}

/// Jump one stream forward by a deterministic step count.
pub unsafe fn destack_random_jump(
    context: &RuntimeCallContext,
    stream: RandomStream,
    jump: u64,
) -> RuntimeResult<()> {
    unsafe { destack_random_stream_jump(context, stream, jump) }
}

/// Split one stream into one child stream identifier.
pub unsafe fn destack_random_split(
    context: &RuntimeCallContext,
    out: *mut RandomStream,
    parent: RandomStream,
) -> RuntimeResult<()> {
    unsafe { destack_random_stream_split(context, out, parent) }
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
