#![allow(dead_code)]

use std::sync::Arc;

use parking_lot::RwLock;
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_BROKEN_PIPE, ERROR_CALL_NOT_IMPLEMENTED,
    ERROR_INVALID_HANDLE, ERROR_INVALID_PARAMETER, ERROR_NO_DATA, ERROR_OLD_WIN_VERSION,
    ERROR_PROC_NOT_FOUND, HANDLE,
};
use windows_sys::Win32::System::Console::{ClosePseudoConsole, HPCON};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceId, ResourceKind};
use crate::platform::tty::core::{PTY_RESOURCE_LABEL, TTY_RESOURCE_LABEL, invalid_tty_handle};
use crate::platform::tty::{PtyPair, TtySize};
use crate::platform::{PlatformError, core as core_platform, resource};
use crate::runtime::BindingCallContext;

/// E_NOTIMPL hresult value.
const HRESULT_NOT_IMPLEMENTED: i32 = 0x8000_4001u32 as i32;

/// Finalizer that closes one duplicated windows handle.
#[derive(Debug)]
pub(super) struct WindowsHandleFinalizer {
    /// Handle to close.
    pub(super) handle: HANDLE,
}

impl ResourceFinalizer for WindowsHandleFinalizer {
    /// Close the handle during resource finalization.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

/// Finalizer that closes one windows pseudo console handle.
#[derive(Debug)]
pub(super) struct WindowsPseudoConsoleFinalizer {
    /// Pseudo console handle to close.
    pub(super) pseudo_console: HPCON,
}

impl ResourceFinalizer for WindowsPseudoConsoleFinalizer {
    /// Close the pseudo console during resource finalization.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            ClosePseudoConsole(self.pseudo_console);
        }
    }
}

/// Finalizer that closes one pair of worker pipe handles.
#[derive(Debug)]
pub(super) struct WindowsPipePairFinalizer {
    /// Read side to close.
    pub(super) read_handle: HANDLE,
    /// Write side to close.
    pub(super) write_handle: HANDLE,
}

impl ResourceFinalizer for WindowsPipePairFinalizer {
    /// Close both pipe endpoints during resource finalization.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            CloseHandle(self.read_handle);
            CloseHandle(self.write_handle);
        }
    }
}

/// Resource payload for one pty-backed tty worker binding.
#[derive(Debug)]
pub(super) struct WindowsTtyBinding {
    /// Pipe endpoint used by tty reads.
    pub(super) read_handle: HANDLE,
    /// Pipe endpoint used by tty writes.
    pub(super) write_handle: HANDLE,
    /// Optional pseudo-console handle used for resize operations.
    pub(super) pseudo_console: Option<HPCON>,
    /// Cached tty size snapshot for this worker.
    pub(super) size: RwLock<TtySize>,
}

/// Build one mapped windows I/O error from one explicit code.
pub(super) fn io_error_with_code(
    operation: &'static str,
    syscall: &'static str,
    code: u32,
    message: &str,
) -> Box<RuntimeError> {
    let platform_code = if code == ERROR_INVALID_HANDLE || code == ERROR_INVALID_PARAMETER {
        Some(PlatformErrorCode::IoNotFound)
    } else if code == ERROR_ACCESS_DENIED {
        Some(PlatformErrorCode::IoPermissionDenied)
    } else if code == ERROR_BROKEN_PIPE {
        Some(PlatformErrorCode::IoBrokenPipe)
    } else if code == ERROR_NO_DATA {
        Some(PlatformErrorCode::IoWouldBlock)
    } else {
        None
    };

    RuntimeError::from(PlatformError::io_with(
        platform_code,
        None,
        Some(code as i32),
        Some(operation.to_string()),
        None,
        format!("{syscall} failed: {message} ({code})"),
    ))
    .boxed()
}

/// Build one mapped windows I/O error from `GetLastError`.
pub(super) fn io_error(
    operation: &'static str,
    syscall: &'static str,
    message: &str,
) -> Box<RuntimeError> {
    let code = core_platform::last_error_code() as u32;
    io_error_with_code(operation, syscall, code, message)
}

/// Extract one Win32 error code from one HRESULT when available.
pub(super) fn win32_code_from_hresult(status: i32) -> Option<u32> {
    let status = status as u32;
    if status & 0xFFFF_0000 == 0x8007_0000 {
        return Some(status & 0x0000_FFFF);
    }

    None
}

