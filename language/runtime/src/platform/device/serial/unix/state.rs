use std::ffi::CString;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::sync::Arc;
use std::thread::JoinHandle;

use parking_lot::Mutex;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeAbi;
use crate::platform::core::BoundedQueue;
use crate::platform::device::{
    SerialDisconnectedEvent, SerialErrorEvent, SerialErrorKind, SerialEvent, SerialEventMetadata,
    SerialInputSignals, SerialModemStatusChangedEvent, SerialPortDescriptor, SerialPortTransport,
    SerialReadReadyEvent,
};
use crate::platform::diagnostic::{PlatformErrorCode, io_error_code_from_errno};
use crate::platform::fs::core::path_ref_from_bytes;
use crate::platform::resource::{ResourceFinalizer, ResourceKind};
use crate::platform::{NativeArray, PlatformError, core as core_platform, fs, resource};
use crate::runtime::BindingCallContext;

/// Maximum queued serial session events per open unix port.
pub(super) const SERIAL_EVENT_QUEUE_CAPACITY: usize = 128;

/// One queued unix serial event record.
#[derive(Debug, Clone, Copy)]
pub(super) enum UnixSerialEventRecord {
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

/// Prefix used when one unix serial identifier needs one byte-safe encoding.
pub(super) const SERIAL_BYTES_ID_PREFIX: &str = "serial-unix-bytes:";

/// Close one owned unix serial descriptor during finalization.
pub(super) struct UnixSerialDescriptorFinalizer {
    /// Shared event runtime teardown state.
    pub(super) runtime: UnixSerialEventRuntime,
    /// Descriptor to close.
    pub(super) descriptor: RawFd,
    /// Shared event queue to close during teardown.
    pub(super) event_queue: Arc<BoundedQueue<UnixSerialEventRecord>>,
}

impl ResourceFinalizer for UnixSerialDescriptorFinalizer {
    /// Close the descriptor during resource finalization.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        drop(self.runtime);
        self.event_queue.close();

        unsafe {
            libc::close(self.descriptor);
        }
    }
}

/// One live unix serial event watcher.
pub(super) struct UnixSerialEventRuntime {
    /// Write side of the shutdown pipe.
    pub(super) shutdown_write: OwnedFd,
    /// Joined watcher thread.
    pub(super) join_handle: Option<JoinHandle<()>>,
}

impl Drop for UnixSerialEventRuntime {
    fn drop(&mut self) {
        // wake the watcher thread before waiting for it
        let _ = unsafe { libc::write(self.shutdown_write.as_raw_fd(), [1u8].as_ptr().cast(), 1) };

        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

/// Stable serial descriptor snapshot carried by one opened resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct UnixSerialDescriptorInfo {
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
    /// Device path bytes used for reopening.
    pub(super) path_bytes: Vec<u8>,
    /// USB vendor identifier when available.
    pub(super) usb_vendor_id: Option<u16>,
    /// USB product identifier when available.
    pub(super) usb_product_id: Option<u16>,
    /// Bluetooth service-class identifier when available.
    pub(super) bluetooth_service_class_id: Option<String>,
}

/// Mutable event state for one opened unix serial resource.
#[derive(Debug, Clone, Copy)]
pub(super) struct UnixSerialPortState {
    /// The next event sequence number to emit.
    pub(super) next_sequence: u64,
    /// Number of dropped session events already surfaced to the caller.
    pub(super) reported_dropped_count: u64,
    /// Last observed modem-status snapshot when available.
    pub(super) last_signals: Option<SerialInputSignals>,
    /// Whether one read-ready event is already pending in the queue.
    pub(super) is_read_ready_queued: bool,
}

/// Shared unix serial resource state.
pub(super) struct UnixSerialPortResource {
    /// Owned host descriptor.
    pub(super) descriptor: RawFd,
    /// Stable descriptor snapshot for the resource.
    pub(super) descriptor_info: UnixSerialDescriptorInfo,
    /// Mutex that serializes host configuration mutations.
    pub(super) operation_lock: Mutex<()>,
    /// Bounded unix serial event queue.
    pub(super) event_queue: Arc<BoundedQueue<UnixSerialEventRecord>>,
    /// Mutable event state guarded by one mutex.
    pub(super) state: Mutex<UnixSerialPortState>,
}

/// Build one invalid serial-handle error.
pub(super) fn invalid_serial_handle(operation: &'static str) -> Box<RuntimeError> {
    core_platform::io_operation_error(
        operation,
        Some(PlatformErrorCode::InvalidArgumentValue),
        "unknown serial handle",
    )
}

/// Build one mapped unix serial I/O error from explicit errno.
pub(super) fn serial_io_error_with_errno(
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
        Some(syscall.to_string()),
        format!("{syscall} failed: {message}"),
    ))
    .boxed()
}

