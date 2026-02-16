use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::random::{RandomStream, RandomStreamDomain, RandomStreamState};
use crate::platform::{NativeSlice, PlatformError};
use crate::random::RandomStreamId;
use crate::runtime::RuntimeCallContext;

/// Convert a platform stream handle into a runtime stream id.
fn stream_id(stream: RandomStream) -> RandomStreamId {
    RandomStreamId::new(stream.0)
}

/// Convert a runtime stream id into a platform stream handle.
fn stream_handle(stream_id: RandomStreamId) -> RandomStream {
    RandomStream(stream_id.get())
}

/// Export deterministic stream state.
///
/// Serialize one stream state into a versioned byte payload for checkpointing.
/// State bytes are opaque to callers and validated on restore.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses runtime deterministic PRNG state serialization.
///
/// # Errors
/// Returns randomUnavailable, invalidArgument, notSupported.
///
/// # Security
/// Requires `random.deterministic`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_random_stream_export(
    _context: &RuntimeCallContext,
    out: *mut RandomStreamState,
    stream: RandomStream,
) -> RuntimeResult<()> {
    let _ = (out, stream);

    Err(RuntimeError::from(PlatformError::not_supported("destack.random.stream.export")).boxed())
}

/// Fill a slice with deterministic random bytes from the default stream.
///
/// Advance the default stream to fill one mutable buffer.
/// Buffer fill order is deterministic for a fixed stream state.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses runtime deterministic PRNG state.
///
/// # Errors
/// Returns randomUnavailable, notSupported.
///
/// # Security
/// Requires `random.deterministic`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_random_fill_bytes(
    context: &RuntimeCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // resolve one mutable native slice
    let bytes = unsafe { buffer.as_mut_slice()? };

    // fill bytes from the runtime stream for this call context
    let stream_id = context.random_stream_id();
    context.runtime().random.fill_stream_bytes(stream_id, bytes);

    Ok(())
}

/// Fill a slice with deterministic random bytes from a specific stream.
///
/// Advance the selected stream to fill one mutable buffer.
/// Buffer fill order is deterministic for a fixed stream state.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses runtime deterministic PRNG state.
///
/// # Errors
/// Returns randomUnavailable, notSupported.
///
/// # Security
/// Requires `random.deterministic`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_random_fill_bytes_from(
    context: &RuntimeCallContext,
    stream: RandomStream,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // resolve one mutable native slice
    let bytes = unsafe { buffer.as_mut_slice()? };

    // fill bytes from the requested runtime stream
    context
        .runtime()
        .random
        .fill_stream_bytes(stream_id(stream), bytes);

    Ok(())
}

/// Import deterministic stream state.
///
/// Restore one stream state from a versioned byte payload.
/// Restored state replaces the prior stream state atomically.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses runtime deterministic PRNG state deserialization.
///
/// # Errors
/// Returns randomUnavailable, invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `random.deterministic`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_random_stream_import(
    _context: &RuntimeCallContext,
    stream: RandomStream,
    state: RandomStreamState,
) -> RuntimeResult<()> {
    let _ = (stream, state);

    Err(RuntimeError::from(PlatformError::not_supported("destack.random.stream.import")).boxed())
}

/// Allocate a deterministic random stream in one domain.
///
/// Create one runtime-managed deterministic PRNG stream in the requested domain.
/// Domain assignment controls stream inheritance and replay grouping behavior.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses runtime deterministic PRNG state.
///
/// # Errors
/// Returns randomUnavailable, invalidArgument, notSupported.
///
/// # Security
/// Requires `random.deterministic`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_random_stream_in(
    context: &RuntimeCallContext,
    out: *mut RandomStream,
    domain: RandomStreamDomain,
) -> RuntimeResult<()> {
    // validate out pointer for native ABI writes
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // allocate the stream by domain
    let stream_id = match domain {
        RandomStreamDomain::Process => context.runtime().random.new_stream_id(),
        RandomStreamDomain::Task => context
            .runtime()
            .random
            .split_stream(context.random_stream_id()),
    };

    // write the stream handle to the ABI out pointer
    unsafe {
        std::ptr::write(out, stream_handle(stream_id));
    }

    Ok(())
}

