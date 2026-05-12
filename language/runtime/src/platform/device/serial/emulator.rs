use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use parking_lot::{Condvar, Mutex};

use super::core::SERIAL_PORT_RESOURCE_LABEL;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeAbi, NativeSlice};
use crate::platform::core::{
    BoundedQueue, io_operation_error, io_would_block, monotonic_now_ns, timeout_deadline,
};
use crate::platform::device::{
    SerialDataBits, SerialDisconnectedEvent, SerialEvent, SerialEventMetadata, SerialFlowControl,
    SerialInputSignals, SerialModemStatusChangedEvent, SerialOutputSignals, SerialOverflowEvent,
    SerialOverflowEventMetadata, SerialParity, SerialPortConfig, SerialPortDescriptor,
    SerialPortTransport, SerialReadReadyEvent, SerialStopBits,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::core as core_fs;
use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceKind};
use crate::platform::{PlatformError, fs, resource};
use crate::runtime::BindingCallContext;

/// Prefix used by test-only virtual serial identifiers.
pub(crate) const TEST_SERIAL_ID_PREFIX: &str = "serial-test:";

/// Maximum queued serial session events for one virtual handle.
const TEST_SERIAL_EVENT_QUEUE_CAPACITY: usize = 128;

/// Default line configuration for one virtual serial endpoint.
const TEST_SERIAL_DEFAULT_BAUD_RATE: u32 = 115_200;

/// Shared per-worker registry for test-only virtual serial endpoints.
#[derive(Default)]
pub(crate) struct SerialTestRegistry {
    /// Installed test ports keyed by their stable binding id.
    ports: Mutex<BTreeMap<String, Arc<TestSerialPort>>>,
}

/// Controller handle used by tests to drive one virtual serial endpoint.
#[derive(Clone)]
pub(crate) struct SerialTestController {
    /// Shared virtual serial endpoint.
    port: Arc<TestSerialPort>,
}

/// One queued virtual serial event.
#[derive(Debug, Clone, Copy)]
enum TestSerialEventRecord {
    /// One read-ready transition.
    ReadReady {
        /// Estimated currently readable byte count.
        available_bytes: Option<u64>,
    },
    /// One modem-status transition.
    ModemStatusChanged {
        /// Latest input-signal snapshot.
        signals: SerialInputSignals,
    },
    /// One disconnect transition.
    Disconnected,
}

/// Mutable per-handle event state.
#[derive(Debug, Clone, Copy)]
struct TestSerialHandleState {
    /// The next event sequence number to emit.
    next_sequence: u64,
    /// Number of dropped events already surfaced as overflow.
    reported_dropped_count: u64,
    /// Last input-signal snapshot observed by this handle.
    last_signals: SerialInputSignals,
    /// Whether one unread read-ready event is already queued.
    is_read_ready_queued: bool,
}

/// One opened virtual serial session resource.
pub(crate) struct TestSerialPortResource {
    /// Shared virtual serial endpoint.
    port: Arc<TestSerialPort>,
    /// Per-handle queued events.
    event_queue: Arc<BoundedQueue<TestSerialEventRecord>>,
    /// Mutable per-handle event state.
    state: Mutex<TestSerialHandleState>,
}

/// Finalizer that unregisters one virtual serial session.
struct TestSerialPortFinalizer {
    /// Shared virtual serial endpoint.
    port: Arc<TestSerialPort>,
    /// Stable registration identifier inside the port.
    registration_id: u64,
    /// Per-handle queue to close during teardown.
    event_queue: Arc<BoundedQueue<TestSerialEventRecord>>,
}

impl ResourceFinalizer for TestSerialPortFinalizer {
    /// Unregister the session and close its queue.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        self.port.unregister(self.registration_id);
        self.event_queue.close();
    }
}

/// One live virtual serial endpoint.
struct TestSerialPort {
    /// Stable binding-visible identifier.
    id: String,
    /// Binding-visible name.
    name: String,
    /// Shared mutable port state.
    state: Mutex<TestSerialPortState>,
    /// Waiters for inbound bytes and disconnect changes.
    input_condvar: Condvar,
    /// Waiters for outbound drain changes.
    output_condvar: Condvar,
    /// The next per-session registration id.
    next_registration_id: AtomicU64,
}

