use super::super::core::overflow_event;
use super::config::*;
use super::core::*;
use super::identity::*;
use super::state::*;

/// One owned overlapped I/O state for one windows serial operation.
pub(super) struct WindowsSerialOverlappedIo {
    /// Event handle used to signal completion.
    event_handle: HANDLE,
    /// Overlapped header passed into Win32 I/O calls.
    pub(super) overlapped: OVERLAPPED,
}

impl WindowsSerialOverlappedIo {
    /// Build one fresh overlapped I/O state.
    pub(super) fn new(
        operation: &'static str,
        syscall: &'static str,
    ) -> RuntimeResult<WindowsSerialOverlappedIo> {
        let event_handle = unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) };
        if event_handle == 0 {
            return Err(serial_io_error_with_code(
                operation,
                syscall,
                core_platform::last_error_code() as u32,
                "failed to create overlapped event handle",
            ));
        }

        // safety: zeroed OVERLAPPED is valid before first submission
        let mut overlapped = unsafe { std::mem::zeroed::<OVERLAPPED>() };
        overlapped.hEvent = event_handle;

        Ok(Self {
            event_handle,
            overlapped,
        })
    }

    /// Reset one overlapped state before reusing it.
    pub(super) fn reset(
        &mut self,
        operation: &'static str,
        syscall: &'static str,
    ) -> RuntimeResult<()> {
        let status = unsafe { ResetEvent(self.event_handle) };
        if status == 0 {
            return Err(serial_io_error_with_code(
                operation,
                syscall,
                core_platform::last_error_code() as u32,
                "failed to reset overlapped event handle",
            ));
        }

        // safety: zeroed OVERLAPPED is valid before the next submission
        self.overlapped = unsafe { std::mem::zeroed::<OVERLAPPED>() };
        self.overlapped.hEvent = self.event_handle;

        Ok(())
    }

    /// Return the underlying wait event handle.
    pub(super) fn event_handle(&self) -> HANDLE {
        self.event_handle
    }
}

impl Drop for WindowsSerialOverlappedIo {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.event_handle);
        }
    }
}

// NOTE #Architecture: this wrapper owns one event handle and one OVERLAPPED header.
// Access is synchronized by the caller, and Win32 allows cross-thread completion on this state.
unsafe impl Send for WindowsSerialOverlappedIo {}

// NOTE #Architecture: shared references are only used behind external synchronization.
unsafe impl Sync for WindowsSerialOverlappedIo {}

/// One submitted windows serial event wait state.
pub(super) enum WindowsSerialWaitSubmission {
    /// The wait completed immediately.
    Ready,
    /// The wait is still pending on the overlapped event handle.
    Pending,
}

/// Return one read-would-block error for one operation.
pub(super) fn serial_read_would_block(operation: &'static str) -> Box<RuntimeError> {
    core_platform::io_would_block(operation, "serial read would block")
}

/// Return one event-would-block error for one operation.
pub(super) fn serial_event_would_block(operation: &'static str) -> Box<RuntimeError> {
    core_platform::io_would_block(operation, "serial event would block")
}

/// Return one write-would-block error for one operation.
pub(super) fn serial_write_would_block(operation: &'static str) -> Box<RuntimeError> {
    core_platform::io_would_block(operation, "serial write would block")
}

/// Queue one windows serial event record.
fn queue_serial_event(resource: &WindowsSerialPortResource, record: WindowsSerialEventRecord) {
    // gate read-ready records so the queue tracks transitions, not level-trigger churn
    if let WindowsSerialEventRecord::ReadReady { .. } = record
        && !should_queue_read_ready(resource)
    {
        return;
    }

    resource.event_queue.push_drop_oldest(record);
}

/// Queue one backend error record and stop the event loop.
fn queue_serial_error(
    resource: &WindowsSerialPortResource,
    kind: SerialErrorKind,
    backend_code: Option<i32>,
    backend_detail: Option<i32>,
) {
    queue_serial_event(
        resource,
        WindowsSerialEventRecord::Error {
            kind,
            backend_code,
            backend_detail,
        },
    );
}

