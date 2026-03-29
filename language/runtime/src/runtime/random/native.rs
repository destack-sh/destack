use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::random::{
    RandomStream, RandomStreamDomain, RandomStreamState, SecureRandomMetadata, SecureRandomSource,
};
use crate::runtime::BindingCallContext;
use crate::runtime::random::{RandomStreamId, StreamStateDecodeError};

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
pub(crate) unsafe fn destack_random_secure_bytes(
    binding: &BindingCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    binding.on_random_read();

    // resolve one mutable native slice
    let bytes = unsafe { buffer.as_mut_slice()? };

    // fill secure bytes from the world-routed random service
    binding.world().fill_secure_bytes(bytes)?;

    Ok(())
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
pub(crate) unsafe fn destack_random_secure_bytes_try(
    binding: &BindingCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    binding.on_random_read();

    // resolve one mutable native slice
    let bytes = unsafe { buffer.as_mut_slice()? };

    // request secure bytes in nonblocking mode when supported
    binding.world().try_fill_secure_bytes(bytes)?;

    Ok(())
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
pub(crate) unsafe fn destack_random_secure_metadata(
    binding: &BindingCallContext,
    out: *mut SecureRandomMetadata,
) -> RuntimeResult<()> {
    // validate out pointer for native ABI writes
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve world-routed secure random metadata
    let backend_name = binding.world().random().secure_backend_name();
    let may_block = binding.world().random().secure_may_block();

    // construct conservative metadata for the current secure source
    let info = SecureRandomMetadata {
        source: SecureRandomSource::Kernel,
        backend_name: NativeStringRef::from(backend_name),
        may_block,
        is_cryptographic: true,
        is_seeded: true,
        is_fips_approved: false,
        entropy_bits_per_byte: 8.0,
    };

    // write one metadata payload to the native out pointer
    unsafe {
        std::ptr::write(out, info);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut RandomStreamState,
    stream: RandomStream,
) -> RuntimeResult<()> {
    // validate out pointer for native ABI writes
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // serialize stream state into one versioned byte payload
    let bytes = binding
        .world()
        .random()
        .export_stream_state_bytes(RandomStreamId::new(stream.0));
    let state = RandomStreamState {
        version: STREAM_STATE_VERSION,
        bytes: binding.store_array(bytes),
    };

    // write stream state to the native ABI out pointer
    unsafe {
        std::ptr::write(out, state);
    }

    Ok(())
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
    binding: &BindingCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    binding.on_random_read();

    // resolve one mutable native slice
    let bytes = unsafe { buffer.as_mut_slice()? };

    // fill bytes from the runtime stream for this call binding
    let stream_id = binding.random_stream_id();
    binding.world().fill_stream_bytes(stream_id, bytes)?;

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
    binding: &BindingCallContext,
    stream: RandomStream,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    binding.on_random_read();

    // resolve one mutable native slice
    let bytes = unsafe { buffer.as_mut_slice()? };

    // fill bytes from the requested runtime stream
    binding
        .world()
        .fill_stream_bytes(RandomStreamId::new(stream.0), bytes)?;

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
    binding: &BindingCallContext,
    stream: RandomStream,
    state: RandomStreamState,
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

    // decode bytes from the ABI payload
    let bytes = unsafe { state.bytes.as_slice()? };

    // import the serialized stream state into the runtime random service
    binding
        .world()
        .random()
        .import_stream_state_bytes(RandomStreamId::new(stream.0), bytes)
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
pub(crate) unsafe fn destack_random_stream_in(
    binding: &BindingCallContext,
    out: *mut RandomStream,
    domain: RandomStreamDomain,
) -> RuntimeResult<()> {
    // validate out pointer for native ABI writes
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // allocate the stream by domain
    let stream_id = match domain {
        RandomStreamDomain::Process => binding.world().random().new_stream_id(),
        RandomStreamDomain::Task => binding
            .world()
            .random()
            .split_stream(binding.random_stream_id()),
    };
    // write the stream handle to the ABI out pointer
    unsafe {
        std::ptr::write(out, RandomStream(stream_id.get()));
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
    binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_random_next_u64(
    binding: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    // validate out pointer for native ABI writes
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    binding.on_random_read();

    // sample one value from the current runtime stream
    let value = binding
        .world()
        .next_stream_u64(binding.random_stream_id())?;

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
    binding: &BindingCallContext,
    out: *mut u64,
    stream: RandomStream,
) -> RuntimeResult<()> {
    // validate out pointer for native ABI writes
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    binding.on_random_read();

    // sample one value from the requested runtime stream
    let value = binding
        .world()
        .next_stream_u64(RandomStreamId::new(stream.0))?;

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
    binding: &BindingCallContext,
    out: *mut RandomStream,
    parent: RandomStream,
) -> RuntimeResult<()> {
    // validate out pointer for native ABI writes
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // derive one child stream from the parent stream
    let child_stream = binding
        .world()
        .random()
        .split_stream(RandomStreamId::new(parent.0));
    // write the child stream handle to the ABI out pointer
    unsafe {
        std::ptr::write(out, RandomStream(child_stream.get()));
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
    binding: &BindingCallContext,
    out: *mut RandomStream,
) -> RuntimeResult<()> {
    // validate out pointer for native ABI writes
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // allocate one process-scoped stream id
    let stream_id = binding.world().random().new_stream_id();
    // write the stream handle to the ABI out pointer
    unsafe {
        std::ptr::write(out, RandomStream(stream_id.get()));
    }

    Ok(())
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
