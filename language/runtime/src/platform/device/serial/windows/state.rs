use std::sync::Arc;

use parking_lot::Mutex;
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_BAD_COMMAND, ERROR_DEVICE_NOT_CONNECTED,
    ERROR_FILE_NOT_FOUND, ERROR_GEN_FAILURE, ERROR_INVALID_FUNCTION, ERROR_INVALID_HANDLE,
    ERROR_NOT_SUPPORTED, ERROR_OPERATION_ABORTED, ERROR_PATH_NOT_FOUND, ERROR_SEM_TIMEOUT,
    ERROR_TIMEOUT, HANDLE,
};

use super::service::WindowsSerialService;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeAbi;
use crate::platform::device::{
    SerialDisconnectedEvent, SerialErrorEvent, SerialErrorKind, SerialEvent, SerialEventMetadata,
    SerialInputSignals, SerialModemStatusChangedEvent, SerialPortDescriptor, SerialPortTransport,
    SerialReadReadyEvent,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::core::path_ref_from_utf16;
use crate::platform::resource::{ResourceFinalizer, ResourceKind};
use crate::platform::{NativeArray, PlatformError, core as core_platform, fs, resource};
use crate::runtime::BindingCallContext;
use crate::runtime::control::queue::BoundedQueue;

/// One queued windows serial event record.
#[derive(Debug, Clone, Copy)]
pub(super) enum WindowsSerialEventRecord {
    /// One read-ready event.
    ReadReady {
        /// Estimated readable byte count.
        available_bytes: Option<u64>,
    },
    /// One modem-status change event.
    ModemStatusChanged {
        /// Current modem-signal snapshot.
        signals: SerialInputSignals,
    },
    /// One disconnected event.
    Disconnected,
    /// One backend error event.
    Error {
        /// Portable serial error kind.
        kind: SerialErrorKind,
        /// Backend-specific error code.
        backend_code: Option<i32>,
        /// Backend-specific detail payload.
        backend_detail: Option<i32>,
    },
}

/// Finalizer that stops the windows serial event loop and closes the handle.
pub(super) struct WindowsSerialEventFinalizer {
    /// Shared windows serial ingress service.
    pub(super) service: Arc<WindowsSerialService>,
    /// Registered ingress runtime identifier.
    pub(super) registration_id: u64,
    /// Shared serial resource state.
    pub(super) resource: Arc<WindowsSerialPortResource>,
}

impl ResourceFinalizer for WindowsSerialEventFinalizer {
    /// Stop the ingress runtime, close the queue, and close the serial handle.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        self.service.stop_event_runtime(self.registration_id);

        self.resource.event_queue.close();

        unsafe {
            CloseHandle(self.resource.handle);
        }
    }
}

/// Stable serial descriptor snapshot carried by one opened resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WindowsSerialDescriptorInfo {
    /// Stable serial identifier used by the binding surface.
    pub(super) id: String,
    /// Serial transport kind.
    pub(super) transport: SerialPortTransport,
    /// Host-visible endpoint name.
    pub(super) name: String,
    /// Manufacturer string when available.
    pub(super) manufacturer: Option<String>,
    /// Product string when available.
    pub(super) product: Option<String>,
    /// Device serial number when available.
    pub(super) serial_number: Option<String>,
    /// Device path units used for reopening.
    pub(super) path_units: Vec<u16>,
    /// USB vendor identifier when available.
    pub(super) usb_vendor_id: Option<u16>,
    /// USB product identifier when available.
    pub(super) usb_product_id: Option<u16>,
    /// Bluetooth service-class identifier when available.
    pub(super) bluetooth_service_class_id: Option<String>,
}

/// Classified windows serial line error.
#[derive(Debug, Clone, Copy)]
pub(super) struct WindowsSerialPendingError {
    /// Portable serial error kind.
    pub(super) kind: SerialErrorKind,
    /// Backend-specific error code.
    pub(super) backend_code: Option<i32>,
    /// Backend-specific detail payload.
    pub(super) backend_detail: Option<i32>,
}

