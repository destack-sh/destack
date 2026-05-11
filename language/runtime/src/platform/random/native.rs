use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::random::{
    RandomStream, RandomStreamDomain, RandomStreamState, SecureRandomMetadata,
};
use crate::runtime::{self, BindingCallContext};

/// Fill a slice with cryptographically secure random bytes.
pub(crate) unsafe fn destack_random_secure_bytes(
    binding: &BindingCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { runtime::random::native::destack_random_secure_bytes(binding, buffer) }
}

/// Fill a slice with secure random bytes without blocking.
pub(crate) unsafe fn destack_random_secure_bytes_try(
    binding: &BindingCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { runtime::random::native::destack_random_secure_bytes_try(binding, buffer) }
}

/// Query secure randomness source metadata.
pub(crate) unsafe fn destack_random_secure_metadata(
    binding: &BindingCallContext,
    out: *mut SecureRandomMetadata,
) -> RuntimeResult<()> {
    unsafe { runtime::random::native::destack_random_secure_metadata(binding, out) }
}

/// Export deterministic stream state.
pub(crate) unsafe fn destack_random_stream_export(
    binding: &BindingCallContext,
    out: *mut RandomStreamState,
    stream: RandomStream,
) -> RuntimeResult<()> {
    unsafe { runtime::random::native::destack_random_stream_export(binding, out, stream) }
}

/// Fill a slice with deterministic random bytes from the default stream.
pub(crate) unsafe fn destack_random_fill_bytes(
    binding: &BindingCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { runtime::random::native::destack_random_fill_bytes(binding, buffer) }
}

/// Fill a slice with deterministic random bytes from a specific stream.
pub(crate) unsafe fn destack_random_fill_bytes_from(
    binding: &BindingCallContext,
    stream: RandomStream,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { runtime::random::native::destack_random_fill_bytes_from(binding, stream, buffer) }
}

/// Import deterministic stream state.
pub(crate) unsafe fn destack_random_stream_import(
    binding: &BindingCallContext,
    stream: RandomStream,
    state: RandomStreamState,
) -> RuntimeResult<()> {
    unsafe { runtime::random::native::destack_random_stream_import(binding, stream, state) }
}

/// Allocate a deterministic random stream in one domain.
pub(crate) unsafe fn destack_random_stream_in(
    binding: &BindingCallContext,
    out: *mut RandomStream,
    domain: RandomStreamDomain,
) -> RuntimeResult<()> {
    unsafe { runtime::random::native::destack_random_stream_in(binding, out, domain) }
}

/// Advance a deterministic stream by one jump count.
pub(crate) unsafe fn destack_random_stream_jump(
    binding: &BindingCallContext,
    stream: RandomStream,
    jump: u64,
) -> RuntimeResult<()> {
    unsafe { runtime::random::native::destack_random_stream_jump(binding, stream, jump) }
}

/// Return a deterministic random uint64 from the default stream.
pub(crate) unsafe fn destack_random_next_u64(
    binding: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    unsafe { runtime::random::native::destack_random_next_u64(binding, out) }
}

/// Return a deterministic random uint64 from a specific stream.
pub(crate) unsafe fn destack_random_next_u64_from(
    binding: &BindingCallContext,
    out: *mut u64,
    stream: RandomStream,
) -> RuntimeResult<()> {
    unsafe { runtime::random::native::destack_random_next_u64_from(binding, out, stream) }
}

/// Split a deterministic stream into one child stream.
pub(crate) unsafe fn destack_random_stream_split(
    binding: &BindingCallContext,
    out: *mut RandomStream,
    parent: RandomStream,
) -> RuntimeResult<()> {
    unsafe { runtime::random::native::destack_random_stream_split(binding, out, parent) }
}

/// Allocate a deterministic random stream identifier.
pub(crate) unsafe fn destack_random_stream(
    binding: &BindingCallContext,
    out: *mut RandomStream,
) -> RuntimeResult<()> {
    unsafe { runtime::random::native::destack_random_stream(binding, out) }
}
