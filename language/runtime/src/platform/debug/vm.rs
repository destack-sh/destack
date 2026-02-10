use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::debug::{InspectorEndpointVm, ProfileKind, TraceLevel};
use crate::platform::{PlatformError, VmArray, resource};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.debug.core.breakNow.
pub(super) fn destack_debug_break_now(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.debug.core.breakNow is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.debug.core.mark.
pub(super) fn destack_debug_mark(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    label: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = label;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.debug.core.mark is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.debug.inspector.endpoint.
pub(super) fn destack_debug_inspector_endpoint(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::InspectorHandle,
) -> RuntimeResult<InspectorEndpointVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.debug.inspector.endpoint is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.debug.inspector.start.
pub(super) fn destack_debug_inspector_start(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    host: vm::StringHandle,
    port: u16,
) -> RuntimeResult<resource::InspectorHandle> {
    let _ = (host, port);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.debug.inspector.start is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.debug.inspector.stop.
pub(super) fn destack_debug_inspector_stop(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::InspectorHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.debug.inspector.stop is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.debug.profile.snapshot.
pub(super) fn destack_debug_profile_snapshot(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::ProfileHandle,
) -> RuntimeResult<VmArray<u8>> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.debug.profile.snapshot is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.debug.profile.start.
pub(super) fn destack_debug_profile_start(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    kind: ProfileKind,
) -> RuntimeResult<resource::ProfileHandle> {
    let _ = kind;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.debug.profile.start is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.debug.profile.stop.
pub(super) fn destack_debug_profile_stop(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::ProfileHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.debug.profile.stop is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.debug.trace.emit.
pub(super) fn destack_debug_trace_emit(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    category: vm::StringHandle,
    name: vm::StringHandle,
    payloadjson: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (category, name, payloadjson);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.debug.trace.emit is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.debug.trace.start.
pub(super) fn destack_debug_trace_start(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    level: TraceLevel,
    destination: vm::StringHandle,
) -> RuntimeResult<resource::TraceHandle> {
    let _ = (level, destination);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.debug.trace.start is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.debug.trace.stop.
pub(super) fn destack_debug_trace_stop(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::TraceHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.debug.trace.stop is not available in the VM yet",
    ))
    .boxed())
}
