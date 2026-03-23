use super::*;

/// Read one Android Bluetooth scan event.
pub(crate) fn read_scan_event_from_host(
    binding: &BindingCallContext,
    session_id: u64,
    timeout_ns: Option<u64>,
    try_read: bool,
    operation: &'static str,
) -> RuntimeResult<(AndroidHostBluetoothScanEventHeader, Vec<u8>)> {
    let runtime_id = host_session_id(binding, operation)?;
    let mut string_bytes = vec![0u8; INITIAL_BLUETOOTH_STRING_CAPACITY];

    loop {
        let string_capacity = checked_u32_length(string_bytes.len(), operation, "string bytes")?;
        let mut header = AndroidHostBluetoothScanEventHeader::default();
        let mut string_bytes_written = 0u32;

        let status = if try_read {
            unsafe {
                destack_host_android_bluetooth_scan_try_read_event(
                    runtime_id,
                    session_id,
                    &mut header,
                    NativeSlice {
                        data: string_bytes.as_mut_ptr(),
                        len: string_capacity,
                    },
                    &mut string_bytes_written,
                )
            }
        } else {
            unsafe {
                destack_host_android_bluetooth_scan_read_event(
                    runtime_id,
                    session_id,
                    timeout_ns.unwrap_or(0),
                    &mut header,
                    NativeSlice {
                        data: string_bytes.as_mut_ptr(),
                        len: string_capacity,
                    },
                    &mut string_bytes_written,
                )
            }
        };

        // grow the string buffer when the host reports truncation
        if status == HostStatus::BufferTooSmall.code() {
            let next_string_capacity = string_bytes
                .len()
                .max(string_bytes_written as usize)
                .saturating_mul(2);
            if next_string_capacity > MAX_BLUETOOTH_STRING_CAPACITY {
                return Err(invalid_data(
                    operation,
                    "android host bluetooth scan event exceeded the maximum string payload size",
                ));
            }

            string_bytes.resize(next_string_capacity, 0);
            continue;
        }

        host_status_result(status, operation, "bluetooth scan event read")?;

        string_bytes.truncate(string_bytes_written as usize);
        return Ok((header, string_bytes));
    }
}

/// Read one Android Bluetooth session event.
pub(crate) fn read_session_event_from_host(
    binding: &BindingCallContext,
    session_id: u64,
    timeout_ns: Option<u64>,
    try_read: bool,
    operation: &'static str,
) -> RuntimeResult<AndroidHostBluetoothSessionEventHeader> {
    let runtime_id = host_session_id(binding, operation)?;
    let mut header = AndroidHostBluetoothSessionEventHeader::default();
    let status = if try_read {
        unsafe {
            destack_host_android_bluetooth_session_try_read_event(
                runtime_id,
                session_id,
                &mut header,
            )
        }
    } else {
        unsafe {
            destack_host_android_bluetooth_session_read_event(
                runtime_id,
                session_id,
                timeout_ns.unwrap_or(0),
                &mut header,
            )
        }
    };

    host_status_result(status, operation, "bluetooth session event read")?;

    Ok(header)
}
