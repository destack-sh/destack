#![allow(dead_code)]

use std::ffi::CString;
use std::os::fd::RawFd;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeStringRef;
use crate::platform::diagnostic::{PlatformErrorCode, io_error_code_from_errno};
use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceId, ResourceKind};
use crate::platform::{PlatformError, core as core_platform, resource};
use crate::runtime::BindingCallContext;

/// Resource label used for pipe endpoints.
pub(super) const PIPE_RESOURCE_LABEL: &str = "ipc.pipe";
/// Resource label used for shared-memory objects.
pub(super) const SHARED_MEMORY_RESOURCE_LABEL: &str = "ipc.shared_memory";
/// Resource label used for semaphore objects.
pub(super) const SEMAPHORE_RESOURCE_LABEL: &str = "ipc.semaphore";
/// Resource label used for message queue objects.
pub(super) const MESSAGE_QUEUE_RESOURCE_LABEL: &str = "ipc.message_queue";
/// Resource label used for transferred descriptor objects.
pub(super) const TRANSFERRED_RESOURCE_LABEL: &str = "ipc.transferred";

/// Payload for one named semaphore handle.
#[derive(Debug)]
pub(super) struct UnixSemaphoreState {
    /// Native semaphore pointer returned by sem_open.
    pub(super) semaphore: usize,
}

/// Payload for one POSIX message queue handle.
#[cfg(target_os = "linux")]
#[derive(Debug)]
pub(super) struct UnixMessageQueueState {
    /// Native queue descriptor returned by mq_open.
    pub(super) queue: libc::mqd_t,
}

/// Finalizer that closes one unix file descriptor.
#[derive(Debug)]
pub(super) struct UnixFileDescriptorFinalizer {
    /// Descriptor to close.
    pub(super) descriptor: RawFd,
}

impl ResourceFinalizer for UnixFileDescriptorFinalizer {
    /// Close the descriptor during resource cleanup.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            libc::close(self.descriptor);
        }
    }
}

/// Finalizer that closes one named semaphore pointer.
#[derive(Debug)]
pub(super) struct UnixSemaphoreFinalizer {
    /// Semaphore pointer to close.
    pub(super) semaphore: usize,
}

impl ResourceFinalizer for UnixSemaphoreFinalizer {
    /// Close the semaphore during resource cleanup.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        let semaphore = self.semaphore as *mut libc::sem_t;
        unsafe {
            libc::sem_close(semaphore);
        }
    }
}

/// Finalizer that closes one POSIX message queue descriptor.
#[cfg(target_os = "linux")]
#[derive(Debug)]
pub(super) struct UnixMessageQueueFinalizer {
    /// Queue descriptor to close.
    pub(super) queue: libc::mqd_t,
}

#[cfg(target_os = "linux")]
impl ResourceFinalizer for UnixMessageQueueFinalizer {
    /// Close the queue descriptor during resource cleanup.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            libc::mq_close(self.queue);
        }
    }
}

/// Build one mapped unix I/O error from explicit errno.
pub(super) fn io_error_with_errno(
    operation: &'static str,
    syscall: &'static str,
    errno: i32,
    message: &str,
) -> Box<RuntimeError> {
    let platform_code = io_error_code_from_errno(errno);
    RuntimeError::from(PlatformError::io_with(
        platform_code,
        None,
        Some(errno),
        Some(operation.to_string()),
        None,
        format!("{syscall} failed: {message}"),
    ))
    .boxed()
}

/// Build one mapped unix I/O error from current errno.
pub(super) fn io_error(
    operation: &'static str,
    syscall: &'static str,
    message: &str,
) -> Box<RuntimeError> {
    let errno = core_platform::get_errno();
    io_error_with_errno(operation, syscall, errno, message)
}

/// Build one timed-out runtime error.
pub(super) fn timed_out(operation: &'static str, message: &str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoTimedOut),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.to_string(),
    ))
    .boxed()
}

/// Build one would-block runtime error.
pub(super) fn would_block(operation: &'static str, message: &str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoWouldBlock),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.to_string(),
    ))
    .boxed()
}