/// Submit one overlapped `WaitCommEvent` request.
pub(super) fn submit_serial_event_wait(
    resource: &WindowsSerialPortResource,
    event_mask: &mut u32,
    overlapped_io: &mut WindowsSerialOverlappedIo,
) -> Result<WindowsSerialWaitSubmission, u32> {
    if overlapped_io
        .reset("destack.device.serial.open", "WaitCommEvent")
        .is_err()
    {
        let code = core_platform::last_error_code() as u32;

        return Err(code);
    }

    let wait_status =
        unsafe { WaitCommEvent(resource.handle, event_mask, &mut overlapped_io.overlapped) };
    if wait_status != 0 {
        return Ok(WindowsSerialWaitSubmission::Ready);
    }

    let code = core_platform::last_error_code() as u32;
    if code == ERROR_IO_PENDING {
        return Ok(WindowsSerialWaitSubmission::Pending);
    }

    Err(code)
}

/// Process one completed windows serial event mask.
pub(super) fn process_serial_event_mask(
    resource: &WindowsSerialPortResource,
    event_mask: u32,
) -> bool {
    let mut read_ready = false;
    let mut available_bytes = None;

    // data and line errors
    if event_mask & (EV_RXCHAR | EV_ERR | EV_BREAK) != 0 {
        match clear_comm_status_raw(resource.handle) {
            Ok((errors, input_bytes, _)) => {
                if let Some(pending_error) = pending_error_from_flags(errors) {
                    queue_serial_error(
                        resource,
                        pending_error.kind,
                        pending_error.backend_code,
                        pending_error.backend_detail,
                    );
                }

                if event_mask & EV_RXCHAR != 0 {
                    read_ready = true;
                    available_bytes = (input_bytes != 0).then_some(input_bytes);
                }
            }
            Err(code) => {
                if is_disconnected_code(code) {
                    queue_serial_event(resource, WindowsSerialEventRecord::Disconnected);
                } else {
                    queue_serial_error(resource, SerialErrorKind::Unknown, Some(code as i32), None);
                }

                return false;
            }
        }
    }

    // modem lines
    if event_mask & (EV_CTS | EV_DSR | EV_RING | EV_RLSD) != 0 {
        match modem_status_raw(resource.handle) {
            Ok(status) => {
                let signals = decode_input_signals(status);
                if let Some(signals) = update_modem_status(resource, signals) {
                    queue_serial_event(
                        resource,
                        WindowsSerialEventRecord::ModemStatusChanged { signals },
                    );
                }
            }
            Err(code) if is_optional_signal_code(code) => {}
            Err(code) => {
                if is_disconnected_code(code) {
                    queue_serial_event(resource, WindowsSerialEventRecord::Disconnected);
                } else {
                    queue_serial_error(resource, SerialErrorKind::Unknown, Some(code as i32), None);
                }

                return false;
            }
        }
    }

    // read readiness
    if read_ready {
        queue_serial_event(
            resource,
            WindowsSerialEventRecord::ReadReady { available_bytes },
        );
    }

    true
}

/// Convert one timeout in nanoseconds into one Win32 wait timeout in milliseconds.
fn timeout_to_wait_milliseconds(timeout_ns: u64) -> u32 {
    if timeout_ns == u64::MAX {
        return u32::MAX;
    }

    let milliseconds = timeout_ns.div_ceil(1_000_000);
    if milliseconds > u64::from(u32::MAX - 1) {
        return u32::MAX - 1;
    }

    milliseconds as u32
}

/// Cancel one pending overlapped I/O request and wait for teardown.
fn cancel_pending_overlapped_io(
    handle: HANDLE,
    overlapped_io: &mut WindowsSerialOverlappedIo,
    operation: &'static str,
    syscall: &'static str,
) -> RuntimeResult<()> {
    let cancel_status = unsafe { CancelIoEx(handle, &overlapped_io.overlapped) };
    if cancel_status == 0 {
        let code = core_platform::last_error_code() as u32;
        if code != ERROR_NOT_FOUND {
            return Err(serial_io_error_with_code(
                operation,
                "CancelIoEx",
                code,
                "failed to cancel overlapped serial I/O",
            ));
        }
    }

    let mut transferred = 0u32;
    let completion_status = unsafe {
        GetOverlappedResultEx(
            handle,
            &overlapped_io.overlapped,
            &mut transferred,
            INFINITE,
            0,
        )
    };
    if completion_status == 0 {
        let code = core_platform::last_error_code() as u32;
        if code != ERROR_OPERATION_ABORTED {
            return Err(serial_io_error_with_code(
                operation,
                syscall,
                code,
                "failed to retire canceled overlapped serial I/O",
            ));
        }
    }

    Ok(())
}