/// Advance a deterministic stream by one jump count.
///
/// Move one stream forward by a deterministic jump count without generating intermediate values.
/// Jump semantics are runtime-defined and versioned for replay compatibility.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses runtime deterministic PRNG jump logic.
///
/// # Errors
/// Returns randomUnavailable, invalidArgument, notSupported.
///
/// # Security
/// Requires `random.deterministic`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_random_stream_jump(
    context: &RuntimeCallContext,
    stream: RandomStream,
    jump: u64,
) -> RuntimeResult<()> {
    // advance the deterministic stream state
    context
        .runtime()
        .random
        .jump_stream(stream_id(stream), jump);

    Ok(())
}

/// Return a deterministic random uint64 from the default stream.
///
/// Advance the default deterministic PRNG stream and return one value.
/// Stream step behavior is stable for replay within one runtime version.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses runtime deterministic PRNG state.
///
/// # Errors
/// Returns randomUnavailable, notSupported.
///
/// # Security
/// Requires `random.deterministic`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_random_next_u64(
    context: &RuntimeCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    // validate out pointer for native ABI writes
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // sample one value from the current runtime stream
    let value = context
        .runtime()
        .random
        .next_stream_u64(context.random_stream_id());

    // write the sampled value to the ABI out pointer
    unsafe {
        std::ptr::write(out, value);
    }

    Ok(())
}

/// Return a deterministic random uint64 from a specific stream.
///
/// Advance the selected deterministic PRNG stream and return one value.
/// Stream step behavior is stable for replay within one runtime version.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses runtime deterministic PRNG state.
///
/// # Errors
/// Returns randomUnavailable, notSupported.
///
/// # Security
/// Requires `random.deterministic`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_random_next_u64_from(
    context: &RuntimeCallContext,
    out: *mut u64,
    stream: RandomStream,
) -> RuntimeResult<()> {
    // validate out pointer for native ABI writes
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // sample one value from the requested runtime stream
    let value = context.runtime().random.next_stream_u64(stream_id(stream));

    // write the sampled value to the ABI out pointer
    unsafe {
        std::ptr::write(out, value);
    }

    Ok(())
}

/// Split a deterministic stream into one child stream.
///
/// Derive one child stream from one parent stream using deterministic split semantics.
/// Parent and child sequences remain stable for replay in one runtime version.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses runtime deterministic PRNG split logic.
///
/// # Errors
/// Returns randomUnavailable, invalidArgument, notSupported.
///
/// # Security
/// Requires `random.deterministic`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_random_stream_split(
    context: &RuntimeCallContext,
    out: *mut RandomStream,
    parent: RandomStream,
) -> RuntimeResult<()> {
    // validate out pointer for native ABI writes
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // derive one child stream from the parent stream
    let child_stream = context.runtime().random.split_stream(stream_id(parent));

    // write the child stream handle to the ABI out pointer
    unsafe {
        std::ptr::write(out, stream_handle(child_stream));
    }

    Ok(())
}

/// Allocate a deterministic random stream identifier.
///
/// Create one runtime-managed deterministic PRNG stream.
/// Stream seeding follows runtime determinism and replay policy.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses runtime deterministic PRNG state.
///
/// # Errors
/// Returns randomUnavailable, notSupported.
///
/// # Security
/// Requires `random.deterministic`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_random_stream(
    context: &RuntimeCallContext,
    out: *mut RandomStream,
) -> RuntimeResult<()> {
    // validate out pointer for native ABI writes
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // allocate one process-scoped stream id
    let stream_id = context.runtime().random.new_stream_id();

    // write the stream handle to the ABI out pointer
    unsafe {
        std::ptr::write(out, stream_handle(stream_id));
    }

    Ok(())
}