/// Decode one native name and normalize it for POSIX object APIs.
pub(super) fn posix_name(name: NativeStringRef, field: &'static str) -> RuntimeResult<CString> {
    let name = unsafe { name.as_str()? };
    if name.is_empty() {
        return Err(core_platform::invalid_argument(
            field,
            "name must not be empty",
        ));
    }

    let normalized_name = if name.starts_with('/') {
        name.to_string()
    } else {
        format!("/{name}")
    };

    CString::new(normalized_name).map_err(|_| {
        core_platform::invalid_argument(field, "name must not contain interior nul bytes")
    })
}

/// Convert one timeout to an absolute realtime deadline.
pub(super) fn realtime_deadline(
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<libc::timespec> {
    // read one realtime clock value
    let mut now = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let rc = unsafe { libc::clock_gettime(libc::CLOCK_REALTIME, &mut now) };
    if rc != 0 {
        return Err(io_error(
            operation,
            "clock_gettime",
            "failed to read realtime clock",
        ));
    }

    // add timeout duration to realtime clock
    let timeout_seconds = timeout_ns / 1_000_000_000;
    let timeout_nanoseconds = timeout_ns % 1_000_000_000;

    let base_seconds = i128::from(now.tv_sec);
    let base_nanoseconds = i128::from(now.tv_nsec);
    let mut deadline_seconds = base_seconds + i128::from(timeout_seconds);
    let mut deadline_nanoseconds = base_nanoseconds + i128::from(timeout_nanoseconds);

    if deadline_nanoseconds >= 1_000_000_000 {
        deadline_seconds += 1;
        deadline_nanoseconds -= 1_000_000_000;
    }

    if deadline_seconds > i128::from(i64::MAX) {
        return Err(core_platform::invalid_argument(
            "timeoutNs",
            "timeout produced a realtime deadline outside host range",
        ));
    }

    Ok(libc::timespec {
        tv_sec: deadline_seconds as libc::time_t,
        tv_nsec: deadline_nanoseconds as libc::c_long,
    })
}

/// Resolve one pipe handle into one unix file descriptor.
pub(super) fn pipe_descriptor(
    binding: &BindingCallContext,
    handle: resource::PipeHandle,
    operation: &'static str,
) -> RuntimeResult<RawFd> {
    let descriptor = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Pipe {
                return None;
            }

            entry.fd()
        })
        .flatten()
        .ok_or_else(|| {
            core_platform::invalid_argument(
                "handle",
                format!("{operation} expected one valid pipe handle"),
            )
        })?;

    Ok(descriptor)
}

/// Resolve one shared-memory handle into one unix file descriptor.
pub(super) fn shared_memory_descriptor(
    binding: &BindingCallContext,
    handle: resource::SharedMemoryHandle,
    operation: &'static str,
) -> RuntimeResult<RawFd> {
    let descriptor = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::SharedMemory {
                return None;
            }

            entry.fd()
        })
        .flatten()
        .ok_or_else(|| {
            core_platform::invalid_argument(
                "handle",
                format!("{operation} expected one valid shared-memory handle"),
            )
        })?;

    Ok(descriptor)
}

/// Resolve one socket handle into one unix file descriptor.
pub(super) fn socket_descriptor(
    binding: &BindingCallContext,
    handle: resource::SocketHandle,
    operation: &'static str,
) -> RuntimeResult<RawFd> {
    let descriptor = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Socket {
                return None;
            }

            entry.fd()
        })
        .flatten()
        .ok_or_else(|| {
            core_platform::invalid_argument(
                "socket",
                format!("{operation} expected one valid socket handle"),
            )
        })?;

    Ok(descriptor)
}

/// Resolve one transferred handle into one unix file descriptor.
pub(super) fn transferable_descriptor(
    binding: &BindingCallContext,
    handle: resource::TransferredHandle,
    field: &'static str,
) -> RuntimeResult<RawFd> {
    let descriptor = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| entry.fd())
        .flatten()
        .ok_or_else(|| core_platform::invalid_argument(field, "handle is not fd-backed"))?;

    Ok(descriptor)
}

