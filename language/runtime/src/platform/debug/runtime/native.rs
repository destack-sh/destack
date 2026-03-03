use crate::diagnostic::RuntimeResult;
use crate::platform::NativeArray;
use crate::platform::debug::{InspectorEndpoint, ProfileKind, TraceLevel, native as debug_native};
use crate::runtime::{BindingCallContext, NativeStringRef};

use crate::platform::resource;

/// Request a runtime debug break.
///
/// Trigger one debugger stop request for the current runtime execution context.
/// Break behavior depends on whether a debugger session is attached.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime debugger integration hooks, not direct host syscalls.
///
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `debug.inspect`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_debug_break_now(context: &BindingCallContext) -> RuntimeResult<()> {
    unsafe { debug_native::destack_debug_break_now(context) }
}

/// Mark a debug timeline point.
///
/// Record one runtime debug marker with a user-provided label.
/// Marker ordering follows the active runtime scheduler and trace clock.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime debug instrumentation, not direct host syscalls.
///
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `debug.trace`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_debug_mark(
    context: &BindingCallContext,
    label: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { debug_native::destack_debug_mark(context, label) }
}

/// Read inspector endpoint metadata.
///
/// Return endpoint metadata for one active inspector session.
/// Metadata shape is stable across transports and does not expose host-private handles.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime inspector session state.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `debug.inspect`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_debug_inspector_endpoint(
    context: &BindingCallContext,
    out: *mut InspectorEndpoint,
    handle: resource::InspectorHandle,
) -> RuntimeResult<()> {
    unsafe { debug_native::destack_debug_inspector_endpoint(context, out, handle) }
}

/// Start an inspector session.
///
/// Create one debugger inspector transport endpoint and attach it to the active runtime.
/// Endpoint visibility and connection policy are controlled by runtime security settings.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime inspector transport, optionally backed by host sockets.
///
/// # Errors
/// Returns invalidArgument, netAddressInUse, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `debug.inspect`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_debug_inspector_start(
    context: &BindingCallContext,
    out: *mut resource::InspectorHandle,
    host: NativeStringRef,
    port: u16,
) -> RuntimeResult<()> {
    unsafe { debug_native::destack_debug_inspector_start(context, out, host, port) }
}

/// Stop an inspector session.
///
/// Detach one inspector transport endpoint from the runtime.
/// Active clients are disconnected according to runtime debugger policy.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime inspector transport teardown.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `debug.inspect`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_debug_inspector_stop(
    context: &BindingCallContext,
    handle: resource::InspectorHandle,
) -> RuntimeResult<()> {
    unsafe { debug_native::destack_debug_inspector_stop(context, handle) }
}

/// Capture a profiling snapshot.
///
/// Export one profile snapshot as bytes for offline analysis.
/// Output format is runtime-defined but versioned for tooling compatibility.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime profiler snapshot serialization.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `debug.profile`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_debug_profile_snapshot(
    context: &BindingCallContext,
    out: *mut NativeArray<u8>,
    handle: resource::ProfileHandle,
) -> RuntimeResult<()> {
    unsafe { debug_native::destack_debug_profile_snapshot(context, out, handle) }
}

/// Start a profiling session.
///
/// Start one runtime profiler with the selected profile kind.
/// Sampling cadence and event capture depend on runtime profiling policy.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime profiler integration, optionally backed by host perf APIs.
///
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `debug.profile`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_debug_profile_start(
    context: &BindingCallContext,
    out: *mut resource::ProfileHandle,
    kind: ProfileKind,
) -> RuntimeResult<()> {
    unsafe { debug_native::destack_debug_profile_start(context, out, kind) }
}

/// Stop a profiling session.
///
/// Stop one active profiling session and release profiler resources.
/// Profile data remains available for snapshot export until explicit cleanup.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime profiler teardown.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `debug.profile`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_debug_profile_stop(
    context: &BindingCallContext,
    handle: resource::ProfileHandle,
) -> RuntimeResult<()> {
    unsafe { debug_native::destack_debug_profile_stop(context, handle) }
}

/// Emit one trace event.
///
/// Append one structured trace record to the active runtime trace stream.
/// Event ordering follows runtime scheduling and stream flush policy.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime trace event encoder and sink.
///
/// # Errors
/// Returns invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `debug.trace`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_debug_trace_emit(
    context: &BindingCallContext,
    category: NativeStringRef,
    name: NativeStringRef,
    payloadjson: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { debug_native::destack_debug_trace_emit(context, category, name, payloadjson) }
}

/// Start a runtime trace stream.
///
/// Start one trace stream for runtime events with the requested level and destination.
/// Destination routing is runtime-defined and may target file, socket, or collector backends.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime trace pipeline, optionally backed by host transports.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `debug.trace`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_debug_trace_start(
    context: &BindingCallContext,
    out: *mut resource::TraceHandle,
    level: TraceLevel,
    destination: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { debug_native::destack_debug_trace_start(context, out, level, destination) }
}

/// Stop a runtime trace stream.
///
/// Stop one active trace stream and flush buffered events.
/// Flush behavior and durability guarantees follow runtime trace backend policy.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime trace stream teardown and flush logic.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `debug.trace`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_debug_trace_stop(
    context: &BindingCallContext,
    handle: resource::TraceHandle,
) -> RuntimeResult<()> {
    unsafe { debug_native::destack_debug_trace_stop(context, handle) }
}