/// Shared mutable virtual serial endpoint state.
struct TestSerialPortState {
    /// Current line configuration.
    config: SerialPortConfig,
    /// Pending inbound bytes readable by the runtime.
    input: VecDeque<u8>,
    /// Pending outbound bytes written by the runtime.
    output: VecDeque<u8>,
    /// Current input-signal snapshot.
    input_signals: SerialInputSignals,
    /// Current output-signal snapshot.
    output_signals: SerialOutputSignals,
    /// Whether the endpoint has been disconnected.
    is_disconnected: bool,
    /// Registered live sessions for this endpoint.
    sessions: HashMap<u64, std::sync::Weak<TestSerialPortResource>>,
}

impl SerialTestRegistry {
    /// Install one named virtual serial endpoint for the active runtime.
    fn install_port(&self, name: &str) -> SerialTestController {
        let id = format!("{TEST_SERIAL_ID_PREFIX}{name}");
        let mut ports = self.ports.lock();

        let port = ports
            .entry(id.clone())
            .or_insert_with(|| Arc::new(TestSerialPort::new(id, name.to_string())))
            .clone();

        SerialTestController { port }
    }

    /// Resolve one installed endpoint by stable id.
    fn port(&self, id: &str) -> Option<Arc<TestSerialPort>> {
        self.ports.lock().get(id).cloned()
    }
}

impl SerialTestController {
    /// Return the stable binding id for this endpoint.
    pub(crate) fn id(&self) -> &str {
        self.port.id.as_str()
    }

    /// Queue inbound bytes and publish one read-ready transition.
    pub(crate) fn enqueue_input(&self, bytes: &[u8]) {
        self.port.enqueue_input(bytes);
    }

    /// Wait until the runtime has produced at least one outbound payload.
    pub(crate) fn wait_for_output(&self, minimum_length: usize) {
        self.port.wait_for_output(minimum_length);
    }

    /// Take one exact outbound payload from the virtual transmit queue.
    pub(crate) fn take_output_exact(&self, length: usize) -> Vec<u8> {
        self.port.take_output_exact(length)
    }

    /// Replace the visible input-signal snapshot and publish one event.
    pub(crate) fn set_input_signals(&self, signals: SerialInputSignals) {
        self.port.set_input_signals(signals);
    }

    /// Read the current output-signal snapshot.
    pub(crate) fn output_signals(&self) -> SerialOutputSignals {
        self.port.output_signals()
    }

    /// Disconnect the endpoint and publish one disconnect event.
    pub(crate) fn disconnect(&self) {
        self.port.disconnect();
    }
}

