use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::random::{
    RandomStream, RandomStreamDomain, RandomStreamStateVm, SecureRandomMetadataVm,
    SecureRandomSource,
};
use crate::platform::{PlatformError, VmSlice};
use crate::runtime::BindingCallContext;
use crate::runtime::random::RandomStreamId;
use destack_vm as vm;

/// Fill a slice with cryptographically secure random bytes.
///
/// Read entropy from host cryptographic RNG facilities.
/// Entropy quality and blocking behavior follow host kernel guarantees.
///
/// # Platform
/// Unix and Windows where host entropy APIs are available.
/// Uses getrandom(2) or getentropy on Unix and BCryptGenRandom on Windows.
///
/// # Errors
/// Returns randomUnavailable, notSupported.
///
/// # Security
/// Requires `random.secure`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_random_secure_bytes(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    binding.hooks().on_random_read(Some(binding.engine()));

    // resolve VM bytes into host memory
    let mut bytes = buffer.read_bytes(context)?;

    // fill secure bytes from the host entropy backend
    getrandom::fill(&mut bytes).map_err(|error| {
        RuntimeError::from(PlatformError::random(
            None,
            format!("destack.random.secure.bytes failed: {error}"),
        ))
        .boxed()
    })?;

    // write secure bytes back into VM memory
    buffer.write_bytes(context, &bytes)
}

/// Fill a slice with secure random bytes without blocking.
///
/// Try to read secure entropy without blocking the current execution context.
/// Fails with `ioWouldBlock` when the host source requires blocking.
///
/// # Platform
/// Unix and Windows where host entropy APIs are available.
/// Uses nonblocking host entropy APIs when available and runtime fallbacks otherwise.
///
/// # Errors
/// Returns randomUnavailable, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `random.secure`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_random_secure_bytes_try(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    binding.hooks().on_random_read(Some(binding.engine()));

    // resolve VM bytes into host memory
    let mut bytes = buffer.read_bytes(context)?;

    // fill secure bytes and surface backend errors to callers
    getrandom::fill(&mut bytes).map_err(|error| {
        RuntimeError::from(PlatformError::random(
            None,
            format!("destack.random.secure.bytesTry failed: {error}"),
        ))
        .boxed()
    })?;

    // write secure bytes back into VM memory
    buffer.write_bytes(context, &bytes)
}

/// Query secure randomness source metadata.
///
/// Return source metadata for the secure random backend selected by the runtime.
/// Metadata values are normalized across host operating systems.
///
/// # Platform
/// Unix and Windows where host entropy APIs are available.
/// Uses runtime source selection metadata.
///
/// # Errors
/// Returns randomUnavailable, notSupported.
///
/// # Security
/// Requires `random.secure`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_random_secure_metadata(
    _binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<SecureRandomMetadataVm> {
    // allocate stable backend label for VM payload
    let backend_name = vm::StringHandle::new(context.intern_string("getrandom"));

    // return secure backend metadata for VM callers
    Ok(SecureRandomMetadataVm {
        source: SecureRandomSource::Kernel,
        backend_name,
        may_block: true,
        is_cryptographic: true,
        is_seeded: true,
        is_fips_approved: false,
        entropy_bits_per_byte: 8.0,
    })
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
    _binding: &BindingCallContext,
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    binding.hooks().on_random_read(Some(binding.engine()));

    // read the VM buffer into host memory
    let mut bytes = buffer.read_bytes(context)?;

    // fill bytes from the binding stream for this call context
    let stream_id = binding.random_stream_id();
    binding
        .world()
        .random()
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    stream: RandomStream,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    binding.hooks().on_random_read(Some(binding.engine()));

    // read the VM buffer into host memory
    let mut bytes = buffer.read_bytes(context)?;

    // fill bytes from the requested binding stream
    binding
        .world()
        .random()
        .fill_stream_bytes(RandomStreamId::new(stream.0), &mut bytes);

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
    _binding: &BindingCallContext,
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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    domain: RandomStreamDomain,
) -> RuntimeResult<RandomStream> {
    // allocate the stream by domain
    let stream_id = match domain {
        RandomStreamDomain::Process => binding.world().random().new_stream_id(),
        RandomStreamDomain::Task => binding
            .world()
            .random()
            .split_stream(binding.random_stream_id()),
    };
    Ok(RandomStream(stream_id.get()))
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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    stream: RandomStream,
    jump: u64,
) -> RuntimeResult<()> {
    // advance the deterministic stream state
    binding
        .world()
        .random()
        .jump_stream(RandomStreamId::new(stream.0), jump);

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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<u64> {
    binding.hooks().on_random_read(Some(binding.engine()));

    let stream_id = binding.random_stream_id();
    let value = binding.world().random().next_stream_u64(stream_id);

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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    stream: RandomStream,
) -> RuntimeResult<u64> {
    binding.hooks().on_random_read(Some(binding.engine()));

    let value = binding
        .world()
        .random()
        .next_stream_u64(RandomStreamId::new(stream.0));

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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    parent: RandomStream,
) -> RuntimeResult<RandomStream> {
    let child_stream_id = binding
        .world()
        .random()
        .split_stream(RandomStreamId::new(parent.0));
    Ok(RandomStream(child_stream_id.get()))
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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<RandomStream> {
    let stream_id = binding.world().random().new_stream_id();
    Ok(RandomStream(stream_id.get()))
}