/// Wait for one overlapped serial I/O request to complete.
fn wait_for_overlapped_io(
    handle: HANDLE,
    overlapped_io: &mut WindowsSerialOverlappedIo,
    timeout_ns: u64,
    operation: &'static str,
    syscall: &'static str,
    would_block: fn(&'static str) -> Box<RuntimeError>,
) -> RuntimeResult<u32> {
    let timeout_milliseconds = timeout_to_wait_milliseconds(timeout_ns);
    let mut transferred = 0u32;
    let status = unsafe {
        GetOverlappedResultEx(
            handle,
            &overlapped_io.overlapped,
            &mut transferred,
            timeout_milliseconds,
            0,
        )
    };
    if status != 0 {
        return Ok(transferred);
    }

    let code = core_platform::last_error_code() as u32;

    // timeout: cancel the request before reporting would-block
    if code == WAIT_TIMEOUT {
        cancel_pending_overlapped_io(handle, overlapped_io, operation, syscall)?;

        return Err(would_block(operation));
    }

    Err(serial_io_error_with_code(
        operation,
        syscall,
        code,
        "failed to wait for overlapped serial I/O",
    ))
}

/// Read one serial buffer using one timeout-aware windows handle.
pub(super) fn read_serial_bytes(
    resource: &WindowsSerialPortResource,
    buffer: NativeSlice<u8>,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<u64> {
    let buffer = unsafe { buffer.as_mut_slice()? };
    if buffer.is_empty() {
        return Ok(0);
    }

    // rearm read-ready publication before attempting a new read
    mark_read_ready_queued(resource, false);

    let length = u32::try_from(buffer.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "buffer",
            "buffer length exceeds windows u32 range",
        ))
        .boxed()
    })?;

    // retain line errors before issuing the read
    if let Ok((errors, _, _)) = clear_comm_status_raw(resource.handle) {
        if let Some(pending_error) = pending_error_from_flags(errors) {
            queue_serial_error(
                resource,
                pending_error.kind,
                pending_error.backend_code,
                pending_error.backend_detail,
            );
        }
    }

    let mut overlapped_io = WindowsSerialOverlappedIo::new(operation, "ReadFile")?;
    let read_status = unsafe {
        ReadFile(
            resource.handle,
            buffer.as_mut_ptr().cast(),
            length,
            std::ptr::null_mut(),
            &mut overlapped_io.overlapped,
        )
    };
    if read_status == 0 {
        let code = core_platform::last_error_code() as u32;
        if code != ERROR_IO_PENDING {
            if code == ERROR_SEM_TIMEOUT {
                return Err(serial_read_would_block(operation));
            }

            return Err(serial_io_error_with_code(
                operation,
                "ReadFile",
                code,
                "failed to submit serial read",
            ));
        }
    }

    let read = wait_for_overlapped_io(
        resource.handle,
        &mut overlapped_io,
        timeout_ns,
        operation,
        "ReadFile",
        serial_read_would_block,
    )?;
    if read == 0 {
        return Err(serial_read_would_block(operation));
    }

    Ok(u64::from(read))
}

/// Read one serial buffer without blocking.
pub(super) fn try_read_serial_bytes(
    resource: &WindowsSerialPortResource,
    buffer: NativeSlice<u8>,
    operation: &'static str,
) -> RuntimeResult<u64> {
    let buffer = unsafe { buffer.as_mut_slice()? };
    if buffer.is_empty() {
        return Ok(0);
    }

    // rearm read-ready publication before attempting a new read
    mark_read_ready_queued(resource, false);

    let length = u32::try_from(buffer.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "buffer",
            "buffer length exceeds windows u32 range",
        ))
        .boxed()
    })?;

    let (errors, available_bytes, _) = match clear_comm_status_raw(resource.handle) {
        Ok(status) => status,
        Err(code) => {
            return Err(serial_io_error_with_code(
                operation,
                "ClearCommError",
                code,
                "failed to query serial read status",
            ));
        }
    };
    if let Some(pending_error) = pending_error_from_flags(errors) {
        queue_serial_error(
            resource,
            pending_error.kind,
            pending_error.backend_code,
            pending_error.backend_detail,
        );
    }
    if available_bytes == 0 {
        return Err(serial_read_would_block(operation));
    }

    let mut overlapped_io = WindowsSerialOverlappedIo::new(operation, "ReadFile")?;
    let status = unsafe {
        ReadFile(
            resource.handle,
            buffer.as_mut_ptr().cast(),
            length,
            std::ptr::null_mut(),
            &mut overlapped_io.overlapped,
        )
    };
    if status == 0 {
        let code = core_platform::last_error_code() as u32;
        if code != ERROR_IO_PENDING {
            if code == ERROR_SEM_TIMEOUT {
                return Err(serial_read_would_block(operation));
            }

            return Err(serial_io_error_with_code(
                operation,
                "ReadFile",
                code,
                "failed to submit serial read",
            ));
        }
    }

    let read = wait_for_overlapped_io(
        resource.handle,
        &mut overlapped_io,
        0,
        operation,
        "ReadFile",
        serial_read_would_block,
    )?;
    if read == 0 {
        return Err(serial_read_would_block(operation));
    }

    Ok(u64::from(read))
}

