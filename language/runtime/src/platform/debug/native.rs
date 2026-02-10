#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::debug::bindings_generated as bindings;
use crate::platform::{NativeArray, NativeStringRef, PlatformError};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::debug::{InspectorEndpoint, ProfileKind, TraceLevel};
use crate::platform::resource;

/// Stub for destack.debug.core.breakNow.
pub unsafe fn destack_debug_break_now(context: &RuntimeCallContext) -> RuntimeResult<()> {
    context.check_policy(DEBUG_CORE_BREAK_NOW)?;

    Err(RuntimeError::from(PlatformError::not_supported("destack.debug.core.breakNow")).boxed())
}

/// Stub for destack.debug.core.mark.
pub unsafe fn destack_debug_mark(
    context: &RuntimeCallContext,
    label: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(DEBUG_CORE_MARK)?;
    let _ = label;

    Err(RuntimeError::from(PlatformError::not_supported("destack.debug.core.mark")).boxed())
}

/// Stub for destack.debug.inspector.endpoint.
pub unsafe fn destack_debug_inspector_endpoint(
    context: &RuntimeCallContext,
    out: *mut InspectorEndpoint,
    handle: resource::InspectorHandle,
) -> RuntimeResult<()> {
    context.check_policy(DEBUG_INSPECTOR_ENDPOINT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.debug.inspector.endpoint",
    ))
    .boxed())
}

/// Stub for destack.debug.inspector.start.
pub unsafe fn destack_debug_inspector_start(
    context: &RuntimeCallContext,
    out: *mut resource::InspectorHandle,
    host: NativeStringRef,
    port: u16,
) -> RuntimeResult<()> {
    context.check_policy(DEBUG_INSPECTOR_START)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, host, port);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.debug.inspector.start",
    ))
    .boxed())
}

/// Stub for destack.debug.inspector.stop.
pub unsafe fn destack_debug_inspector_stop(
    context: &RuntimeCallContext,
    handle: resource::InspectorHandle,
) -> RuntimeResult<()> {
    context.check_policy(DEBUG_INSPECTOR_STOP)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.debug.inspector.stop")).boxed())
}

/// Stub for destack.debug.profile.snapshot.
pub unsafe fn destack_debug_profile_snapshot(
    context: &RuntimeCallContext,
    out: *mut NativeArray<u8>,
    handle: resource::ProfileHandle,
) -> RuntimeResult<()> {
    context.check_policy(DEBUG_PROFILE_SNAPSHOT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.debug.profile.snapshot",
    ))
    .boxed())
}

/// Stub for destack.debug.profile.start.
pub unsafe fn destack_debug_profile_start(
    context: &RuntimeCallContext,
    out: *mut resource::ProfileHandle,
    kind: ProfileKind,
) -> RuntimeResult<()> {
    context.check_policy(DEBUG_PROFILE_START)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, kind);

    Err(RuntimeError::from(PlatformError::not_supported("destack.debug.profile.start")).boxed())
}

/// Stub for destack.debug.profile.stop.
pub unsafe fn destack_debug_profile_stop(
    context: &RuntimeCallContext,
    handle: resource::ProfileHandle,
) -> RuntimeResult<()> {
    context.check_policy(DEBUG_PROFILE_STOP)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.debug.profile.stop")).boxed())
}

/// Stub for destack.debug.trace.emit.
pub unsafe fn destack_debug_trace_emit(
    context: &RuntimeCallContext,
    category: NativeStringRef,
    name: NativeStringRef,
    payloadjson: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(DEBUG_TRACE_EMIT)?;
    let _ = (category, name, payloadjson);

    Err(RuntimeError::from(PlatformError::not_supported("destack.debug.trace.emit")).boxed())
}

/// Stub for destack.debug.trace.start.
pub unsafe fn destack_debug_trace_start(
    context: &RuntimeCallContext,
    out: *mut resource::TraceHandle,
    level: TraceLevel,
    destination: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(DEBUG_TRACE_START)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, level, destination);

    Err(RuntimeError::from(PlatformError::not_supported("destack.debug.trace.start")).boxed())
}

/// Stub for destack.debug.trace.stop.
pub unsafe fn destack_debug_trace_stop(
    context: &RuntimeCallContext,
    handle: resource::TraceHandle,
) -> RuntimeResult<()> {
    context.check_policy(DEBUG_TRACE_STOP)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.debug.trace.stop")).boxed())
}
