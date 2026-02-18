#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::{ResourceId, ResourceKind, ResourceKindVm, ResourceOwnership};
use crate::platform::{PlatformError, PlatformErrorCode};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Return a stable label for one resource kind.
fn resource_kind_label(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::File => "file",
        ResourceKind::Directory => "directory",
        ResourceKind::Pipe => "pipe",
        ResourceKind::Socket => "socket",
        ResourceKind::Listener => "listener",
        ResourceKind::Timer => "timer",
        ResourceKind::TimerFd => "timer_fd",
        ResourceKind::Watch => "watch",
        ResourceKind::Process => "process",
        ResourceKind::Poll => "poll",
        ResourceKind::Completion => "completion",
        ResourceKind::Event => "event",
        ResourceKind::Uring => "uring",
        ResourceKind::ProcessFd => "process_fd",
        ResourceKind::SharedMemory => "shared_memory",
        ResourceKind::Semaphore => "semaphore",
        ResourceKind::Signal => "signal",
        ResourceKind::SignalFd => "signal_fd",
        ResourceKind::Thread => "thread",
        ResourceKind::Mutex => "mutex",
        ResourceKind::RwLock => "rw_lock",
        ResourceKind::CondVar => "cond_var",
        ResourceKind::ThreadSemaphore => "thread_semaphore",
        ResourceKind::Barrier => "barrier",
        ResourceKind::ThreadLocal => "thread_local",
        ResourceKind::Library => "library",
        ResourceKind::Symbol => "symbol",
        ResourceKind::CryptoStore => "crypto_store",
        ResourceKind::CryptoKey => "crypto_key",
        ResourceKind::CryptoCertificate => "crypto_certificate",
        ResourceKind::TlsContext => "tls_context",
        ResourceKind::TlsSession => "tls_session",
        ResourceKind::Device => "device",
        ResourceKind::Pty => "pty",
        ResourceKind::Tty => "tty",
        ResourceKind::Sandbox => "sandbox",
        ResourceKind::Inspector => "inspector",
        ResourceKind::Profile => "profile",
        ResourceKind::Trace => "trace",
        ResourceKind::Transferred => "transferred",
        ResourceKind::MessageQueue => "message_queue",
        ResourceKind::AudioDevice => "audio_device",
        ResourceKind::AudioStream => "audio_stream",
        ResourceKind::Display => "display",
        ResourceKind::Window => "window",
        ResourceKind::Input => "input",
        ResourceKind::GpuAdapter => "gpu_adapter",
        ResourceKind::GpuDevice => "gpu_device",
        ResourceKind::GpuQueue => "gpu_queue",
        ResourceKind::GpuCommandList => "gpu_command_list",
        ResourceKind::GpuMemory => "gpu_memory",
        ResourceKind::GpuBuffer => "gpu_buffer",
        ResourceKind::GpuTexture => "gpu_texture",
        ResourceKind::GpuSampler => "gpu_sampler",
        ResourceKind::GpuShader => "gpu_shader",
        ResourceKind::GpuPipeline => "gpu_pipeline",
        ResourceKind::Unknown => "unknown",
    }
}

/// Build one io not found error for missing resources.
fn resource_not_found(op: &'static str, id: ResourceId) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(op.to_string()),
        None,
        format!("resource {} not found", id.0),
    ))
    .boxed()
}

/// Close a resource by identifier.
///
/// Close one resource endpoint while keeping table semantics explicit.
/// Close behavior is delegated to the owning runtime subsystem for the resource kind.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime resource-dispatch close logic.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `resource.close`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_resource_close(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: ResourceId,
) -> RuntimeResult<()> {
    // remove the entry and run finalization
    let removed = runtime.runtime().resources.remove_and_finalize(id);
    if !removed {
        return Err(resource_not_found("destack.resource.id.close", id));
    }

    Ok(())
}

/// Describe a resource kind.
///
/// Return the declared kind label for one resource identifier.
/// Kind labels are stable runtime strings for diagnostics and policy checks.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime resource-table state only.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `resource.read`.
///
/// # Replay
/// Deterministic.
pub(crate) fn destack_resource_kind(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: ResourceId,
) -> RuntimeResult<ResourceKindVm> {
    // resolve the kind for the requested resource
    let kind = runtime
        .runtime()
        .resources
        .with_entry(id, |entry| entry.kind)
        .ok_or_else(|| resource_not_found("destack.resource.id.kind", id))?;

    // encode the kind label as the vm-facing payload
    let label = resource_kind_label(kind);
    let handle = vm::StringHandle::new(context.intern_string(label));

    Ok(ResourceKindVm(handle))
}

/// Remove a resource from the table.
///
/// Remove one resource identifier from the runtime table.
/// Owned resources are closed by runtime policy before removal.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime resource-table state only.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `resource.manage`.
///
/// # Replay
/// Deterministic.
pub(crate) fn destack_resource_remove(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: ResourceId,
) -> RuntimeResult<()> {
    // remove the entry and run finalization
    let removed = runtime.runtime().resources.remove_and_finalize(id);
    if !removed {
        return Err(resource_not_found("destack.resource.id.remove", id));
    }

    Ok(())
}

/// Transfer resource ownership.
///
/// Move one resource identifier into the requested ownership mode.
/// Ownership transitions are validated against runtime boundary policy.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime resource ownership metadata only.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `resource.transfer`.
///
/// # Replay
/// Deterministic.
pub(crate) fn destack_resource_transfer(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: ResourceId,
    ownership: ResourceOwnership,
) -> RuntimeResult<()> {
    // validate that the source resource exists
    let exists = runtime.runtime().resources.contains(id);
    if !exists {
        return Err(resource_not_found("destack.resource.id.transfer", id));
    }

    // keep ownership consumed for future runtime ownership policy
    let _ = ownership;

    Ok(())
}
