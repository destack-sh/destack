use std::sync::OnceLock;

use crate::diagnostic::RuntimeError;
use crate::host::HostError;
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::Networking::WinSock::WSAGetLastError;
use windows_sys::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

/// Nanoseconds per second.
const NANOS_PER_SECOND: u128 = 1_000_000_000;

/// Process-relative epoch for QPC monotonic normalization.
static QPC_MONOTONIC_EPOCH_TICKS: OnceLock<u64> = OnceLock::new();

/// Return the last Winsock error code.
pub(crate) fn last_wsa_error_code() -> i32 {
    // SAFETY: WSAGetLastError has no arguments and reads thread-local Winsock state
    unsafe { WSAGetLastError() }
}

/// Build an I/O runtime error from the last Win32 error value.
pub(crate) fn io_error(syscall: &str) -> Box<RuntimeError> {
    // SAFETY: GetLastError has no arguments and reads thread-local Win32 state
    let errno = unsafe { GetLastError() } as i32;
    let message = error_message(syscall, errno);
    RuntimeError::from(HostError::io_with(
        None,
        None,
        Some(errno),
        Some(syscall.to_string()),
        None,
        message,
    ))
    .boxed()
}

/// Format a simple Windows syscall error message.
pub(crate) fn error_message(syscall: &str, code: i32) -> String {
    format!("{syscall} failed: {code}")
}

/// Read one process-relative monotonic timestamp from QueryPerformanceCounter.
pub(crate) fn qpc_process_monotonic_nanos() -> Option<u64> {
    let epoch_ticks = qpc_process_epoch_ticks()?;
    let now_ticks = qpc_now_ticks()?;
    let delta_ticks = now_ticks.saturating_sub(epoch_ticks);

    qpc_ticks_to_ns(delta_ticks)
}

/// Return one cached QueryPerformanceCounter frequency.
fn qpc_frequency_hz() -> u64 {
    static QPC_FREQUENCY_HZ: OnceLock<u64> = OnceLock::new();

    *QPC_FREQUENCY_HZ.get_or_init(|| {
        let mut frequency = 0i64;

        // SAFETY: the Win32 call writes one counter frequency to the provided out pointer
        let status = unsafe { QueryPerformanceFrequency(&mut frequency) };
        if status == 0 || frequency <= 0 {
            return 0;
        }

        frequency as u64
    })
}

/// Return one cached process-relative QPC epoch sample.
fn qpc_process_epoch_ticks() -> Option<u64> {
    if let Some(epoch_ticks) = QPC_MONOTONIC_EPOCH_TICKS.get().copied() {
        return Some(epoch_ticks);
    }

    let sample_ticks = qpc_now_ticks()?;
    let _ = QPC_MONOTONIC_EPOCH_TICKS.set(sample_ticks);

    match QPC_MONOTONIC_EPOCH_TICKS.get().copied() {
        Some(epoch_ticks) => Some(epoch_ticks),
        None => Some(sample_ticks),
    }
}

/// Read one QueryPerformanceCounter tick value.
fn qpc_now_ticks() -> Option<u64> {
    let mut counter = 0i64;

    // SAFETY: the Win32 call writes one counter sample to the provided out pointer
    let status = unsafe { QueryPerformanceCounter(&mut counter) };
    if status == 0 || counter < 0 {
        return None;
    }

    Some(counter as u64)
}

/// Convert one QPC tick value into nanoseconds.
fn qpc_ticks_to_ns(counter: u64) -> Option<u64> {
    let frequency = qpc_frequency_hz();
    if frequency == 0 {
        return None;
    }

    Some(((u128::from(counter) * NANOS_PER_SECOND) / u128::from(frequency)) as u64)
}
