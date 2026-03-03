#![allow(dead_code)]

use crate::diagnostic::RuntimeResult;
use crate::platform::random::runtime::native as runtime_random;
use crate::platform::random::{
    RandomStream, RandomStreamDomain, RandomStreamState, SecureRandomMetadata,
};
use crate::runtime::{BindingCallContext, NativeSlice};

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
    context: &BindingCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { runtime_random::destack_random_secure_bytes(context, buffer) }
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
    context: &BindingCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { runtime_random::destack_random_secure_bytes_try(context, buffer) }
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
    context: &BindingCallContext,
    out: *mut SecureRandomMetadata,
) -> RuntimeResult<()> {
    unsafe { runtime_random::destack_random_secure_metadata(context, out) }
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
    context: &BindingCallContext,
    out: *mut RandomStreamState,
    stream: RandomStream,
) -> RuntimeResult<()> {
    unsafe { runtime_random::destack_random_stream_export(context, out, stream) }
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
    context: &BindingCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { runtime_random::destack_random_fill_bytes(context, buffer) }
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
    context: &BindingCallContext,
    stream: RandomStream,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { runtime_random::destack_random_fill_bytes_from(context, stream, buffer) }
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
    context: &BindingCallContext,
    stream: RandomStream,
    state: RandomStreamState,
) -> RuntimeResult<()> {
    unsafe { runtime_random::destack_random_stream_import(context, stream, state) }
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
    context: &BindingCallContext,
    out: *mut RandomStream,
    domain: RandomStreamDomain,
) -> RuntimeResult<()> {
    unsafe { runtime_random::destack_random_stream_in(context, out, domain) }
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
    context: &BindingCallContext,
    stream: RandomStream,
    jump: u64,
) -> RuntimeResult<()> {
    unsafe { runtime_random::destack_random_stream_jump(context, stream, jump) }
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
    context: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    unsafe { runtime_random::destack_random_next_u64(context, out) }
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
    context: &BindingCallContext,
    out: *mut u64,
    stream: RandomStream,
) -> RuntimeResult<()> {
    unsafe { runtime_random::destack_random_next_u64_from(context, out, stream) }
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
    context: &BindingCallContext,
    out: *mut RandomStream,
    parent: RandomStream,
) -> RuntimeResult<()> {
    unsafe { runtime_random::destack_random_stream_split(context, out, parent) }
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
    context: &BindingCallContext,
    out: *mut RandomStream,
) -> RuntimeResult<()> {
    unsafe { runtime_random::destack_random_stream(context, out) }
}
