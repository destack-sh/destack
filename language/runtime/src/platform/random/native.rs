#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::random::bindings_generated as bindings;
use crate::platform::{NativeSlice, PlatformError};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::random::{
    RandomStream, RandomStreamDomain, RandomStreamState, SecureRandomInfo,
};

/// Stub for destack.random.secure.bytes.
pub unsafe fn destack_random_secure_bytes(
    context: &RuntimeCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(RANDOM_SECURE_BYTES)?;
    let _ = buffer;

    Err(RuntimeError::from(PlatformError::not_supported("destack.random.secure.bytes")).boxed())
}

/// Stub for destack.random.secure.bytesTry.
pub unsafe fn destack_random_secure_bytes_try(
    context: &RuntimeCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(RANDOM_SECURE_BYTES_TRY)?;
    let _ = buffer;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.random.secure.bytesTry",
    ))
    .boxed())
}

/// Stub for destack.random.secure.info.
pub unsafe fn destack_random_secure_info(
    context: &RuntimeCallContext,
    out: *mut SecureRandomInfo,
) -> RuntimeResult<()> {
    context.check_policy(RANDOM_SECURE_INFO)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.random.secure.info")).boxed())
}

/// Stub for destack.random.stream.export.
pub unsafe fn destack_random_stream_export(
    context: &RuntimeCallContext,
    out: *mut RandomStreamState,
    stream: RandomStream,
) -> RuntimeResult<()> {
    context.check_policy(RANDOM_STREAM_EXPORT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, stream);

    Err(RuntimeError::from(PlatformError::not_supported("destack.random.stream.export")).boxed())
}

/// Stub for destack.random.stream.fillBytes.
pub unsafe fn destack_random_fill_bytes(
    context: &RuntimeCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(RANDOM_STREAM_FILL_BYTES)?;
    let _ = buffer;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.random.stream.fillBytes",
    ))
    .boxed())
}

/// Stub for destack.random.stream.fillBytesFrom.
pub unsafe fn destack_random_fill_bytes_from(
    context: &RuntimeCallContext,
    stream: RandomStream,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(RANDOM_STREAM_FILL_BYTES_FROM)?;
    let _ = (stream, buffer);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.random.stream.fillBytesFrom",
    ))
    .boxed())
}

/// Stub for destack.random.stream.import.
pub unsafe fn destack_random_stream_import(
    context: &RuntimeCallContext,
    stream: RandomStream,
    state: RandomStreamState,
) -> RuntimeResult<()> {
    context.check_policy(RANDOM_STREAM_IMPORT)?;
    let _ = (stream, state);

    Err(RuntimeError::from(PlatformError::not_supported("destack.random.stream.import")).boxed())
}

/// Stub for destack.random.stream.in.
pub unsafe fn destack_random_stream_in(
    context: &RuntimeCallContext,
    out: *mut RandomStream,
    domain: RandomStreamDomain,
) -> RuntimeResult<()> {
    context.check_policy(RANDOM_STREAM_IN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, domain);

    Err(RuntimeError::from(PlatformError::not_supported("destack.random.stream.in")).boxed())
}

/// Stub for destack.random.stream.jump.
pub unsafe fn destack_random_stream_jump(
    context: &RuntimeCallContext,
    stream: RandomStream,
    jump: u64,
) -> RuntimeResult<()> {
    context.check_policy(RANDOM_STREAM_JUMP)?;
    let _ = (stream, jump);

    Err(RuntimeError::from(PlatformError::not_supported("destack.random.stream.jump")).boxed())
}

/// Stub for destack.random.stream.nextU64.
pub unsafe fn destack_random_next_u64(
    context: &RuntimeCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    context.check_policy(RANDOM_STREAM_NEXT_U64)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.random.stream.nextU64",
    ))
    .boxed())
}

/// Stub for destack.random.stream.nextU64From.
pub unsafe fn destack_random_next_u64_from(
    context: &RuntimeCallContext,
    out: *mut u64,
    stream: RandomStream,
) -> RuntimeResult<()> {
    context.check_policy(RANDOM_STREAM_NEXT_U64_FROM)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, stream);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.random.stream.nextU64From",
    ))
    .boxed())
}

/// Stub for destack.random.stream.split.
pub unsafe fn destack_random_stream_split(
    context: &RuntimeCallContext,
    out: *mut RandomStream,
    parent: RandomStream,
) -> RuntimeResult<()> {
    context.check_policy(RANDOM_STREAM_SPLIT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, parent);

    Err(RuntimeError::from(PlatformError::not_supported("destack.random.stream.split")).boxed())
}

/// Stub for destack.random.stream.stream.
pub unsafe fn destack_random_stream(
    context: &RuntimeCallContext,
    out: *mut RandomStream,
) -> RuntimeResult<()> {
    context.check_policy(RANDOM_STREAM_STREAM)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported("destack.random.stream.stream")).boxed())
}