/// Resolve one message-queue handle into one native queue descriptor.
#[cfg(target_os = "linux")]
pub(super) fn message_queue_descriptor(
    binding: &BindingCallContext,
    handle: resource::MessageQueueHandle,
    operation: &'static str,
) -> RuntimeResult<libc::mqd_t> {
    let queue = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::MessageQueue {
                return None;
            }

            entry
                .payload_ref::<UnixMessageQueueState>()
                .map(|payload| payload.queue)
        })
        .flatten()
        .ok_or_else(|| {
            core_platform::invalid_argument(
                "handle",
                format!("{operation} expected one valid message-queue handle"),
            )
        })?;

    Ok(queue)
}

/// Resolve one semaphore handle into one native semaphore pointer.
pub(super) fn semaphore_pointer(
    binding: &BindingCallContext,
    handle: resource::SemaphoreHandle,
    operation: &'static str,
) -> RuntimeResult<*mut libc::sem_t> {
    let pointer = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Semaphore {
                return None;
            }

            entry
                .payload_ref::<UnixSemaphoreState>()
                .map(|payload| payload.semaphore as *mut libc::sem_t)
        })
        .flatten()
        .ok_or_else(|| {
            core_platform::invalid_argument(
                "handle",
                format!("{operation} expected one valid semaphore handle"),
            )
        })?;

    Ok(pointer)
}

/// Register one pipe descriptor in the runtime resource table.
pub(super) fn register_pipe_descriptor(
    binding: &BindingCallContext,
    descriptor: RawFd,
) -> resource::PipeHandle {
    let entry = ResourceEntry::labeled_fd_finalizer(
        ResourceKind::Pipe,
        PIPE_RESOURCE_LABEL,
        descriptor,
        UnixFileDescriptorFinalizer { descriptor },
    );
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    resource::PipeHandle(resource_id)
}

/// Register one shared-memory descriptor in the runtime resource table.
pub(super) fn register_shared_memory_descriptor(
    binding: &BindingCallContext,
    descriptor: RawFd,
) -> resource::SharedMemoryHandle {
    let entry = ResourceEntry::labeled_fd_finalizer(
        ResourceKind::SharedMemory,
        SHARED_MEMORY_RESOURCE_LABEL,
        descriptor,
        UnixFileDescriptorFinalizer { descriptor },
    );
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    resource::SharedMemoryHandle(resource_id)
}

/// Register one semaphore pointer in the runtime resource table.
pub(super) fn register_semaphore(
    binding: &BindingCallContext,
    semaphore: *mut libc::sem_t,
) -> resource::SemaphoreHandle {
    let entry = ResourceEntry::labeled_payload_finalizer(
        ResourceKind::Semaphore,
        SEMAPHORE_RESOURCE_LABEL,
        UnixSemaphoreState {
            semaphore: semaphore as usize,
        },
        UnixSemaphoreFinalizer {
            semaphore: semaphore as usize,
        },
    );
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    resource::SemaphoreHandle(resource_id)
}

/// Register one transferred descriptor in the runtime resource table.
pub(super) fn register_transferred_descriptor(
    binding: &BindingCallContext,
    descriptor: RawFd,
) -> resource::TransferredHandle {
    let entry = ResourceEntry::labeled_fd_finalizer(
        ResourceKind::Transferred,
        TRANSFERRED_RESOURCE_LABEL,
        descriptor,
        UnixFileDescriptorFinalizer { descriptor },
    );
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    resource::TransferredHandle(resource_id)
}

/// Register one POSIX message queue descriptor in the runtime resource table.
#[cfg(target_os = "linux")]
pub(super) fn register_message_queue(
    binding: &BindingCallContext,
    queue: libc::mqd_t,
) -> resource::MessageQueueHandle {
    let entry = ResourceEntry::labeled_payload_finalizer(
        ResourceKind::MessageQueue,
        MESSAGE_QUEUE_RESOURCE_LABEL,
        UnixMessageQueueState { queue },
        UnixMessageQueueFinalizer { queue },
    );
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    resource::MessageQueueHandle(resource_id)
}
