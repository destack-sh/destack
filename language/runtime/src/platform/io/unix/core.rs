use std::ffi::c_void;

use libc::{c_int, c_ulong};

use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(target_os = "linux")]
use crate::platform::IoUringProactor;
#[cfg(all(unix, not(target_os = "linux")))]
use crate::platform::UnixProactor;
use crate::platform::io::{
    DescriptorControlCommand, DescriptorControlFlags, DescriptorRequest, DescriptorResult,
    EventToken, PollBackend, core as io_core,
};
use crate::platform::proactor::Proactor;
use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceKind};
use crate::platform::{PlatformError, PlatformErrorCode, ResourceId};
use crate::runtime::BindingCallContext;
use crate::runtime::poller::{HostPollerBackend, PlatformHandle};

/// Host ioctl request code type.
#[cfg(target_os = "android")]
type HostIoctlRequest = libc::Ioctl;
/// Host ioctl request code type.
#[cfg(not(target_os = "android"))]
type HostIoctlRequest = c_ulong;

/// Return one standardized null-pointer error for output arguments.
pub(super) fn require_out<T>(out: *mut T) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    Ok(())
}

/// Finalizer that closes one raw file descriptor.
#[derive(Debug)]
struct UnixDescriptorFinalizer {
    /// Descriptor to close.
    descriptor: c_int,
    /// Optional paired descriptor to close.
    paired_descriptor: Option<c_int>,
}

impl ResourceFinalizer for UnixDescriptorFinalizer {
    /// Close one or two owned descriptors.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        if let Some(paired_descriptor) = self.paired_descriptor
            && paired_descriptor != self.descriptor
        {
            unsafe {
                libc::close(paired_descriptor);
            }
        }

        unsafe {
            libc::close(self.descriptor);
        }
    }
}

/// Finalizer that closes one pipe-backed event token and clears write-descriptor routing state.
#[cfg(all(unix, not(target_os = "linux")))]
#[derive(Debug)]
struct UnixEventPipeFinalizer {
    /// Read descriptor stored in the runtime resource table.
    read_descriptor: c_int,
    /// Write descriptor used for event signal writes.
    write_descriptor: c_int,
}

#[cfg(all(unix, not(target_os = "linux")))]
impl ResourceFinalizer for UnixEventPipeFinalizer {
    /// Close one pipe descriptor pair.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            libc::close(self.write_descriptor);
            libc::close(self.read_descriptor);
        }
    }
}

/// Payload stored for one pipe-backed event token.
#[cfg(all(unix, not(target_os = "linux")))]
#[derive(Debug)]
struct UnixEventPipeResource {
    /// Write descriptor used for event signals.
    write_descriptor: c_int,
}

/// Create one nonblocking close-on-exec pipe pair for events.
#[cfg(all(
    unix,
    not(target_os = "linux"),
    not(any(
        target_vendor = "apple",
        target_os = "aix",
        target_os = "espidf",
        target_os = "haiku",
        target_os = "horizon",
        target_os = "nto"
    ))
))]
fn create_event_pipe() -> RuntimeResult<(c_int, c_int)> {
    // allocate one pipe with atomic nonblocking and close-on-exec flags
    let mut descriptors = [0; 2];
    let result =
        unsafe { libc::pipe2(descriptors.as_mut_ptr(), libc::O_NONBLOCK | libc::O_CLOEXEC) };
    if result < 0 {
        return Err(io_core::io_error_from_errno("pipe2"));
    }

    Ok((descriptors[0], descriptors[1]))
}

/// Create one nonblocking close-on-exec pipe pair for events.
#[cfg(all(
    unix,
    not(target_os = "linux"),
    any(
        target_vendor = "apple",
        target_os = "aix",
        target_os = "espidf",
        target_os = "haiku",
        target_os = "horizon",
        target_os = "nto"
    )
))]
fn create_event_pipe() -> RuntimeResult<(c_int, c_int)> {
    // allocate one anonymous pipe
    let mut descriptors = [0; 2];
    let result = unsafe { libc::pipe(descriptors.as_mut_ptr()) };
    if result < 0 {
        return Err(io_core::io_error_from_errno("pipe"));
    }

    // configure read descriptor status flags
    if let Err(error) = configure_event_pipe_descriptor(descriptors[0]) {
        unsafe {
            libc::close(descriptors[0]);
            libc::close(descriptors[1]);
        }
        return Err(error);
    }

    // configure write descriptor status flags
    if let Err(error) = configure_event_pipe_descriptor(descriptors[1]) {
        unsafe {
            libc::close(descriptors[0]);
            libc::close(descriptors[1]);
        }
        return Err(error);
    }

    Ok((descriptors[0], descriptors[1]))
}

