use core::ffi::c_void;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::Networking::WinSock::{SOCKET, SOCKET_ERROR, WSAIoctl, closesocket};
use windows_sys::Win32::System::IO::DeviceIoControl;
use windows_sys::Win32::System::Threading::{CreateEventW, SetEvent};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::io::{
    DescriptorControlCommand, DescriptorControlFlags, DescriptorRequest, DescriptorResult,
    EventToken, PollBackend, core as io_core,
};
use crate::platform::proactor::Proactor;
use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceKind};
use crate::platform::{IocpProactor, PlatformError, ResourceId, core as core_platform};
use crate::runtime::BindingCallContext;
use crate::runtime::poller::{HostPollerBackend, PlatformHandle};

/// Return one standardized null-pointer error for output arguments.
pub(super) fn require_out<T>(out: *mut T) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    Ok(())
}

/// Finalizer that closes one owned handle.
#[derive(Debug)]
struct WindowsHandleFinalizer {
    /// Raw host handle to close.
    handle: HANDLE,
}

impl ResourceFinalizer for WindowsHandleFinalizer {
    /// Close one owned host handle.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

/// Finalizer that closes one owned socket.
#[derive(Debug)]
struct WindowsSocketFinalizer {
    /// Raw socket to close.
    socket: SOCKET,
}

impl ResourceFinalizer for WindowsSocketFinalizer {
    /// Close one owned socket.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            closesocket(self.socket);
        }
    }
}

/// Create one completion backend for Windows hosts.
pub(crate) fn host_completion_create_proactor(entries: u32) -> RuntimeResult<Box<dyn Proactor>> {
    let _ = entries;
    Ok(Box::new(IocpProactor::new()?))
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

    // reject fcntl controls on windows hosts
    let _ = (binding, handle, command, argument);
    Err(RuntimeError::from(PlatformError::not_supported("destack.io.control.fcntl")).boxed())
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

    // decode one operation code for Win32 APIs
    let control_code = u32::try_from(request.code).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "request.code",
            "request.code must fit in one uint32 on Windows",
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
    let input_len = u32::try_from(input.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "request.input",
            "request.input length must fit in one uint32 on Windows",
        ))
        .boxed()
    })?;
    let mut input_lane = input.to_vec();
    let mut output_lane = vec![0u8; request.output_size as usize];

    // resolve one host-backed resource entry
    let entry = binding
        .worker()
        .resources
        .with_entry(handle, |entry| (entry.socket(), entry.handle()))
        .ok_or_else(|| io_core::io_target_not_found("destack.io.control.ioctl", handle))?;
    let (socket, host_handle) = entry;

    // dispatch to winsock when the resource is socket-backed
    if let Some(socket) = socket {
        core_platform::ensure_winsock()?;

        let mut bytes_returned = 0u32;
        let input_ptr = if input_lane.is_empty() {
            std::ptr::null_mut()
        } else {
            input_lane.as_mut_ptr().cast::<c_void>()
        };
        let output_ptr = if output_lane.is_empty() {
            std::ptr::null_mut()
        } else {
            output_lane.as_mut_ptr().cast::<c_void>()
        };
        let result = unsafe {
            WSAIoctl(
                socket as SOCKET,
                control_code,
                input_ptr,
                input_len,
                output_ptr,
                request.output_size,
                &mut bytes_returned,
                std::ptr::null_mut(),
                None,
            )
        };
        if result == SOCKET_ERROR {
            let code = core_platform::last_wsa_error_code();
            return Err(core_platform::net_error_with_code("WSAIoctl", code));
        }

        let output_len = (bytes_returned as usize).min(output_lane.len());
        let output = binding.store_slice_copy(&output_lane[..output_len]);
        return Ok(DescriptorResult {
            return_value: 0,
            output,
        });
    }

    // dispatch to DeviceIoControl when the resource is handle-backed
    if let Some(host_handle) = host_handle {
        let mut bytes_returned = 0u32;
        let input_ptr = if input_lane.is_empty() {
            std::ptr::null_mut()
        } else {
            input_lane.as_mut_ptr().cast::<c_void>()
        };
        let output_ptr = if output_lane.is_empty() {
            std::ptr::null_mut()
        } else {
            output_lane.as_mut_ptr().cast::<c_void>()
        };
        let ok = unsafe {
            DeviceIoControl(
                host_handle as HANDLE,
                control_code,
                input_ptr,
                input_len,
                output_ptr,
                request.output_size,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(core_platform::io_error("DeviceIoControl"));
        }

        let output_len = (bytes_returned as usize).min(output_lane.len());
        let output = binding.store_slice_copy(&output_lane[..output_len]);
        return Ok(DescriptorResult {
            return_value: 0,
            output,
        });
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "handle",
        "target resource is not backed by one socket or handle",
    ))
    .boxed())
}