impl TestSerialPort {
    /// Build one fresh virtual serial endpoint.
    fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            state: Mutex::new(TestSerialPortState {
                config: SerialPortConfig {
                    baud_rate: TEST_SERIAL_DEFAULT_BAUD_RATE,
                    data_bits: SerialDataBits::Eight,
                    parity: SerialParity::None,
                    stop_bits: SerialStopBits::One,
                    flow_control: SerialFlowControl {
                        request_to_send_clear_to_send_enabled: false,
                        data_terminal_ready_data_set_ready_enabled: false,
                        xon_xoff_enabled: false,
                    },
                },
                input: VecDeque::new(),
                output: VecDeque::new(),
                input_signals: SerialInputSignals {
                    clear_to_send: false,
                    data_set_ready: false,
                    data_carrier_detect: false,
                    ring_indicator: false,
                },
                output_signals: SerialOutputSignals {
                    data_terminal_ready: None,
                    request_to_send: None,
                    break_condition_active: None,
                },
                is_disconnected: false,
                sessions: HashMap::new(),
            }),
            input_condvar: Condvar::new(),
            output_condvar: Condvar::new(),
            next_registration_id: AtomicU64::new(1),
        }
    }

    /// Register one opened session for this endpoint.
    fn register(&self, registration_id: u64, resource: &Arc<TestSerialPortResource>) {
        self.state
            .lock()
            .sessions
            .insert(registration_id, Arc::downgrade(resource));
    }

    /// Reserve one stable session registration id.
    fn reserve_registration_id(&self) -> u64 {
        self.next_registration_id.fetch_add(1, Ordering::AcqRel)
    }

    /// Unregister one opened session.
    fn unregister(&self, registration_id: u64) {
        self.state.lock().sessions.remove(&registration_id);
    }

    /// Build one binding descriptor for this endpoint.
    fn descriptor(&self, binding: &BindingCallContext) -> SerialPortDescriptor {
        SerialPortDescriptor {
            id: binding.store_string_owned(self.id.clone()),
            transport: SerialPortTransport::Unknown,
            name: binding.store_string_owned(self.name.clone()),
            manufacturer: None,
            product: None,
            serial_number: None,
            path: store_test_serial_path(binding, self.id.as_str()),
            usb_vendor_id: None,
            usb_product_id: None,
            bluetooth_service_class_id: None,
        }
    }

    /// Queue inbound bytes and publish one read-ready transition per session.
    fn enqueue_input(&self, bytes: &[u8]) {
        let (available_bytes, sessions) = {
            let mut state = self.state.lock();

            // keep disconnected endpoints fail-closed
            if state.is_disconnected {
                return;
            }

            // append inbound bytes before publishing readiness
            for byte in bytes {
                state.input.push_back(*byte);
            }

            let available_bytes = state.input.len() as u64;
            let sessions = state.sessions.values().cloned().collect::<Vec<_>>();

            self.input_condvar.notify_all();

            (available_bytes, sessions)
        };

        // publish one unread read-ready transition per live session
        for session in sessions {
            let Some(session) = session.upgrade() else {
                continue;
            };

            let mut session_state = session.state.lock();
            if session_state.is_read_ready_queued {
                continue;
            }

            session_state.is_read_ready_queued = true;
            drop(session_state);

            session
                .event_queue
                .push_drop_oldest(TestSerialEventRecord::ReadReady {
                    available_bytes: Some(available_bytes),
                });
        }
    }

    /// Wait until at least one outbound byte is pending.
    fn wait_for_output(&self, minimum_length: usize) {
        let mut state = self.state.lock();

        while state.output.len() < minimum_length && !state.is_disconnected {
            self.output_condvar.wait(&mut state);
        }
    }

    /// Take one exact outbound payload from the transmit queue.
    fn take_output_exact(&self, length: usize) -> Vec<u8> {
        let mut state = self.state.lock();

        while state.output.len() < length && !state.is_disconnected {
            self.output_condvar.wait(&mut state);
        }

        let mut bytes = Vec::with_capacity(length);

        for _ in 0..length {
            if let Some(byte) = state.output.pop_front() {
                bytes.push(byte);
            }
        }

        self.output_condvar.notify_all();

        bytes
    }

    /// Replace the visible input-signal snapshot and publish one change event.
    fn set_input_signals(&self, signals: SerialInputSignals) {
        let sessions = {
            let mut state = self.state.lock();
            state.input_signals = signals;

            state.sessions.values().cloned().collect::<Vec<_>>()
        };

        for session in sessions {
            let Some(session) = session.upgrade() else {
                continue;
            };

            let mut session_state = session.state.lock();
            if session_state.last_signals == signals {
                continue;
            }

            session_state.last_signals = signals;
            drop(session_state);

            session
                .event_queue
                .push_drop_oldest(TestSerialEventRecord::ModemStatusChanged { signals });
        }
    }

    /// Return the current output-signal snapshot.
    fn output_signals(&self) -> SerialOutputSignals {
        self.state.lock().output_signals
    }

    /// Disconnect the endpoint and notify every live session.
    fn disconnect(&self) {
        let sessions = {
            let mut state = self.state.lock();
            state.is_disconnected = true;

            self.input_condvar.notify_all();
            self.output_condvar.notify_all();

            state.sessions.values().cloned().collect::<Vec<_>>()
        };

        for session in sessions {
            let Some(session) = session.upgrade() else {
                continue;
            };

            session
                .event_queue
                .push_drop_oldest(TestSerialEventRecord::Disconnected);
        }
    }
}

/// Install one virtual serial endpoint for the active runtime.
pub(crate) fn install_test_serial_port(
    binding: &BindingCallContext,
    name: &str,
) -> SerialTestController {
    binding
        .worker()
        .platform_state
        .device
        .serial_test_registry()
        .install_port(name)
}

/// Return whether one identifier targets the virtual serial test substrate.
pub(crate) fn is_test_serial_id(id: &str) -> bool {
    id.starts_with(TEST_SERIAL_ID_PREFIX)
}

