use crate::host::android::abi::midi::tests::callbacks::test_callbacks;
use crate::host::android::tests::{
    callback_test_lock, register_android_bindings_midi, register_android_runtime,
};
use crate::host::core::HostQueue;
use crate::host::core::registry::HostSessionRegistrationGuard;
use crate::host::{HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK};
use crate::runtime::NativeStringRef;
use std::sync::{Arc, Mutex, OnceLock};

/// One test Android MIDI capability bitset.
pub(super) const TEST_CAPABILITY_FLAGS: u64 = 0x33;
/// One test Android MIDI data-format bitset.
pub(super) const TEST_DATA_FORMATS: u32 = 0x7;
/// One test Android MIDI protocol bitset.
pub(super) const TEST_PROTOCOLS: u32 = 0x9;
/// One test opened input session id.
pub(super) const TEST_INPUT_SESSION_ID: u64 = 101;
/// One test opened output session id.
pub(super) const TEST_OUTPUT_SESSION_ID: u64 = 202;
/// One test input source id.
pub(super) const TEST_SOURCE_ID: &str = "android.source";
/// One test event session id.
pub(super) const TEST_EVENT_SESSION_ID: u64 = 303;
/// One test descriptor id.
pub(super) const TEST_DESCRIPTOR_ID: &str = "android.endpoint";
/// One test descriptor name.
pub(super) const TEST_DESCRIPTOR_NAME: &str = "Android Endpoint";
/// One test manufacturer name.
pub(super) const TEST_MANUFACTURER: &str = "Destack";
/// One test model name.
pub(super) const TEST_MODEL: &str = "Bridge";
/// One test version string.
pub(super) const TEST_VERSION: &str = "1";

/// Return one runtime-not-found status for unregistered binding tests.
pub(super) const TEST_STATUS_NOT_FOUND: u32 = HOST_STATUS_NOT_FOUND;
/// Return one callback-not-supported status for unset binding tests.
pub(super) const TEST_STATUS_NOT_SUPPORTED: u32 = HOST_STATUS_NOT_SUPPORTED;

/// One live Android MIDI test callback registration.
pub(super) struct AndroidMidiTestCallbacks {
    /// The runtime id bound to the registered callback table.
    pub runtime_id: u64,
    /// The host queue kept alive for the runtime registration.
    _queue: Arc<HostQueue>,
    /// The host registration guard kept alive for the callback table.
    _registration: HostSessionRegistrationGuard,
}

/// One recorded input-port open request.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedInputOpenCall {
    /// The requested endpoint id.
    pub id: String,
    /// The requested data-format code.
    pub data_format: u32,
    /// The requested protocol code.
    pub protocol: u32,
    /// The requested queue capacity.
    pub queue_capacity: u32,
}

/// One recorded output-port open request.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedOutputOpenCall {
    /// The requested endpoint id.
    pub id: String,
    /// The requested data-format code.
    pub data_format: u32,
    /// The requested protocol code.
    pub protocol: u32,
}

/// One recorded virtual-input creation request.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedInputVirtualCreateCall {
    /// The requested endpoint name.
    pub name: String,
    /// The requested manufacturer string.
    pub manufacturer: String,
    /// The requested model string.
    pub model: String,
    /// The requested version string.
    pub version: String,
    /// The requested data-format code.
    pub data_format: u32,
    /// The requested protocol code.
    pub protocol: u32,
    /// The requested queue capacity.
    pub queue_capacity: u32,
}

/// One recorded virtual-output creation request.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedOutputVirtualCreateCall {
    /// The requested endpoint name.
    pub name: String,
    /// The requested manufacturer string.
    pub manufacturer: String,
    /// The requested model string.
    pub model: String,
    /// The requested version string.
    pub version: String,
    /// The requested data-format code.
    pub data_format: u32,
    /// The requested protocol code.
    pub protocol: u32,
}

/// One recorded event-open request.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedEventOpenCall {
    /// The requested subscription flags.
    pub flags: u32,
    /// The requested direction mask.
    pub direction_mask: u32,
}