/// Return whether one Win32 code means ConPTY is unavailable.
fn conpty_not_supported_code(code: u32) -> bool {
    code == ERROR_CALL_NOT_IMPLEMENTED
        || code == ERROR_PROC_NOT_FOUND
        || code == ERROR_OLD_WIN_VERSION
}

/// Build one mapped windows I/O error from one HRESULT code.
pub(super) fn io_error_from_hresult(
    operation: &'static str,
    syscall: &'static str,
    status: i32,
    message: &str,
) -> Box<RuntimeError> {
    if let Some(code) = win32_code_from_hresult(status) {
        return io_error_with_code(operation, syscall, code, message);
    }

    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        Some(status),
        Some(operation.to_string()),
        None,
        format!(
            "{syscall} failed: {message} (hresult 0x{:08x})",
            status as u32
        ),
    ))
    .boxed()
}

/// Build one mapped ConPTY error from one HRESULT code.
pub(super) fn conpty_error_from_hresult(
    operation: &'static str,
    syscall: &'static str,
    status: i32,
    message: &str,
) -> Box<RuntimeError> {
    if status == HRESULT_NOT_IMPLEMENTED {
        return RuntimeError::from(PlatformError::not_supported(operation)).boxed();
    }

    if let Some(code) = win32_code_from_hresult(status)
        && conpty_not_supported_code(code)
    {
        return RuntimeError::from(PlatformError::not_supported(operation)).boxed();
    }

    io_error_from_hresult(operation, syscall, status, message)
}

/// Resolve one tty handle into one typed worker binding.
pub(super) fn tty_binding(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    operation: &'static str,
) -> RuntimeResult<Option<Arc<WindowsTtyBinding>>> {
    let resolved = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Tty {
                return None;
            }

            entry.payload_cloned::<Arc<WindowsTtyBinding>>()
        })
        .flatten();

    if resolved.is_none() {
        let kind = binding
            .worker()
            .resources
            .with_entry(handle.0, |entry| entry.kind);
        if let Some(kind) = kind
            && kind != ResourceKind::Tty
        {
            return Err(invalid_tty_handle(operation));
        }
    }

    Ok(resolved)
}

/// Resolve one tty handle into one raw windows handle.
pub(super) fn tty_handle(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    operation: &'static str,
) -> RuntimeResult<HANDLE> {
    let resolved = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Tty {
                return None;
            }

            entry.handle().map(|value| value as HANDLE)
        })
        .flatten()
        .ok_or_else(|| invalid_tty_handle(operation))?;

    Ok(resolved)
}

/// Validate one terminal dimension as one i16 console value.
pub(super) fn validate_console_dimension(value: u32, field: &str) -> RuntimeResult<i16> {
    if value == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "dimension must be greater than zero",
        ))
        .boxed());
    }

    i16::try_from(value).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "dimension exceeds Win32 console range",
        ))
        .boxed()
    })
}

/// Register one windows pseudo-terminal pair in the resource table.
pub(super) fn register_pty_pair(
    binding: &BindingCallContext,
    pseudo_console: HPCON,
    read_handle: HANDLE,
    write_handle: HANDLE,
    initial_size: TtySize,
) -> PtyPair {
    let resolved_binding = Arc::new(WindowsTtyBinding {
        read_handle,
        write_handle,
        pseudo_console: Some(pseudo_console),
        size: RwLock::new(initial_size),
    });

    let controller_entry = ResourceEntry::new(ResourceKind::Pty)
        .with_label(PTY_RESOURCE_LABEL)
        .with_finalizer(WindowsPseudoConsoleFinalizer { pseudo_console });
    let controller_id = binding.worker().resources.insert(
        binding.world(),
        controller_entry,
        Some(binding.engine()),
    );

    let worker_entry = ResourceEntry::new(ResourceKind::Tty)
        .with_label(TTY_RESOURCE_LABEL)
        .with_payload(resolved_binding)
        .with_finalizer(WindowsPipePairFinalizer {
            read_handle,
            write_handle,
        });
    let worker_id =
        binding
            .worker()
            .resources
            .insert(binding.world(), worker_entry, Some(binding.engine()));

    PtyPair {
        controller: resource::PtyHandle(controller_id),
        worker: resource::TtyHandle(worker_id),
    }
}

/// Validate one buffer length for Win32 file calls.
pub(super) fn validate_buffer_length(length: usize, field: &str) -> RuntimeResult<u32> {
    u32::try_from(length).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "buffer length exceeds Win32 I/O range",
        ))
        .boxed()
    })
}