/// Build one mapped unix serial I/O error from current errno.
pub(super) fn serial_io_error(
    operation: &'static str,
    syscall: &'static str,
    message: &str,
) -> Box<RuntimeError> {
    let errno = core_platform::get_errno();

    serial_io_error_with_errno(operation, syscall, errno, message)
}

/// Resolve one serial handle into one shared unix serial resource.
pub(super) fn serial_resource(
    binding: &BindingCallContext,
    handle: resource::SerialPortHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<UnixSerialPortResource>> {
    let resolved = binding.worker().resources.with_entry(handle.0, |entry| {
        if entry.kind != ResourceKind::SerialPort {
            return None;
        }

        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<Arc<UnixSerialPortResource>>())
            .map(Arc::clone)
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

/// Decode one raw unix serial path into one owned c string.
pub(super) fn serial_path_cstring(
    path_bytes: &[u8],
    field: &'static str,
) -> RuntimeResult<CString> {
    CString::new(path_bytes).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "serial path contains an interior nul byte",
        ))
        .boxed()
    })
}

/// Store one unix path payload as one platform `OsPath`.
pub(super) fn store_path_bytes(binding: &BindingCallContext, path_bytes: &[u8]) -> fs::OsPath {
    let path_bytes = if path_bytes.is_empty() {
        NativeArray {
            data: std::ptr::null_mut(),
            len: 0,
            capacity: 0,
        }
    } else {
        binding.store_array_copy(path_bytes)
    };

    path_ref_from_bytes(fs::PathBytesAbi::<NativeAbi>(path_bytes))
}

/// Build one binding-visible serial descriptor from one stored descriptor snapshot.
pub(super) fn serial_descriptor_from_info(
    binding: &BindingCallContext,
    info: &UnixSerialDescriptorInfo,
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
        path: store_path_bytes(binding, &info.path_bytes),
        usb_vendor_id: info.usb_vendor_id,
        usb_product_id: info.usb_product_id,
        bluetooth_service_class_id: info
            .bluetooth_service_class_id
            .as_ref()
            .map(|value| binding.store_string_owned(value.clone())),
    }
}

/// Build one shutdown pipe for one unix serial event runtime.
pub(super) fn unix_event_shutdown_pipe(
    operation: &'static str,
) -> RuntimeResult<(OwnedFd, OwnedFd)> {
    let mut descriptors = [0; 2];

    // build one shutdown pipe for the watcher thread
    let status = unsafe { libc::pipe(descriptors.as_mut_ptr()) };
    if status < 0 {
        return Err(RuntimeError::from(PlatformError::io_with(
            None,
            None,
            Some(core_platform::get_errno()),
            Some(operation.to_string()),
            Some(String::from("pipe")),
            String::from("failed to build unix serial event shutdown pipe"),
        ))
        .boxed());
    }

    // take ownership of the shutdown descriptors
    let read_fd = unsafe { OwnedFd::from_raw_fd(descriptors[0]) };
    let write_fd = unsafe { OwnedFd::from_raw_fd(descriptors[1]) };

    // mark both pipe ends close-on-exec
    for descriptor in [read_fd.as_raw_fd(), write_fd.as_raw_fd()] {
        let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
        if flags < 0 {
            return Err(RuntimeError::from(PlatformError::io_with(
                None,
                None,
                Some(core_platform::get_errno()),
                Some(operation.to_string()),
                Some(String::from("fcntl(F_GETFD)")),
                String::from("failed to read unix serial event pipe flags"),
            ))
            .boxed());
        }

        let status = unsafe { libc::fcntl(descriptor, libc::F_SETFD, flags | libc::FD_CLOEXEC) };
        if status < 0 {
            return Err(RuntimeError::from(PlatformError::io_with(
                None,
                None,
                Some(core_platform::get_errno()),
                Some(operation.to_string()),
                Some(String::from("fcntl(F_SETFD)")),
                String::from("failed to mark unix serial event pipe close-on-exec"),
            ))
            .boxed());
        }
    }

    Ok((read_fd, write_fd))
}