/// Try to open one virtual serial session through the public binding path.
pub(crate) fn try_open_test_serial(
    binding: &BindingCallContext,
    id: &str,
) -> RuntimeResult<Option<resource::SerialPortHandle>> {
    if !is_test_serial_id(id) {
        return Ok(None);
    }

    let Some(port) = binding
        .worker()
        .platform_state
        .device
        .serial_test_registry()
        .port(id)
    else {
        return Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoNotFound),
            None,
            None,
            Some(String::from("destack.device.serial.open")),
            None,
            format!("virtual serial endpoint not found: {id}"),
        ))
        .boxed());
    };

    let input_signals = port.state.lock().input_signals;
    let event_queue = Arc::new(BoundedQueue::new(TEST_SERIAL_EVENT_QUEUE_CAPACITY));
    let registration_id = port.reserve_registration_id();
    let resource = Arc::new(TestSerialPortResource {
        port: Arc::clone(&port),
        event_queue: Arc::clone(&event_queue),
        state: Mutex::new(TestSerialHandleState {
            next_sequence: 1,
            reported_dropped_count: 0,
            last_signals: input_signals,
            is_read_ready_queued: false,
        }),
    });

    // install the live session before inserting the resource table entry
    port.register(registration_id, &resource);

    let entry = ResourceEntry::new(ResourceKind::SerialPort)
        .with_label(SERIAL_PORT_RESOURCE_LABEL)
        .with_payload(Arc::clone(&resource))
        .with_finalizer(binding.worker().platform_state.device.wrap_finalizer(
            TestSerialPortFinalizer {
                port,
                registration_id,
                event_queue,
            },
        ));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));

    Ok(Some(resource::SerialPortHandle(resource_id)))
}

/// Resolve one serial handle into one virtual serial resource when present.
pub(crate) fn test_serial_resource(
    binding: &BindingCallContext,
    handle: resource::SerialPortHandle,
) -> Option<Arc<TestSerialPortResource>> {
    binding.worker().resources.with_entry(handle.0, |entry| {
        if entry.kind != ResourceKind::SerialPort {
            return None;
        }

        entry.payload_cloned::<Arc<TestSerialPortResource>>()
    })?
}

/// Close one virtual serial resource and run finalization.
pub(crate) fn close_test_serial_resource(
    binding: &BindingCallContext,
    handle: resource::SerialPortHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    let kind = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| entry.kind)
        .ok_or_else(|| invalid_test_serial_handle(operation))?;
    if kind != ResourceKind::SerialPort {
        return Err(invalid_test_serial_handle(operation));
    }

    if !binding.worker().resources.remove_and_finalize(
        binding.world(),
        handle.0,
        Some(binding.engine()),
    ) {
        return Err(invalid_test_serial_handle(operation));
    }

    Ok(())
}

/// Read one virtual serial descriptor.
pub(crate) fn test_serial_descriptor(
    binding: &BindingCallContext,
    resource: &TestSerialPortResource,
) -> SerialPortDescriptor {
    resource.port.descriptor(binding)
}

/// Read one virtual serial configuration snapshot.
pub(crate) fn test_serial_config(resource: &TestSerialPortResource) -> SerialPortConfig {
    resource.port.state.lock().config
}

/// Apply one virtual serial configuration snapshot.
pub(crate) fn test_serial_configure(
    resource: &TestSerialPortResource,
    config: SerialPortConfig,
) -> RuntimeResult<()> {
    let mut state = resource.port.state.lock();

    // keep disconnected endpoints fail-closed
    if state.is_disconnected {
        return Err(disconnected_test_serial_error(
            "destack.device.serial.configure",
        ));
    }

    state.config = config;

    Ok(())
}