/// Configure one event-pipe descriptor as nonblocking and close-on-exec.
#[cfg(all(
    unix,
    not(target_os = "linux"),
    any(
        target_vendor = "apple",
        target_os = "aix",
        target_os = "espidf",
        target_os = "haiku",
        target_os = "horizon",
        target_os = "nto"
    )
))]
fn configure_event_pipe_descriptor(descriptor: c_int) -> RuntimeResult<()> {
    // set nonblocking mode on the descriptor
    let nonblocking_status = unsafe { libc::fcntl(descriptor, libc::F_SETFL, libc::O_NONBLOCK) };
    if nonblocking_status < 0 {
        return Err(io_core::io_error_from_errno("fcntl"));
    }

    // set close-on-exec mode on the descriptor
    let cloexec_status = unsafe { libc::fcntl(descriptor, libc::F_SETFD, libc::FD_CLOEXEC) };
    if cloexec_status < 0 {
        return Err(io_core::io_error_from_errno("fcntl"));
    }

    Ok(())
}

/// Write one full event payload to one descriptor with interrupt retry handling.
fn write_event_payload(descriptor: c_int, payload: &[u8]) -> RuntimeResult<()> {
    loop {
        // write one event payload into the wakeup descriptor
        let written =
            unsafe { libc::write(descriptor, payload.as_ptr().cast::<c_void>(), payload.len()) };
        if written == payload.len() as isize {
            return Ok(());
        }

        // retry interrupted writes and map all other host failures
        if written < 0 {
            let interrupted = std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR);
            if interrupted {
                continue;
            }

            return Err(io_core::io_error_from_errno("write"));
        }

        return Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoInvalidData),
            None,
            None,
            Some("write".to_string()),
            None,
            "short write while signaling one event token".to_string(),
        ))
        .boxed());
    }
}

/// Create one completion backend for Unix hosts.
pub(crate) fn host_completion_create_proactor(entries: u32) -> RuntimeResult<Box<dyn Proactor>> {
    #[cfg(target_os = "linux")]
    {
        Ok(Box::new(IoUringProactor::with_entries(entries)?))
    }

    #[cfg(all(unix, not(target_os = "linux")))]
    {
        let _ = entries;
        Ok(Box::new(UnixProactor::new()?))
    }
}

/// Execute one generic descriptor fcntl-style operation.
pub(crate) fn host_control_fcntl(
    binding: &BindingCallContext,
    handle: ResourceId,
    command: DescriptorControlCommand,
    argument: u64,
    flags: DescriptorControlFlags,
) -> RuntimeResult<i64> {
    // reject unknown descriptor control flags
    if flags.0 != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "flags must be zero",
        ))
        .boxed());
    }

    // resolve one fd-backed resource from the table
    let fd = binding
        .worker()
        .resources
        .with_entry(handle, |entry| entry.fd())
        .flatten()
        .ok_or_else(|| io_core::io_target_not_found("destack.io.control.fcntl", handle))?;

    // validate command and argument lanes before syscall conversion
    let command = c_int::try_from(command.0).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "command",
            "command must fit one int32 on unix hosts",
        ))
        .boxed()
    })?;
    let argument = c_ulong::try_from(argument).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "argument",
            "argument must fit one c_ulong on unix hosts",
        ))
        .boxed()
    })?;

    // forward the generic fcntl command to the host kernel
    let result = unsafe { libc::fcntl(fd, command, argument) };
    if result == -1 {
        return Err(io_core::io_error_from_errno("fcntl"));
    }

    Ok(result as i64)
}

