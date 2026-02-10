use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::random::{
    RandomStream, RandomStreamDomain, RandomStreamStateVm, SecureRandomInfoVm,
};
use crate::platform::{PlatformError, VmSlice};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.random.secure.bytes.
pub(super) fn destack_random_secure_bytes(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = buffer;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.random.secure.bytes is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.random.secure.bytesTry.
pub(super) fn destack_random_secure_bytes_try(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = buffer;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.random.secure.bytesTry is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.random.secure.info.
pub(super) fn destack_random_secure_info(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<SecureRandomInfoVm> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.random.secure.info is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.random.stream.export.
pub(super) fn destack_random_stream_export(
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

/// Stub for destack.random.stream.fillBytes.
pub(super) fn destack_random_fill_bytes(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = buffer;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.random.stream.fillBytes is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.random.stream.fillBytesFrom.
pub(super) fn destack_random_fill_bytes_from(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    stream: RandomStream,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (stream, buffer);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.random.stream.fillBytesFrom is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.random.stream.import.
pub(super) fn destack_random_stream_import(
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

/// Stub for destack.random.stream.in.
pub(super) fn destack_random_stream_in(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    domain: RandomStreamDomain,
) -> RuntimeResult<RandomStream> {
    let _ = domain;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.random.stream.in is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.random.stream.jump.
pub(super) fn destack_random_stream_jump(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    stream: RandomStream,
    jump: u64,
) -> RuntimeResult<()> {
    let _ = (stream, jump);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.random.stream.jump is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.random.stream.nextU64.
pub(super) fn destack_random_next_u64(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<u64> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.random.stream.nextU64 is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.random.stream.nextU64From.
pub(super) fn destack_random_next_u64_from(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    stream: RandomStream,
) -> RuntimeResult<u64> {
    let _ = stream;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.random.stream.nextU64From is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.random.stream.split.
pub(super) fn destack_random_stream_split(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    parent: RandomStream,
) -> RuntimeResult<RandomStream> {
    let _ = parent;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.random.stream.split is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.random.stream.stream.
pub(super) fn destack_random_stream(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<RandomStream> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.random.stream.stream is not available in the VM yet",
    ))
    .boxed())
}
