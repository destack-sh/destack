use crate::diagnostic::RuntimeResult;
use crate::platform::VmSlice;
use crate::platform::random::{
    RandomStream, RandomStreamDomain, RandomStreamStateVm, SecureRandomMetadataVm,
};
use crate::runtime::{self, BindingCallContext};
use destack_vm;

/// Fill a slice with cryptographically secure random bytes.
pub(crate) fn destack_random_secure_bytes(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    runtime::random::vm::destack_random_secure_bytes(binding, context, buffer)
}

/// Fill a slice with secure random bytes without blocking.
pub(crate) fn destack_random_secure_bytes_try(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    runtime::random::vm::destack_random_secure_bytes_try(binding, context, buffer)
}

/// Query secure randomness source metadata.
pub(crate) fn destack_random_secure_metadata(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<SecureRandomMetadataVm> {
    runtime::random::vm::destack_random_secure_metadata(binding, context)
}

/// Export deterministic stream state.
pub(crate) fn destack_random_stream_export(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    stream: RandomStream,
) -> RuntimeResult<RandomStreamStateVm> {
    runtime::random::vm::destack_random_stream_export(binding, context, stream)
}

/// Fill a slice with deterministic random bytes from the default stream.
pub(crate) fn destack_random_fill_bytes(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    runtime::random::vm::destack_random_fill_bytes(binding, context, buffer)
}

/// Fill a slice with deterministic random bytes from a specific stream.
pub(crate) fn destack_random_fill_bytes_from(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    stream: RandomStream,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    runtime::random::vm::destack_random_fill_bytes_from(binding, context, stream, buffer)
}

/// Import deterministic stream state.
pub(crate) fn destack_random_stream_import(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    stream: RandomStream,
    state: RandomStreamStateVm,
) -> RuntimeResult<()> {
    runtime::random::vm::destack_random_stream_import(binding, context, stream, state)
}

/// Allocate a deterministic random stream in one domain.
pub(crate) fn destack_random_stream_in(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    domain: RandomStreamDomain,
) -> RuntimeResult<RandomStream> {
    runtime::random::vm::destack_random_stream_in(binding, context, domain)
}

/// Advance a deterministic stream by one jump count.
pub(crate) fn destack_random_stream_jump(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    stream: RandomStream,
    jump: u64,
) -> RuntimeResult<()> {
    runtime::random::vm::destack_random_stream_jump(binding, context, stream, jump)
}

/// Return a deterministic random uint64 from the default stream.
pub(crate) fn destack_random_next_u64(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<u64> {
    runtime::random::vm::destack_random_next_u64(binding, context)
}

/// Return a deterministic random uint64 from a specific stream.
pub(crate) fn destack_random_next_u64_from(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    stream: RandomStream,
) -> RuntimeResult<u64> {
    runtime::random::vm::destack_random_next_u64_from(binding, context, stream)
}

/// Split a deterministic stream into one child stream.
pub(crate) fn destack_random_stream_split(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    parent: RandomStream,
) -> RuntimeResult<RandomStream> {
    runtime::random::vm::destack_random_stream_split(binding, context, parent)
}

/// Allocate a deterministic random stream identifier.
pub(crate) fn destack_random_stream(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<RandomStream> {
    runtime::random::vm::destack_random_stream(binding, context)
}
