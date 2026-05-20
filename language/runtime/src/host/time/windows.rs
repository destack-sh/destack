#![allow(dead_code)]

use crate::diagnostic::RuntimeResult;
use crate::host::core as host_core;
use crate::host::time::{ClockId, ClockProperties, ClockSource};
use windows_sys::Win32::Foundation::FILETIME;
use windows_sys::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
use windows_sys::Win32::System::SystemInformation::{
    GetSystemTimePreciseAsFileTime, GetTickCount64,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentThread, GetProcessTimes, GetThreadTimes,
};

/// Number of nanoseconds in one second.
const NANOS_PER_SECOND: u64 = 1_000_000_000;
/// Number of nanoseconds in one FILETIME tick.
const FILETIME_TICK_NANOS: u64 = 100;
/// Number of nanoseconds in one millisecond.
const NANOS_PER_MILLISECOND: u64 = 1_000_000;
/// Nominal wall-clock resolution in nanoseconds for FILETIME sampling.
const WALL_RESOLUTION_NANOS: u64 = 100;

/// Convert one Win32 FILETIME value into an unsigned 64-bit tick count.
fn filetime_to_u64(value: FILETIME) -> u64 {
    ((value.dwHighDateTime as u64) << 32) | (value.dwLowDateTime as u64)
}

/// Sample one host wall clock in nanoseconds.
fn wall_nanos() -> u64 {
    // sample one precise wall clock filetime value
    let mut filetime = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };

    // SAFETY: the Win32 call writes one FILETIME to the provided out pointer
    unsafe {
        GetSystemTimePreciseAsFileTime(&mut filetime);
    }

    // convert one filetime value to nanoseconds
    let ticks = filetime_to_u64(filetime);
    ticks.saturating_mul(FILETIME_TICK_NANOS)
}

/// Sample one process CPU time in nanoseconds.
fn process_cpu_nanos() -> RuntimeResult<u64> {
    // sample one process cpu time snapshot
    let mut creation = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut exit = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut kernel = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut user = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };

    // SAFETY: pseudo process handles are valid for the current process and outputs are live
    let rc = unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    };
    if rc == 0 {
        return Err(host_core::io_error("GetProcessTimes"));
    }

    // convert one kernel and user pair into nanoseconds
    let kernel_ticks = filetime_to_u64(kernel);
    let user_ticks = filetime_to_u64(user);
    let total_ticks = kernel_ticks.saturating_add(user_ticks);
    Ok(total_ticks.saturating_mul(FILETIME_TICK_NANOS))
}

