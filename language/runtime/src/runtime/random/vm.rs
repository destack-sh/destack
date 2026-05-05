use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::random::{
    RandomStream, RandomStreamDomain, RandomStreamStateVm, SecureRandomMetadataVm,
    SecureRandomSource,
};
use crate::platform::{PlatformError, VmArray, VmSlice};
use crate::runtime::BindingCallContext;
use crate::runtime::random::{RandomStreamId, StreamStateDecodeError};
use destack_vm as vm;

/// Serialized stream state payload version.
const STREAM_STATE_VERSION: u32 = 1;

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
    context: &mut vm::BindingContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    binding.on_random_read();

    // resolve VM bytes into host memory
    let mut bytes = buffer.read_bytes(&context.read())?;

    // fill secure bytes from the world-routed random service
    binding.world().fill_secure_bytes(&mut bytes)?;

    // write secure bytes back into VM memory
    buffer.write_bytes(&mut context.write(), &bytes)
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
    context: &mut vm::BindingContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    binding.on_random_read();

    // resolve VM bytes into host memory
    let mut bytes = buffer.read_bytes(&context.read())?;

    // fill secure bytes in nonblocking mode when supported
    binding.world().try_fill_secure_bytes(&mut bytes)?;

    // write secure bytes back into VM memory
    buffer.write_bytes(&mut context.write(), &bytes)
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
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<SecureRandomMetadataVm> {
    // allocate the active backend label for VM payload
    let backend_name = context
        .string_handle(binding.world().random().secure_backend_name())
        .map_err(Box::<RuntimeError>::from)?;
    let may_block = binding.world().random().secure_may_block();

    // return secure backend metadata for VM callers
    Ok(SecureRandomMetadataVm {
        source: SecureRandomSource::Kernel,
        backend_name,
        may_block,
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
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    stream: RandomStream,
) -> RuntimeResult<RandomStreamStateVm> {
    // serialize stream state into one versioned byte payload
    let bytes = binding
        .world()
        .random()
        .export_stream_state_bytes(RandomStreamId::new(stream.0));
    let state = RandomStreamStateVm {
        version: STREAM_STATE_VERSION,
        bytes: VmArray::from_bytes(&mut context.write(), &bytes)?,
    };

    Ok(state)
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
    context: &mut vm::BindingContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    binding.on_random_read();

    // read the VM buffer into host memory
    let mut bytes = buffer.read_bytes(&context.read())?;

    // fill bytes from the binding stream for this call context
    let stream_id = binding.random_stream_id();
    binding.world().fill_stream_bytes(stream_id, &mut bytes)?;

    // write the filled bytes back into the VM buffer
    buffer.write_bytes(&mut context.write(), &bytes)
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
    context: &mut vm::BindingContext<'_>,
    stream: RandomStream,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    binding.on_random_read();

    // read the VM buffer into host memory
    let mut bytes = buffer.read_bytes(&context.read())?;

    // fill bytes from the requested binding stream
    binding
        .world()
        .fill_stream_bytes(RandomStreamId::new(stream.0), &mut bytes)?;

    // write the filled bytes back into the VM buffer
    buffer.write_bytes(&mut context.write(), &bytes)
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
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    stream: RandomStream,
    state: RandomStreamStateVm,
) -> RuntimeResult<()> {
    // validate stream state payload version
    if state.version != STREAM_STATE_VERSION {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "state.version",
            format!(
                "unsupported random stream state version: expected {STREAM_STATE_VERSION}, got {}",
                state.version
            ),
        ))
        .boxed());
    }

    // decode bytes from the VM payload
    let bytes = state.bytes.read_bytes(&context.read())?;

    // import the serialized stream state into the runtime random service
    binding
        .world()
        .random()
        .import_stream_state_bytes(RandomStreamId::new(stream.0), &bytes)
        .map_err(|error| stream_state_decode_error("state", error))?;

    Ok(())
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
    _context: &mut vm::BindingContext<'_>,
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
    _context: &mut vm::BindingContext<'_>,
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
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<u64> {
    binding.on_random_read();

    let stream_id = binding.random_stream_id();
    let value = binding.world().next_stream_u64(stream_id)?;

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
    _context: &mut vm::BindingContext<'_>,
    stream: RandomStream,
) -> RuntimeResult<u64> {
    binding.on_random_read();

    let value = binding
        .world()
        .next_stream_u64(RandomStreamId::new(stream.0))?;

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
    _context: &mut vm::BindingContext<'_>,
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
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<RandomStream> {
    let stream_id = binding.world().random().new_stream_id();
    Ok(RandomStream(stream_id.get()))
}

/// Build one invalid stream-state error from one decode error.
fn stream_state_decode_error(
    field: &'static str,
    error: StreamStateDecodeError,
) -> Box<RuntimeError> {
    let reason = match error {
        StreamStateDecodeError::InvalidLength => {
            "stream state payload length does not match expected format"
        }
        StreamStateDecodeError::UnsupportedVersion => "stream state payload version is unsupported",
        StreamStateDecodeError::InvalidInitializedFlag => {
            "stream state payload initialized flag is invalid"
        }
    };

    RuntimeError::from(PlatformError::invalid_argument_value(field, reason)).boxed()
}
