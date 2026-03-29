use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeStringRef;
use crate::platform::core::{self as core_platform};
use crate::platform::device::midi::core::binding_timestamp_now;

use super::super::abi::MIDITimeStamp;

/// Build one CoreMIDI operation error.
pub(crate) fn core_midi_status_error(
    operation: &'static str,
    action: &str,
    status: i32,
) -> Box<RuntimeError> {
    core_platform::io_operation_error(
        operation,
        None,
        format!("{action} failed with CoreMIDI status {status}"),
    )
}

/// Convert one CoreMIDI host timestamp into runtime monotonic nanoseconds.
pub(crate) fn core_midi_host_time_to_mono_ns(time_stamp: MIDITimeStamp) -> u64 {
    if time_stamp == 0 {
        return binding_timestamp_now();
    }

    core_platform::apple_host_time_to_process_nanos(time_stamp)
}

/// Convert one runtime monotonic nanosecond timestamp into one CoreMIDI host timestamp.
pub(crate) fn core_midi_mono_ns_to_host_time(send_at_ns: Option<u64>) -> MIDITimeStamp {
    match send_at_ns {
        Some(send_at_ns) if send_at_ns != 0 => {
            core_platform::apple_process_nanos_to_host_time(send_at_ns)
        }
        _ => 0,
    }
}

/// Decode one native binding string into one owned string.
pub(crate) fn native_string(value: NativeStringRef) -> RuntimeResult<String> {
    let value = unsafe { value.as_str()? };

    Ok(value.to_string())
}

/// Decode one optional native binding string.
pub(crate) fn native_optional_string(
    value: Option<NativeStringRef>,
) -> RuntimeResult<Option<String>> {
    match value {
        Some(value) => Ok(Some(native_string(value)?)),
        None => Ok(None),
    }
}