/// Write one serial buffer using one timeout-aware windows handle.
pub(super) fn write_serial_bytes(
    resource: &WindowsSerialPortResource,
    data: NativeSlice<u8>,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<u64> {
    let data = unsafe { data.as_slice()? };
    if data.is_empty() {
        return Ok(0);
    }

    let length = u32::try_from(data.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "data",
            "data length exceeds windows u32 range",
        ))
        .boxed()
    })?;

    let _operation_lock = resource.operation_lock.lock();
    let mut overlapped_io = WindowsSerialOverlappedIo::new(operation, "WriteFile")?;
    let status = unsafe {
        WriteFile(
            resource.handle,
            data.as_ptr().cast(),
            length,
            std::ptr::null_mut(),
            &mut overlapped_io.overlapped,
        )
    };
    if status == 0 {
        let code = core_platform::last_error_code() as u32;
        if code != ERROR_IO_PENDING {
            if code == ERROR_SEM_TIMEOUT {
                return Err(serial_write_would_block(operation));
            }

            return Err(serial_io_error_with_code(
                operation,
                "WriteFile",
                code,
                "failed to submit serial write",
            ));
        }
    }

    let written = wait_for_overlapped_io(
        resource.handle,
        &mut overlapped_io,
        timeout_ns,
        operation,
        "WriteFile",
        serial_write_would_block,
    )?;
    if written == 0 {
        return Err(serial_write_would_block(operation));
    }

    Ok(u64::from(written))
}

/// Wait for one serial event payload and project it into one binding event.
pub(super) fn wait_serial_event(
    binding: &BindingCallContext,
    resource: &WindowsSerialPortResource,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<SerialEvent> {
    // surface any dropped records before the next queued event
    if let Some(dropped_count) = take_event_overflow_count(resource) {
        let sequence = next_event_sequence(resource);

        return Ok(overflow_event(binding, sequence, dropped_count));
    }

    let timeout = Duration::from_nanos(timeout_ns);
    let Some(record) = resource.event_queue.pop_with_timeout(timeout) else {
        return Err(serial_event_would_block(operation));
    };

    Ok(event_from_record(binding, resource, record))
}

/// Validate one requested serial queue size.
pub(super) fn validate_queue_size(
    field: &'static str,
    queue_size: Option<u32>,
) -> RuntimeResult<()> {
    if queue_size == Some(0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "serial queue size must be greater than zero",
        ))
        .boxed());
    }

    Ok(())
}

/// Validate windows serial open options before touching the host.
pub(super) fn validate_open_options(options: &SerialPortOpenOptions) -> RuntimeResult<()> {
    // windows serial endpoints are exclusive by default
    if options.exclusive == Some(false) {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.device.serial.open")).boxed(),
        );
    }

    // validate queue hints before we touch the host
    validate_queue_size("options.readBufferSize", options.read_buffer_size)?;
    validate_queue_size("options.writeBufferSize", options.write_buffer_size)?;

    Ok(())
}

/// Open one windows serial endpoint handle.
pub(super) fn open_serial_handle(port_name: &str) -> RuntimeResult<HANDLE> {
    let path = serial_port_path(port_name);
    let path = core_platform::wide_from_str("id", &path)?;

    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            FILE_GENERIC_READ | FILE_GENERIC_WRITE,
            0,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OVERLAPPED,
            0,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(serial_io_error(
            "destack.device.serial.open",
            "CreateFileW",
            "failed to open serial endpoint",
        ));
    }

    let inherit_status = unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0) };
    if inherit_status == 0 {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(handle);
        }

        return Err(serial_io_error(
            "destack.device.serial.open",
            "SetHandleInformation",
            "failed to clear serial-handle inheritance",
        ));
    }

    Ok(handle)
}