/// Execute one generic descriptor ioctl-style operation.
pub(crate) fn host_control_ioctl(
    binding: &BindingCallContext,
    handle: ResourceId,
    request: DescriptorRequest,
) -> RuntimeResult<DescriptorResult> {
    // reject unknown descriptor request flags
    if request.flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "request.flags",
            "request.flags must be zero",
        ))
        .boxed());
    }

    // reject oversized output requests
    if request.output_size > io_core::IOCTL_MAX_OUTPUT_BYTES {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "request.outputSize",
            "request.outputSize exceeds the runtime safety limit",
        ))
        .boxed());
    }

    // validate ioctl request code width for this host ABI
    let request_code = HostIoctlRequest::try_from(request.code).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "request.code",
            "request.code must fit one ioctl request type on unix hosts",
        ))
        .boxed()
    })?;

    // decode one request payload from call-scoped native storage
    let input = unsafe { request.input.as_slice()? };
    if input.len() > io_core::IOCTL_MAX_INPUT_BYTES as usize {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "request.input",
            "request.input exceeds the runtime safety limit",
        ))
        .boxed());
    }
    let lane_len = input.len().max(request.output_size as usize);
    let mut lane = vec![0u8; lane_len];
    if !input.is_empty() {
        lane[..input.len()].copy_from_slice(input);
    }

    // resolve one fd-backed resource from the table
    let fd = binding
        .worker()
        .resources
        .with_entry(handle, |entry| entry.fd())
        .flatten()
        .ok_or_else(|| io_core::io_target_not_found("destack.io.control.ioctl", handle))?;

    // forward the generic ioctl command to the host kernel
    let pointer = if lane.is_empty() {
        std::ptr::null_mut()
    } else {
        lane.as_mut_ptr().cast::<c_void>()
    };
    let result = unsafe { libc::ioctl(fd, request_code, pointer) };
    if result == -1 {
        return Err(io_core::io_error_from_errno("ioctl"));
    }

    // encode the fixed-size output lane requested by the caller
    let output_len = request.output_size as usize;
    let output = binding.store_slice_copy(&lane[..output_len]);

    Ok(DescriptorResult {
        return_value: result as i64,
        output,
    })
}

/// Map one io poll backend selector for Unix hosts.
pub(crate) const fn host_map_poll_backend(backend: PollBackend) -> HostPollerBackend {
    match backend {
        PollBackend::Auto => HostPollerBackend::Auto,
        PollBackend::Epoll => HostPollerBackend::Epoll,
        PollBackend::Kqueue => HostPollerBackend::Kqueue,
        PollBackend::Poll => HostPollerBackend::Poll,
    }
}

/// Resolve one poll target resource into one platform handle.
pub(crate) fn host_poll_resolve_target_handle(
    binding: &BindingCallContext,
    target: ResourceId,
) -> RuntimeResult<PlatformHandle> {
    // resolve one runtime target entry
    let resolved = binding
        .worker()
        .resources
        .with_entry(target, |entry| entry.fd().map(PlatformHandle::from_raw_fd));

    // reject unknown resources first
    let Some(handle) = resolved else {
        return Err(io_core::io_target_not_found(
            "destack.io.poll.target",
            target,
        ));
    };

    // reject resources without pollable host handles
    let Some(handle) = handle else {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "target",
            "target resource is not pollable on this platform",
        ))
        .boxed());
    };

    Ok(handle)
}

/// Resolve one completion target resource into one platform handle.
pub(crate) fn host_completion_resolve_target_handle(
    binding: &BindingCallContext,
    target: ResourceId,
    operation: &'static str,
) -> RuntimeResult<PlatformHandle> {
    // resolve one runtime target entry
    let resolved = binding
        .worker()
        .resources
        .with_entry(target, |entry| entry.fd().map(PlatformHandle::from_raw_fd));

    // reject unknown targets first
    let Some(handle) = resolved else {
        return Err(io_core::io_target_not_found(operation, target));
    };

    // reject targets without host handles
    let Some(handle) = handle else {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "target",
            "target resource is not backed by one host handle",
        ))
        .boxed());
    };

    Ok(handle)
}

