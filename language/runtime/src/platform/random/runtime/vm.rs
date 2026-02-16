use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::random::{RandomStream, RandomStreamDomain, RandomStreamStateVm};
use crate::platform::{PlatformError, VmSlice};
use crate::random::RandomStreamId;
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

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
pub(crate) fn destack_random_stream_export(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    stream: RandomStream,
) -> RuntimeResult<RandomStreamStateVm> {
    let _ = stream;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.random.stream.export is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_random_fill_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    // read the VM buffer into host memory
    let mut bytes = buffer.read_bytes(context)?;

    // fill bytes from the runtime stream for this call context
    let stream_id = runtime.random_stream_id();
    runtime
        .runtime()
        .random
        .fill_stream_bytes(stream_id, &mut bytes);

    // write the filled bytes back into the VM buffer
    buffer.write_bytes(context, &bytes)
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
pub(crate) fn destack_random_fill_bytes_from(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    stream: RandomStream,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    // read the VM buffer into host memory
    let mut bytes = buffer.read_bytes(context)?;

    // fill bytes from the requested runtime stream
    runtime
        .runtime()
        .random
        .fill_stream_bytes(stream_id(stream), &mut bytes);

    // write the filled bytes back into the VM buffer
    buffer.write_bytes(context, &bytes)
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
pub(crate) fn destack_random_stream_import(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    stream: RandomStream,
    state: RandomStreamStateVm,
) -> RuntimeResult<()> {
    let _ = (stream, state);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.random.stream.import is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_random_stream_in(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    domain: RandomStreamDomain,
) -> RuntimeResult<RandomStream> {
    // allocate the stream by domain
    let stream_id = match domain {
        RandomStreamDomain::Process => runtime.runtime().random.new_stream_id(),
        RandomStreamDomain::Task => runtime
            .runtime()
            .random
            .split_stream(runtime.random_stream_id()),
    };

    Ok(stream_handle(stream_id))
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
pub(crate) fn destack_random_stream_jump(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    stream: RandomStream,
    jump: u64,
) -> RuntimeResult<()> {
    // advance the deterministic stream state
    runtime
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
pub(crate) fn destack_random_next_u64(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<u64> {
    let stream_id = runtime.random_stream_id();
    let value = runtime.runtime().random.next_stream_u64(stream_id);

    Ok(value)
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
pub(crate) fn destack_random_next_u64_from(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    stream: RandomStream,
) -> RuntimeResult<u64> {
    let value = runtime.runtime().random.next_stream_u64(stream_id(stream));

    Ok(value)
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
pub(crate) fn destack_random_stream_split(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    parent: RandomStream,
) -> RuntimeResult<RandomStream> {
    let child_stream_id = runtime.runtime().random.split_stream(stream_id(parent));

    Ok(stream_handle(child_stream_id))
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
pub(crate) fn destack_random_stream(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<RandomStream> {
    let stream_id = runtime.runtime().random.new_stream_id();

    Ok(stream_handle(stream_id))
}