/// Map one io poll backend selector for Windows hosts.
pub(crate) const fn host_map_poll_backend(backend: PollBackend) -> HostPollerBackend {
    match backend {
        PollBackend::Auto => HostPollerBackend::Auto,
        PollBackend::Epoll => HostPollerBackend::Epoll,
        PollBackend::Kqueue => HostPollerBackend::Kqueue,
        PollBackend::Poll => HostPollerBackend::Windows,
    }
}

/// Resolve one poll target resource into one platform handle.
pub(crate) fn host_poll_resolve_target_handle(
    binding: &BindingCallContext,
    target: ResourceId,
) -> RuntimeResult<PlatformHandle> {
    // resolve one runtime target entry
    let resolved = binding.worker().resources.with_entry(target, |entry| {
        entry.socket().map(PlatformHandle::from_raw_socket)
    });

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
    let resolved = binding.worker().resources.with_entry(target, |entry| {
        if let Some(socket) = entry.socket() {
            return Some(PlatformHandle::from_raw_socket(socket));
        }

        entry.handle().map(|handle| PlatformHandle(handle as u64))
    });

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
    let socket = handle.as_raw_socket();
    let entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(socket)
        .with_label("io.completion.accept")
        .with_finalizer(WindowsSocketFinalizer {
            socket: socket as SOCKET,
        });
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));
    Ok(resource_id.0 as i64)
}

/// Open one event token on Windows hosts.
pub(crate) fn host_event_open(
    binding: &BindingCallContext,
    initial: u64,
) -> RuntimeResult<EventToken> {
    // allocate one manual-reset event object
    let handle = unsafe {
        CreateEventW(
            std::ptr::null(),
            1,
            if initial == 0 { 0 } else { 1 },
            std::ptr::null(),
        )
    };
    if handle == 0 {
        return Err(core_platform::io_error("CreateEventW"));
    }

    // store one runtime event token resource
    let entry = ResourceEntry::new(ResourceKind::Event)
        .with_label(io_core::EVENT_RESOURCE_LABEL)
        .with_handle(handle as _)
        .with_finalizer(WindowsHandleFinalizer { handle });
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));
    Ok(EventToken(resource_id.0))
}

/// Close one event token on Windows hosts.
pub(crate) fn host_event_close(
    binding: &BindingCallContext,
    token: EventToken,
) -> RuntimeResult<()> {
    // remove one token resource from the runtime table
    let removed = binding.worker().resources.remove_and_finalize(
        &binding.world(),
        ResourceId(token.0),
        Some(binding.engine()),
    );
    if !removed {
        return Err(io_core::event_not_found("destack.io.event.close", token));
    }

    Ok(())
}

/// Signal one event token on Windows hosts.
pub(crate) fn host_event_signal(
    binding: &BindingCallContext,
    token: EventToken,
    value: u64,
) -> RuntimeResult<()> {
    let _ = value;

    // resolve one event handle from the token resource
    let handle = binding
        .worker()
        .resources
        .with_entry(ResourceId(token.0), |entry| {
            if entry.label.as_deref() != Some(io_core::EVENT_RESOURCE_LABEL) {
                return None;
            }

            entry.handle()
        })
        .flatten()
        .ok_or_else(|| io_core::event_not_found("destack.io.event.signal", token))?;

    // signal the event object
    let ok = unsafe { SetEvent(handle as HANDLE) };
    if ok == 0 {
        return Err(core_platform::io_error("SetEvent"));
    }

    Ok(())
}