/// Mutable event state for one opened windows serial resource.
#[derive(Debug, Clone, Copy)]
pub(super) struct WindowsSerialPortState {
    /// The next event sequence number to emit.
    pub(super) next_sequence: u64,
    /// Number of dropped session events already surfaced to the caller.
    pub(super) reported_dropped_count: u64,
    /// Last observed modem-status snapshot when available.
    pub(super) last_signals: Option<SerialInputSignals>,
    /// Whether one read-ready event is already pending in the queue.
    pub(super) is_read_ready_queued: bool,
}

/// Shared windows serial resource state.
pub(super) struct WindowsSerialPortResource {
    /// Owned host handle.
    pub(super) handle: HANDLE,
    /// Stable descriptor snapshot for the resource.
    pub(super) descriptor_info: WindowsSerialDescriptorInfo,
    /// Mutex that serializes host configuration mutations.
    pub(super) operation_lock: Mutex<()>,
    /// Bounded windows serial event queue.
    pub(super) event_queue: BoundedQueue<WindowsSerialEventRecord>,
    /// Mutable event state guarded by one mutex.
    pub(super) state: Mutex<WindowsSerialPortState>,
}

/// Return whether one Win32 error means the serial endpoint disappeared.
pub(super) fn is_disconnected_code(code: u32) -> bool {
    code == ERROR_INVALID_HANDLE
        || code == ERROR_FILE_NOT_FOUND
        || code == ERROR_PATH_NOT_FOUND
        || code == ERROR_BAD_COMMAND
        || code == ERROR_DEVICE_NOT_CONNECTED
        || code == ERROR_GEN_FAILURE
}

/// Return whether one Win32 error means modem-status queries are unavailable.
pub(super) fn is_optional_signal_code(code: u32) -> bool {
    code == ERROR_INVALID_FUNCTION || code == ERROR_NOT_SUPPORTED
}

/// Build one invalid serial-handle error.
pub(super) fn invalid_serial_handle(operation: &'static str) -> Box<RuntimeError> {
    core_platform::io_operation_error(
        operation,
        Some(PlatformErrorCode::InvalidArgumentValue),
        "unknown serial handle",
    )
}

/// Build one mapped windows serial I/O error from one explicit code.
pub(super) fn serial_io_error_with_code(
    operation: &'static str,
    syscall: &'static str,
    code: u32,
    message: &str,
) -> Box<RuntimeError> {
    let platform_code = if is_disconnected_code(code) {
        Some(PlatformErrorCode::IoNotFound)
    } else if code == ERROR_ACCESS_DENIED {
        Some(PlatformErrorCode::IoPermissionDenied)
    } else if code == ERROR_OPERATION_ABORTED {
        Some(PlatformErrorCode::IoInterrupted)
    } else if code == ERROR_TIMEOUT || code == ERROR_SEM_TIMEOUT {
        Some(PlatformErrorCode::IoWouldBlock)
    } else {
        None
    };

    RuntimeError::from(PlatformError::io_with(
        platform_code,
        None,
        Some(code as i32),
        Some(operation.to_string()),
        Some(syscall.to_string()),
        format!("{syscall} failed: {message} ({code})"),
    ))
    .boxed()
}

/// Build one mapped windows serial I/O error from `GetLastError`.
pub(super) fn serial_io_error(
    operation: &'static str,
    syscall: &'static str,
    message: &str,
) -> Box<RuntimeError> {
    let code = core_platform::last_error_code() as u32;

    serial_io_error_with_code(operation, syscall, code, message)
}

/// Resolve one serial handle into one shared windows serial resource.
pub(super) fn serial_resource(
    binding: &BindingCallContext,
    handle: resource::SerialPortHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<WindowsSerialPortResource>> {
    let resolved = binding.worker().resources.with_entry(handle.0, |entry| {
        if entry.kind != ResourceKind::SerialPort {
            return None;
        }

        entry.payload_cloned::<Arc<WindowsSerialPortResource>>()
    });

    resolved
        .flatten()
        .ok_or_else(|| invalid_serial_handle(operation))
}

/// Close one serial resource and run finalization.
pub(super) fn close_serial_resource(
    binding: &BindingCallContext,
    handle: resource::SerialPortHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    let kind = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| entry.kind)
        .ok_or_else(|| invalid_serial_handle(operation))?;
    if kind != ResourceKind::SerialPort {
        return Err(invalid_serial_handle(operation));
    }

    if !binding.worker().resources.remove_and_finalize(
        binding.world(),
        handle.0,
        Some(binding.engine()),
    ) {
        return Err(invalid_serial_handle(operation));
    }

    Ok(())
}