/// One recorded output-write record.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedOutputWriteRecord {
    /// The requested send timestamp.
    pub send_at_ns: u64,
    /// Whether the send timestamp is present.
    pub has_send_at: u32,
    /// The requested data-format code.
    pub data_format: u32,
    /// The requested protocol code.
    pub protocol: u32,
    /// The requested framing code.
    pub framing: u32,
    /// The encoded payload bytes.
    pub data: Vec<u8>,
}

/// One recorded output-write call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedOutputWriteCall {
    /// The targeted output session.
    pub session_id: u64,
    /// The forwarded output records.
    pub records: Vec<RecordedOutputWriteRecord>,
}

/// One snapshot of the recorded Android MIDI callback traffic.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct AndroidMidiTestState {
    /// The most recent input-port open call.
    pub input_port_open: Option<RecordedInputOpenCall>,
    /// The most recent output-port open call.
    pub output_port_open: Option<RecordedOutputOpenCall>,
    /// The most recent virtual-input create call.
    pub input_virtual_create: Option<RecordedInputVirtualCreateCall>,
    /// The most recent virtual-output create call.
    pub output_virtual_create: Option<RecordedOutputVirtualCreateCall>,
    /// The most recent event-open call.
    pub event_open: Option<RecordedEventOpenCall>,
    /// The most recent output-write call.
    pub output_write: Option<RecordedOutputWriteCall>,
    /// Closed input session ids.
    pub closed_input_sessions: Vec<u64>,
    /// Closed output session ids.
    pub closed_output_sessions: Vec<u64>,
    /// Closed event session ids.
    pub closed_event_sessions: Vec<u64>,
}

/// Return the shared recorded-callback state.
pub(super) fn test_state() -> &'static Mutex<AndroidMidiTestState> {
    static TEST_STATE: OnceLock<Mutex<AndroidMidiTestState>> = OnceLock::new();

    TEST_STATE.get_or_init(|| Mutex::new(AndroidMidiTestState::default()))
}

/// Reset the recorded-callback state for one test case.
fn reset_test_state() {
    *test_state().lock().unwrap() = AndroidMidiTestState::default();
}

/// Return one snapshot of the recorded-callback state.
pub(super) fn recorded_test_state() -> AndroidMidiTestState {
    test_state().lock().unwrap().clone()
}

/// Return one native string reference for the given string.
pub(super) fn native_string_ref(value: &str) -> NativeStringRef {
    NativeStringRef {
        data: value.as_ptr().cast_mut(),
        len: value.len() as u32,
    }
}

/// Append one string to the shared output buffer and return its offset and length.
pub(super) fn append_string(buffer: &mut Vec<u8>, value: &str) -> (u32, u32) {
    let offset = buffer.len() as u32;
    buffer.extend_from_slice(value.as_bytes());

    (offset, value.len() as u32)
}

/// Decode one native string reference into one owned Rust string.
pub(super) fn decode_native_string(value: NativeStringRef) -> String {
    if value.data.is_null() || value.len == 0 {
        return String::new();
    }

    let bytes = unsafe { std::slice::from_raw_parts(value.data, value.len as usize) };

    std::str::from_utf8(bytes)
        .expect("android midi test strings should be utf8")
        .to_string()
}

/// Register one live Android MIDI callback table for one test case.
pub(super) fn register_test_callbacks() -> AndroidMidiTestCallbacks {
    let (queue, registration, runtime_id) = register_android_runtime();
    reset_test_state();

    let status = register_android_bindings_midi(runtime_id, test_callbacks());
    assert_eq!(status, HOST_STATUS_OK);

    AndroidMidiTestCallbacks {
        runtime_id,
        _queue: queue,
        _registration: registration,
    }
}

/// Lock Android host callback registration for one test case.
pub(super) fn lock_test_callbacks() -> std::sync::MutexGuard<'static, ()> {
    callback_test_lock().lock().unwrap()
}