/// Register one accepted socket handle into the runtime resource table.
pub(crate) fn host_completion_register_accepted_handle(
    binding: &BindingCallContext,
    handle: PlatformHandle,
) -> RuntimeResult<i64> {
    let descriptor = handle.as_raw_fd();
    let entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(descriptor)
        .with_label("io.completion.accept")
        .with_finalizer(UnixDescriptorFinalizer {
            descriptor,
            paired_descriptor: None,
        });
    let resource_id =
        binding
            .worker()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));
    Ok(resource_id.0 as i64)
}

/// Open one event token on Unix hosts.
pub(crate) fn host_event_open(
    binding: &BindingCallContext,
    initial: u64,
) -> RuntimeResult<EventToken> {
    #[cfg(target_os = "linux")]
    {
        // allocate one non-blocking close-on-exec eventfd
        let descriptor = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK | libc::EFD_CLOEXEC) };
        if descriptor < 0 {
            return Err(io_core::io_error_from_errno("eventfd"));
        }

        // preload one initial value when requested
        if initial != 0 {
            let payload = initial.to_ne_bytes();
            if let Err(error) = write_event_payload(descriptor, &payload) {
                unsafe {
                    libc::close(descriptor);
                }
                return Err(error);
            }
        }

        // store one runtime event token resource
        let entry = ResourceEntry::new(ResourceKind::Event)
            .with_label(io_core::EVENT_RESOURCE_LABEL)
            .with_fd(descriptor)
            .with_finalizer(UnixDescriptorFinalizer {
                descriptor,
                paired_descriptor: None,
            });
        let resource_id =
            binding
                .worker()
                .resources
                .insert(binding.world(), entry, Some(binding.engine()));
        Ok(EventToken(resource_id.0))
    }

    #[cfg(all(unix, not(target_os = "linux")))]
    {
        // allocate one pipe-backed event token pair
        let (read_descriptor, write_descriptor) = create_event_pipe()?;

        // preload one initial value when requested
        if initial != 0 {
            let payload = initial.to_ne_bytes();
            if let Err(error) = write_event_payload(write_descriptor, &payload) {
                unsafe {
                    libc::close(read_descriptor);
                    libc::close(write_descriptor);
                }
                return Err(error);
            }
        }

        // store one runtime event token resource
        let entry = ResourceEntry::new(ResourceKind::Event)
            .with_label(io_core::EVENT_RESOURCE_LABEL)
            .with_fd(read_descriptor)
            .with_payload(UnixEventPipeResource { write_descriptor })
            .with_finalizer(UnixEventPipeFinalizer {
                read_descriptor,
                write_descriptor,
            });
        let resource_id =
            binding
                .worker()
                .resources
                .insert(binding.world(), entry, Some(binding.engine()));

        Ok(EventToken(resource_id.0))
    }
}

/// Close one event token on Unix hosts.
pub(crate) fn host_event_close(
    binding: &BindingCallContext,
    token: EventToken,
) -> RuntimeResult<()> {
    // remove one token resource from the runtime table
    let removed = binding.worker().resources.remove_and_finalize(
        binding.world(),
        ResourceId(token.0),
        Some(binding.engine()),
    );
    if !removed {
        return Err(io_core::event_not_found("destack.io.event.close", token));
    }

    Ok(())
}

/// Signal one event token on Unix hosts.
pub(crate) fn host_event_signal(
    binding: &BindingCallContext,
    token: EventToken,
    value: u64,
) -> RuntimeResult<()> {
    // resolve one descriptor from the token resource
    #[cfg(target_os = "linux")]
    let descriptor = {
        binding
            .worker()
            .resources
            .with_entry(ResourceId(token.0), |entry| entry.fd())
            .flatten()
            .ok_or_else(|| io_core::event_not_found("destack.io.event.signal", token))?
    };
    #[cfg(all(unix, not(target_os = "linux")))]
    let descriptor = binding
        .worker()
        .resources
        .with_entry(ResourceId(token.0), |entry| {
            if entry.kind != ResourceKind::Event {
                return None;
            }

            entry
                .payload
                .as_ref()
                .and_then(|payload| payload.downcast_ref::<UnixEventPipeResource>())
                .map(|resource| resource.write_descriptor)
        })
        .flatten()
        .ok_or_else(|| io_core::event_not_found("destack.io.event.signal", token))?;

    // write one signal payload
    let payload = value.to_ne_bytes();
    write_event_payload(descriptor, &payload)?;

    Ok(())
}