/// Sample one thread CPU time in nanoseconds.
fn thread_cpu_nanos() -> RuntimeResult<u64> {
    // sample one thread cpu time snapshot
    let mut creation = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut exit = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut kernel = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut user = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };

    // SAFETY: pseudo thread handles are valid for the current thread and outputs are live
    let rc = unsafe {
        GetThreadTimes(
            GetCurrentThread(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    };
    if rc == 0 {
        return Err(host_core::io_error("GetThreadTimes"));
    }

    // convert one kernel and user pair into nanoseconds
    let kernel_ticks = filetime_to_u64(kernel);
    let user_ticks = filetime_to_u64(user);
    let total_ticks = kernel_ticks.saturating_add(user_ticks);
    Ok(total_ticks.saturating_mul(FILETIME_TICK_NANOS))
}

/// Sample one high-resolution performance counter in nanoseconds.
fn performance_counter_nanos() -> RuntimeResult<u64> {
    // sample one high-resolution frequency
    let mut frequency = 0_i64;

    // SAFETY: the Win32 call writes one counter frequency to the provided out pointer
    let rc_frequency = unsafe { QueryPerformanceFrequency(&mut frequency) };
    if rc_frequency == 0 {
        return Err(host_core::io_error("QueryPerformanceFrequency"));
    }
    if frequency <= 0 {
        return Err(host_core::io_error("QueryPerformanceFrequency"));
    }

    // sample one high-resolution counter
    let mut counter = 0_i64;

    // SAFETY: the Win32 call writes one counter sample to the provided out pointer
    let rc_counter = unsafe { QueryPerformanceCounter(&mut counter) };
    if rc_counter == 0 {
        return Err(host_core::io_error("QueryPerformanceCounter"));
    }
    if counter < 0 {
        return Err(host_core::io_error("QueryPerformanceCounter"));
    }

    // convert one counter sample to nanoseconds
    let frequency = frequency as u64;
    let counter = counter as u64;
    let nanos = (counter as u128).saturating_mul(NANOS_PER_SECOND as u128) / (frequency as u128);
    Ok(nanos.min(u128::from(u64::MAX)) as u64)
}

/// Resolve one performance-counter resolution in nanoseconds.
fn performance_counter_resolution_nanos() -> RuntimeResult<u64> {
    // sample one high-resolution frequency
    let mut frequency = 0_i64;

    // SAFETY: the Win32 call writes one counter frequency to the provided out pointer
    let rc_frequency = unsafe { QueryPerformanceFrequency(&mut frequency) };
    if rc_frequency == 0 {
        return Err(host_core::io_error("QueryPerformanceFrequency"));
    }
    if frequency <= 0 {
        return Err(host_core::io_error("QueryPerformanceFrequency"));
    }

    // convert one frequency value to one resolution floor
    let frequency = frequency as u64;
    let nanos = (NANOS_PER_SECOND as u128) / (frequency as u128);
    let nanos = nanos.min(u128::from(u64::MAX)) as u64;
    Ok(nanos.max(1))
}

/// Sample one boot clock in nanoseconds.
fn boot_nanos() -> u64 {
    // SAFETY: GetTickCount64 has no pointer arguments and is safe to call at any time
    let milliseconds = unsafe { GetTickCount64() };

    milliseconds.saturating_mul(NANOS_PER_MILLISECOND)
}

/// Query one host clock metadata snapshot.
pub(crate) fn host_clock_metadata(clock: ClockId) -> RuntimeResult<ClockProperties> {
    // route the selected clock id
    match clock {
        ClockId::Wall => Ok(ClockProperties {
            id: ClockId::Wall,
            source: ClockSource::Realtime,
            resolution_ns: WALL_RESOLUTION_NANOS,
            is_monotonic: false,
        }),
        ClockId::Monotonic => {
            let resolution = performance_counter_resolution_nanos()?;
            Ok(ClockProperties {
                id: ClockId::Monotonic,
                source: ClockSource::PerformanceCounter,
                resolution_ns: resolution,
                is_monotonic: true,
            })
        }
        ClockId::ProcessCpu => Ok(ClockProperties {
            id: ClockId::ProcessCpu,
            source: ClockSource::Monotonic,
            resolution_ns: FILETIME_TICK_NANOS,
            is_monotonic: true,
        }),
        ClockId::ThreadCpu => Ok(ClockProperties {
            id: ClockId::ThreadCpu,
            source: ClockSource::Monotonic,
            resolution_ns: FILETIME_TICK_NANOS,
            is_monotonic: true,
        }),
        ClockId::Boot => Ok(ClockProperties {
            id: ClockId::Boot,
            source: ClockSource::Monotonic,
            resolution_ns: NANOS_PER_MILLISECOND,
            is_monotonic: true,
        }),
        ClockId::MonotonicRaw => {
            let resolution = performance_counter_resolution_nanos()?;
            Ok(ClockProperties {
                id: ClockId::MonotonicRaw,
                source: ClockSource::PerformanceCounter,
                resolution_ns: resolution,
                is_monotonic: true,
            })
        }
    }
}

/// Query one host clock sample in nanoseconds.
pub(crate) fn host_now_nanos(clock: ClockId) -> RuntimeResult<u64> {
    // route the selected clock id
    match clock {
        ClockId::Wall => Ok(wall_nanos()),
        ClockId::Monotonic => Ok(host_core::monotonic_now_ns()),
        ClockId::ProcessCpu => process_cpu_nanos(),
        ClockId::ThreadCpu => thread_cpu_nanos(),
        ClockId::Boot => Ok(boot_nanos()),
        ClockId::MonotonicRaw => performance_counter_nanos(),
    }
}

/// Query one host process CPU clock sample in nanoseconds.
pub(crate) fn host_process_cpu_nanos() -> RuntimeResult<u64> {
    process_cpu_nanos()
}

/// Query one host thread CPU clock sample in nanoseconds.
pub(crate) fn host_thread_cpu_nanos() -> RuntimeResult<u64> {
    thread_cpu_nanos()
}