/// Return one pending unix session-overflow delta.
pub(super) fn take_event_overflow_count(resource: &UnixSerialPortResource) -> Option<u64> {
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
pub(super) fn mark_read_ready_queued(resource: &UnixSerialPortResource, is_queued: bool) {
    let mut state = resource.state.lock();
    state.is_read_ready_queued = is_queued;
}

/// Return whether one read-ready event may be queued now.
pub(super) fn should_queue_read_ready(resource: &UnixSerialPortResource) -> bool {
    let mut state = resource.state.lock();
    if state.is_read_ready_queued {
        return false;
    }

    state.is_read_ready_queued = true;

    true
}

/// Build one read-ready serial event.
pub(super) fn read_ready_event(
    binding: &BindingCallContext,
    resource: &UnixSerialPortResource,
    available_bytes: Option<u64>,
) -> SerialEvent {
    let metadata = next_event_metadata(resource);

    SerialEvent::SerialReadReadyEvent(SerialReadReadyEvent {
        kind: binding.store_string("readReady"),
        metadata,
        available_bytes,
    })
}

/// Build one binding-visible serial event from one queued unix event record.
pub(super) fn event_from_record(
    binding: &BindingCallContext,
    resource: &UnixSerialPortResource,
    record: UnixSerialEventRecord,
) -> SerialEvent {
    match record {
        UnixSerialEventRecord::ReadReady { available_bytes } => {
            read_ready_event(binding, resource, available_bytes)
        }
        UnixSerialEventRecord::ModemStatusChanged { signals } => {
            modem_status_changed_event(binding, resource, signals)
        }
        UnixSerialEventRecord::Disconnected => disconnected_event(binding, resource),
        UnixSerialEventRecord::Error {
            kind,
            backend_code,
            backend_detail,
        } => error_event(binding, resource, kind, backend_code, backend_detail),
    }
}

/// Build one modem-status-changed serial event.
pub(super) fn modem_status_changed_event(
    binding: &BindingCallContext,
    resource: &UnixSerialPortResource,
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
    resource: &UnixSerialPortResource,
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
    resource: &UnixSerialPortResource,
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

/// Return one fresh serial event metadata payload.
fn next_event_metadata(resource: &UnixSerialPortResource) -> SerialEventMetadata {
    let mut state = resource.state.lock();
    let sequence = state.next_sequence;
    state.next_sequence = state.next_sequence.saturating_add(1);

    SerialEventMetadata {
        timestamp_ns: core_platform::monotonic_now_ns(),
        sequence,
    }
}

/// Return one fresh serial session-overflow sequence number.
pub(super) fn next_event_sequence(resource: &UnixSerialPortResource) -> u64 {
    let mut state = resource.state.lock();
    let sequence = state.next_sequence;
    state.next_sequence = state.next_sequence.saturating_add(1);

    sequence
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build one unix serial resource fixture for queue-state tests.
    fn resource() -> UnixSerialPortResource {
        UnixSerialPortResource {
            descriptor: -1,
            descriptor_info: UnixSerialDescriptorInfo {
                id: String::from("serial-unix-bytes:test"),
                transport: SerialPortTransport::Native,
                name: String::from("tty-test"),
                manufacturer: None,
                product: None,
                serial_number: None,
                path_bytes: b"/dev/tty-test".to_vec(),
                usb_vendor_id: None,
                usb_product_id: None,
                bluetooth_service_class_id: None,
            },
            operation_lock: Mutex::new(()),
            event_queue: Arc::new(BoundedQueue::new(1)),
            state: Mutex::new(UnixSerialPortState {
                next_sequence: 1,
                reported_dropped_count: 0,
                last_signals: None,
                is_read_ready_queued: false,
            }),
        }
    }

    /// Report one overflow delta after queued unix session records are dropped.
    #[test]
    fn test_take_event_overflow_count_reports_one_delta() {
        let resource = resource();

        resource
            .event_queue
            .push_drop_oldest(UnixSerialEventRecord::Disconnected);
        resource
            .event_queue
            .push_drop_oldest(UnixSerialEventRecord::Error {
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