/// Store one windows path payload as one platform `OsPath`.
pub(super) fn store_path_utf16(binding: &BindingCallContext, path_units: &[u16]) -> fs::OsPath {
    let path_units = if path_units.is_empty() {
        fs::PathUtf16Abi::<NativeAbi>(NativeArray {
            data: std::ptr::null_mut(),
            len: 0,
            capacity: 0,
        })
    } else {
        fs::PathUtf16Abi::<NativeAbi>(binding.store_array_copy(path_units))
    };

    path_ref_from_utf16(path_units)
}

/// Build one binding-visible serial descriptor from one stored descriptor snapshot.
pub(super) fn serial_descriptor_from_info(
    binding: &BindingCallContext,
    info: &WindowsSerialDescriptorInfo,
) -> SerialPortDescriptor {
    SerialPortDescriptor {
        id: binding.store_string_owned(info.id.clone()),
        transport: info.transport,
        name: binding.store_string_owned(info.name.clone()),
        manufacturer: info
            .manufacturer
            .as_ref()
            .map(|value| binding.store_string_owned(value.clone())),
        product: info
            .product
            .as_ref()
            .map(|value| binding.store_string_owned(value.clone())),
        serial_number: info
            .serial_number
            .as_ref()
            .map(|value| binding.store_string_owned(value.clone())),
        path: store_path_utf16(binding, &info.path_units),
        usb_vendor_id: info.usb_vendor_id,
        usb_product_id: info.usb_product_id,
        bluetooth_service_class_id: info
            .bluetooth_service_class_id
            .as_ref()
            .map(|value| binding.store_string_owned(value.clone())),
    }
}

/// Update one stored modem-status baseline and return whether the snapshot changed.
pub(super) fn update_modem_status(
    resource: &WindowsSerialPortResource,
    signals: SerialInputSignals,
) -> Option<SerialInputSignals> {
    let mut state = resource.state.lock();
    let changed = match state.last_signals {
        Some(previous) => previous != signals,
        None => true,
    };
    state.last_signals = Some(signals);

    if changed {
        return Some(signals);
    }

    None
}

/// Build one read-ready serial event.
pub(super) fn read_ready_event(
    binding: &BindingCallContext,
    resource: &WindowsSerialPortResource,
    available_bytes: Option<u64>,
) -> SerialEvent {
    let metadata = next_event_metadata(resource);

    SerialEvent::SerialReadReadyEvent(SerialReadReadyEvent {
        kind: binding.store_string("readReady"),
        metadata,
        available_bytes,
    })
}

/// Build one modem-status-changed serial event.
pub(super) fn modem_status_changed_event(
    binding: &BindingCallContext,
    resource: &WindowsSerialPortResource,
    signals: SerialInputSignals,
) -> SerialEvent {
    let metadata = next_event_metadata(resource);

    SerialEvent::SerialModemStatusChangedEvent(SerialModemStatusChangedEvent {
        kind: binding.store_string("modemStatusChanged"),
        metadata,
        signals,
    })
}

/// Build one disconnected serial event.
pub(super) fn disconnected_event(
    binding: &BindingCallContext,
    resource: &WindowsSerialPortResource,
) -> SerialEvent {
    let metadata = next_event_metadata(resource);

    SerialEvent::SerialDisconnectedEvent(SerialDisconnectedEvent {
        kind: binding.store_string("disconnected"),
        metadata,
    })
}

/// Build one error serial event.
pub(super) fn error_event(
    binding: &BindingCallContext,
    resource: &WindowsSerialPortResource,
    kind: SerialErrorKind,
    backend_code: Option<i32>,
    backend_detail: Option<i32>,
) -> SerialEvent {
    let metadata = next_event_metadata(resource);

    SerialEvent::SerialErrorEvent(SerialErrorEvent {
        kind: binding.store_string("error"),
        metadata,
        error_kind: kind,
        backend_code,
        backend_detail,
    })
}