/// Read one virtual serial buffer with an optional timeout.
pub(crate) fn test_serial_read_into(
    resource: &TestSerialPortResource,
    buffer: NativeSlice<u8>,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<u64> {
    let buffer = unsafe { buffer.as_mut_slice()? };
    if buffer.is_empty() {
        return Ok(0);
    }

    // rearm read-ready publication before the next read attempt
    resource.state.lock().is_read_ready_queued = false;

    let deadline = timeout_deadline(timeout_ns);
    let mut state = resource.port.state.lock();

    loop {
        // consume buffered input when available
        if !state.input.is_empty() {
            let read_length = usize::min(buffer.len(), state.input.len());

            for slot in &mut buffer[..read_length] {
                if let Some(byte) = state.input.pop_front() {
                    *slot = byte;
                }
            }

            return Ok(read_length as u64);
        }

        // surface disconnect after buffered input is exhausted
        if state.is_disconnected {
            return Err(disconnected_test_serial_error(operation));
        }

        // fail loudly on expired or nonblocking waits
        let Some(deadline) = deadline else {
            return Err(io_would_block(operation, "serial read would block"));
        };
        if Instant::now() >= deadline {
            return Err(io_would_block(operation, "serial read would block"));
        }

        resource.port.input_condvar.wait_until(&mut state, deadline);
    }
}

/// Read one virtual serial buffer without blocking.
pub(crate) fn test_serial_try_read_into(
    resource: &TestSerialPortResource,
    buffer: NativeSlice<u8>,
    operation: &'static str,
) -> RuntimeResult<u64> {
    test_serial_read_into(resource, buffer, 0, operation)
}

/// Write one virtual serial buffer.
pub(crate) fn test_serial_write(
    resource: &TestSerialPortResource,
    data: NativeSlice<u8>,
    operation: &'static str,
) -> RuntimeResult<u64> {
    let data = unsafe { data.as_slice()? };
    if data.is_empty() {
        return Ok(0);
    }

    let mut state = resource.port.state.lock();

    // keep disconnected endpoints fail-closed
    if state.is_disconnected {
        return Err(disconnected_test_serial_error(operation));
    }

    // append outbound bytes before waking any drain waiters
    for byte in data {
        state.output.push_back(*byte);
    }

    resource.port.output_condvar.notify_all();

    Ok(data.len() as u64)
}

/// Wait for the virtual transmit queue to drain.
pub(crate) fn test_serial_drain(
    resource: &TestSerialPortResource,
    operation: &'static str,
) -> RuntimeResult<()> {
    let mut state = resource.port.state.lock();

    while !state.output.is_empty() && !state.is_disconnected {
        resource.port.output_condvar.wait(&mut state);
    }

    if state.is_disconnected && !state.output.is_empty() {
        return Err(disconnected_test_serial_error(operation));
    }

    Ok(())
}

/// Discard queued virtual inbound bytes.
pub(crate) fn test_serial_discard_input(resource: &TestSerialPortResource) {
    resource.port.state.lock().input.clear();
}

/// Discard queued virtual outbound bytes.
pub(crate) fn test_serial_discard_output(resource: &TestSerialPortResource) {
    let mut state = resource.port.state.lock();
    state.output.clear();

    resource.port.output_condvar.notify_all();
}

/// Read the current virtual input-signal snapshot.
pub(crate) fn test_serial_get_signals(resource: &TestSerialPortResource) -> SerialInputSignals {
    resource.port.state.lock().input_signals
}

/// Apply one virtual output-signal update.
pub(crate) fn test_serial_set_signals(
    resource: &TestSerialPortResource,
    signals: SerialOutputSignals,
    operation: &'static str,
) -> RuntimeResult<()> {
    let mut state = resource.port.state.lock();

    // keep disconnected endpoints fail-closed
    if state.is_disconnected {
        return Err(disconnected_test_serial_error(operation));
    }

    if let Some(value) = signals.data_terminal_ready {
        state.output_signals.data_terminal_ready = Some(value);
    }

    if let Some(value) = signals.request_to_send {
        state.output_signals.request_to_send = Some(value);
    }

    if let Some(value) = signals.break_condition_active {
        state.output_signals.break_condition_active = Some(value);
    }

    Ok(())
}

/// Wait for one queued virtual serial event.
pub(crate) fn test_serial_read_event(
    binding: &BindingCallContext,
    resource: &TestSerialPortResource,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<SerialEvent> {
    if let Some(dropped_count) = take_test_serial_overflow_count(resource) {
        return Ok(test_serial_overflow_event(binding, resource, dropped_count));
    }

    let timeout = Duration::from_nanos(timeout_ns);
    let Some(record) = resource.event_queue.pop_with_timeout(timeout) else {
        return Err(io_would_block(operation, "serial event would block"));
    };

    Ok(test_serial_event_from_record(binding, resource, record))
}

/// Poll one queued virtual serial event.
pub(crate) fn test_serial_try_read_event(
    binding: &BindingCallContext,
    resource: &TestSerialPortResource,
    operation: &'static str,
) -> RuntimeResult<SerialEvent> {
    if let Some(dropped_count) = take_test_serial_overflow_count(resource) {
        return Ok(test_serial_overflow_event(binding, resource, dropped_count));
    }

    let Some(record) = resource.event_queue.try_pop() else {
        return Err(io_would_block(operation, "serial event would block"));
    };

    Ok(test_serial_event_from_record(binding, resource, record))
}

/// Return one invalid virtual serial handle error.
fn invalid_test_serial_handle(operation: &'static str) -> Box<RuntimeError> {
    io_operation_error(
        operation,
        Some(PlatformErrorCode::InvalidArgumentValue),
        "unknown serial handle",
    )
}

/// Build one disconnected virtual serial error.
fn disconnected_test_serial_error(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(operation.to_string()),
        None,
        String::from("virtual serial endpoint disconnected"),
    ))
    .boxed()
}

