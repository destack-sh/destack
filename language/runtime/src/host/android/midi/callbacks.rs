use crate::runtime::{NativeSlice, NativeStringRef};

use super::types::{
    AndroidHostMidiEventHeader, AndroidHostMidiInputRecordHeader, AndroidHostMidiOpenedPortHeader,
    AndroidHostMidiOutputRecordHeader, AndroidHostMidiPortDescriptorHeader,
};

/// Host callback for describing Android MIDI backend support.
pub type AndroidHostMidiDescribeBackendCallback = unsafe extern "C" fn(
    runtime_id: u64,
    capability_flags: *mut u64,
    supported_data_formats: *mut u32,
    supported_protocols: *mut u32,
) -> u32;

/// Host callback for listing Android MIDI input ports.
pub type AndroidHostMidiInputPortListCallback = unsafe extern "C" fn(
    runtime_id: u64,
    flags: u32,
    headers: NativeSlice<AndroidHostMidiPortDescriptorHeader>,
    header_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32;

/// Host callback for listing Android MIDI output ports.
pub type AndroidHostMidiOutputPortListCallback = unsafe extern "C" fn(
    runtime_id: u64,
    flags: u32,
    headers: NativeSlice<AndroidHostMidiPortDescriptorHeader>,
    header_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32;

/// Host callback for opening one Android MIDI input session.
pub type AndroidHostMidiInputPortOpenCallback = unsafe extern "C" fn(
    runtime_id: u64,
    id: NativeStringRef,
    data_format: u32,
    protocol: u32,
    queue_capacity: u32,
    opened_port: *mut AndroidHostMidiOpenedPortHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32;

/// Host callback for opening one Android MIDI output session.
pub type AndroidHostMidiOutputPortOpenCallback = unsafe extern "C" fn(
    runtime_id: u64,
    id: NativeStringRef,
    data_format: u32,
    protocol: u32,
    opened_port: *mut AndroidHostMidiOpenedPortHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32;

/// Host callback for creating one Android virtual MIDI input session.
pub type AndroidHostMidiInputVirtualCreateCallback = unsafe extern "C" fn(
    runtime_id: u64,
    name: NativeStringRef,
    manufacturer: NativeStringRef,
    model: NativeStringRef,
    version: NativeStringRef,
    data_format: u32,
    protocol: u32,
    queue_capacity: u32,
    opened_port: *mut AndroidHostMidiOpenedPortHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32;

/// Host callback for creating one Android virtual MIDI output session.
pub type AndroidHostMidiOutputVirtualCreateCallback = unsafe extern "C" fn(
    runtime_id: u64,
    name: NativeStringRef,
    manufacturer: NativeStringRef,
    model: NativeStringRef,
    version: NativeStringRef,
    data_format: u32,
    protocol: u32,
    opened_port: *mut AndroidHostMidiOpenedPortHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32;

/// Host callback for closing one Android MIDI input session.
pub type AndroidHostMidiInputPortCloseCallback =
    unsafe extern "C" fn(runtime_id: u64, session_id: u64) -> u32;

/// Host callback for closing one Android MIDI output session.
pub type AndroidHostMidiOutputPortCloseCallback =
    unsafe extern "C" fn(runtime_id: u64, session_id: u64) -> u32;

/// Host callback for reading Android MIDI input records.
pub type AndroidHostMidiInputReadCallback = unsafe extern "C" fn(
    runtime_id: u64,
    session_id: u64,
    max_records: u32,
    timeout_ns: u64,
    headers: NativeSlice<AndroidHostMidiInputRecordHeader>,
    record_count_written: *mut u32,
    blob_bytes: NativeSlice<u8>,
    blob_bytes_written: *mut u32,
) -> u32;

/// Host callback for opening one Android MIDI event subscription.
pub type AndroidHostMidiEventOpenCallback = unsafe extern "C" fn(
    runtime_id: u64,
    flags: u32,
    direction_mask: u32,
    session_id: *mut u64,
) -> u32;

/// Host callback for reading Android MIDI topology events.
pub type AndroidHostMidiEventReadCallback = unsafe extern "C" fn(
    runtime_id: u64,
    session_id: u64,
    max_events: u32,
    timeout_ns: u64,
    headers: NativeSlice<AndroidHostMidiEventHeader>,
    event_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32;

/// Host callback for closing one Android MIDI event subscription.
pub type AndroidHostMidiEventCloseCallback =
    unsafe extern "C" fn(runtime_id: u64, session_id: u64) -> u32;

/// Host callback for writing Android MIDI output records.
pub type AndroidHostMidiOutputWriteCallback = unsafe extern "C" fn(
    runtime_id: u64,
    session_id: u64,
    headers: NativeSlice<AndroidHostMidiOutputRecordHeader>,
    record_count: u32,
    blob_bytes: NativeSlice<u8>,
    records_written: *mut u32,
) -> u32;

/// Host callback for flushing one Android MIDI output session.
pub type AndroidHostMidiOutputFlushCallback =
    unsafe extern "C" fn(runtime_id: u64, session_id: u64) -> u32;