/// Build one binding-visible serial event from one queued windows event record.
pub(super) fn event_from_record(
    binding: &BindingCallContext,
    resource: &WindowsSerialPortResource,
    record: WindowsSerialEventRecord,
) -> SerialEvent {
    match record {
        WindowsSerialEventRecord::ReadReady { available_bytes } => {
            read_ready_event(binding, resource, available_bytes)
        }
        WindowsSerialEventRecord::ModemStatusChanged { signals } => {
            modem_status_changed_event(binding, resource, signals)
        }
        WindowsSerialEventRecord::Disconnected => disconnected_event(binding, resource),
        WindowsSerialEventRecord::Error {
            kind,
            backend_code,
            backend_detail,
        } => error_event(binding, resource, kind, backend_code, backend_detail),
    }
}

/// Return one pending windows session-overflow delta.
pub(super) fn take_event_overflow_count(resource: &WindowsSerialPortResource) -> Option<u64> {
    let dropped_count = resource.event_queue.dropped_count();
    let mut state = resource.state.lock();
    if dropped_count <= state.reported_dropped_count {
        return None;
    }

    let delta = dropped_count - state.reported_dropped_count;
    state.reported_dropped_count = dropped_count;

    Some(delta)
}

/// Mark whether a read-ready event is currently queued.
pub(super) fn mark_read_ready_queued(resource: &WindowsSerialPortResource, is_queued: bool) {
    let mut state = resource.state.lock();
    state.is_read_ready_queued = is_queued;
}

/// Return whether one read-ready event may be queued now.
pub(super) fn should_queue_read_ready(resource: &WindowsSerialPortResource) -> bool {
    let mut state = resource.state.lock();
    if state.is_read_ready_queued {
        return false;
    }

    state.is_read_ready_queued = true;

    true
}

/// Return one fresh serial event metadata payload.
fn next_event_metadata(resource: &WindowsSerialPortResource) -> SerialEventMetadata {
    let mut state = resource.state.lock();
    let sequence = state.next_sequence;
    state.next_sequence = state.next_sequence.saturating_add(1);

    SerialEventMetadata {
        timestamp_ns: core_platform::monotonic_now_ns(),
        sequence,
    }
}

/// Return one fresh serial session-overflow sequence number.
pub(super) fn next_event_sequence(resource: &WindowsSerialPortResource) -> u64 {
    let mut state = resource.state.lock();
    let sequence = state.next_sequence;
    state.next_sequence = state.next_sequence.saturating_add(1);

    sequence
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build one windows serial resource fixture for queue-state tests.
    fn resource() -> WindowsSerialPortResource {
        WindowsSerialPortResource {
            handle: 0,
            descriptor_info: WindowsSerialDescriptorInfo {
                id: String::from("COM9"),
                transport: SerialPortTransport::Native,
                name: String::from("COM9"),
                manufacturer: None,
                product: None,
                serial_number: None,
                path_units: "COM9".encode_utf16().collect(),
                usb_vendor_id: None,
                usb_product_id: None,
                bluetooth_service_class_id: None,
            },
            operation_lock: Mutex::new(()),
            event_queue: BoundedQueue::new(1),
            state: Mutex::new(WindowsSerialPortState {
                next_sequence: 1,
                reported_dropped_count: 0,
                last_signals: None,
                is_read_ready_queued: false,
            }),
        }
    }

    /// Report one overflow delta after queued windows session records are dropped.
    #[test]
    fn test_take_event_overflow_count_reports_one_delta() {
        let resource = resource();

        resource
            .event_queue
            .push_drop_oldest(WindowsSerialEventRecord::Disconnected);
        resource
            .event_queue
            .push_drop_oldest(WindowsSerialEventRecord::Error {
                kind: SerialErrorKind::Unknown,
                backend_code: None,
                backend_detail: None,
            });

        assert_eq!(take_event_overflow_count(&resource), Some(1));
        assert_eq!(take_event_overflow_count(&resource), None);
    }

    /// Queue one read-ready event once until the caller rearms it.
    #[test]
    fn test_should_queue_read_ready_gates_duplicates() {
        let resource = resource();

        assert!(should_queue_read_ready(&resource));
        assert!(!should_queue_read_ready(&resource));

        mark_read_ready_queued(&resource, false);

        assert!(should_queue_read_ready(&resource));
    }
}