/// Return one pending overflow delta for the virtual event queue.
fn take_test_serial_overflow_count(resource: &TestSerialPortResource) -> Option<u64> {
    let dropped_count = resource.event_queue.dropped_count();
    let mut state = resource.state.lock();
    let reported_dropped_count = state.reported_dropped_count;

    if dropped_count <= reported_dropped_count {
        return None;
    }

    let delta = dropped_count - reported_dropped_count;
    state.reported_dropped_count = dropped_count;

    Some(delta)
}

/// Return one fresh virtual serial event metadata payload.
fn next_test_serial_event_metadata(resource: &TestSerialPortResource) -> SerialEventMetadata {
    let mut state = resource.state.lock();
    let sequence = state.next_sequence;
    state.next_sequence = state.next_sequence.saturating_add(1);

    SerialEventMetadata {
        timestamp_ns: monotonic_now_ns(),
        sequence,
    }
}

/// Build one virtual overflow event.
fn test_serial_overflow_event(
    binding: &BindingCallContext,
    resource: &TestSerialPortResource,
    dropped_count: u64,
) -> SerialEvent {
    let metadata = next_test_serial_event_metadata(resource);

    SerialEvent::SerialOverflowEvent(SerialOverflowEvent {
        kind: binding.store_string("overflow"),
        metadata: SerialOverflowEventMetadata {
            timestamp_ns: metadata.timestamp_ns,
            sequence: metadata.sequence,
            dropped_count,
        },
    })
}

/// Project one queued virtual record into the public serial event surface.
fn test_serial_event_from_record(
    binding: &BindingCallContext,
    resource: &TestSerialPortResource,
    record: TestSerialEventRecord,
) -> SerialEvent {
    match record {
        TestSerialEventRecord::ReadReady { available_bytes } => {
            let metadata = next_test_serial_event_metadata(resource);

            SerialEvent::SerialReadReadyEvent(SerialReadReadyEvent {
                kind: binding.store_string("readReady"),
                metadata,
                available_bytes,
            })
        }
        TestSerialEventRecord::ModemStatusChanged { signals } => {
            let metadata = next_test_serial_event_metadata(resource);

            SerialEvent::SerialModemStatusChangedEvent(SerialModemStatusChangedEvent {
                kind: binding.store_string("modemStatusChanged"),
                metadata,
                signals,
            })
        }
        TestSerialEventRecord::Disconnected => {
            let metadata = next_test_serial_event_metadata(resource);

            SerialEvent::SerialDisconnectedEvent(SerialDisconnectedEvent {
                kind: binding.store_string("disconnected"),
                metadata,
            })
        }
    }
}

/// Store one test serial path as one platform `OsPath`.
fn store_test_serial_path(binding: &BindingCallContext, path: &str) -> fs::OsPath {
    #[cfg(unix)]
    {
        let bytes = binding.store_array_copy(path.as_bytes());

        core_fs::path_ref_from_bytes(fs::PathBytesAbi::<NativeAbi>(bytes))
    }

    #[cfg(windows)]
    {
        let utf16 = binding.store_array(path.encode_utf16().collect::<Vec<_>>());

        return core_fs::path_ref_from_utf16(fs::PathUtf16Abi::<NativeAbi>(utf16));
    }
}
