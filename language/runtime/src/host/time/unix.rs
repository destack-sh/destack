#![allow(dead_code)]
#![allow(clippy::missing_safety_doc)]

use std::mem::MaybeUninit;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::time::{ClockId, ClockProperties, ClockSource};
use crate::host::{HostError, core as core_platform};

/// Number of nanoseconds in one second.
const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Return one unsupported host-operation error.
#[cfg(not(target_os = "linux"))]
fn unsupported_host_operation_error(operation: &str) -> Box<RuntimeError> {
    RuntimeError::from(HostError::not_supported(operation)).boxed()
}

/// Convert one libc timespec into a saturated nanosecond value.
fn timespec_to_nanos(spec: libc::timespec, field: &str) -> RuntimeResult<u64> {
    // reject negative host timespec values
    if spec.tv_sec < 0 || spec.tv_nsec < 0 {
        return Err(RuntimeError::from(HostError::invalid_argument_value(
            field,
            "host returned one negative timespec",
        ))
        .boxed());
    }

    // convert second and nanosecond components
    let seconds = u64::try_from(spec.tv_sec).map_err(|_| {
        RuntimeError::from(HostError::invalid_argument_value(
            field,
            "seconds do not fit in uint64",
        ))
        .boxed()
    })?;
    let nanos = u64::try_from(spec.tv_nsec).map_err(|_| {
        RuntimeError::from(HostError::invalid_argument_value(
            field,
            "nanoseconds do not fit in uint64",
        ))
        .boxed()
    })?;

    // assemble one nanosecond timestamp
    let seconds_nanos = seconds.checked_mul(NANOS_PER_SECOND).ok_or_else(|| {
        RuntimeError::from(HostError::invalid_argument_value(
            field,
            "seconds overflow nanosecond conversion",
        ))
        .boxed()
    })?;

    Ok(seconds_nanos.saturating_add(nanos))
}

/// Sample one host clock value via clock_gettime.
fn clock_gettime_nanos(clock_id: libc::clockid_t, operation: &str) -> RuntimeResult<u64> {
    // query one host timespec value
    let mut spec = MaybeUninit::<libc::timespec>::uninit();
    let rc = unsafe { libc::clock_gettime(clock_id, spec.as_mut_ptr()) };
    if rc != 0 {
        return Err(core_platform::io_error(operation, None));
    }

    // convert one host timespec into nanoseconds
    let spec = unsafe { spec.assume_init() };
    timespec_to_nanos(spec, operation)
}

/// Sample one host clock resolution via clock_getres.
fn clock_getres_nanos(clock_id: libc::clockid_t, operation: &str) -> RuntimeResult<u64> {
    // query one host resolution timespec
    let mut spec = MaybeUninit::<libc::timespec>::uninit();
    let rc = unsafe { libc::clock_getres(clock_id, spec.as_mut_ptr()) };
    if rc != 0 {
        return Err(core_platform::io_error(operation, None));
    }

    // convert one host resolution into nanoseconds
    let spec = unsafe { spec.assume_init() };
    let nanos = timespec_to_nanos(spec, operation)?;
    Ok(nanos.max(1))
}

/// Sample one host process CPU time in nanoseconds.
fn process_cpu_nanos() -> RuntimeResult<u64> {
    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
        target_os = "illumos",
        target_os = "solaris"
    ))]
    {
        clock_gettime_nanos(libc::CLOCK_PROCESS_CPUTIME_ID, "clock_gettime")
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
        target_os = "illumos",
        target_os = "solaris"
    )))]
    {
        Err(unsupported_host_operation_error(
            "runtime.time.host.clock.process_cpu_nanos",
        ))
    }
}

/// Sample one host thread CPU time in nanoseconds.
fn thread_cpu_nanos() -> RuntimeResult<u64> {
    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
        target_os = "illumos",
        target_os = "solaris"
    ))]
    {
        clock_gettime_nanos(libc::CLOCK_THREAD_CPUTIME_ID, "clock_gettime")
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
        target_os = "illumos",
        target_os = "solaris"
    )))]
    {
        Err(unsupported_host_operation_error(
            "runtime.time.host.clock.thread_cpu_nanos",
        ))
    }
}

/// Sample one host boot clock in nanoseconds.
fn boot_nanos() -> RuntimeResult<u64> {
    #[cfg(target_os = "linux")]
    {
        clock_gettime_nanos(libc::CLOCK_BOOTTIME, "clock_gettime")
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(unsupported_host_operation_error(
            "runtime.time.host.clock.now_nanos",
        ))
    }
}

/// Sample one host raw monotonic clock in nanoseconds.
fn monotonic_raw_nanos() -> RuntimeResult<u64> {
    #[cfg(target_os = "linux")]
    {
        clock_gettime_nanos(libc::CLOCK_MONOTONIC_RAW, "clock_gettime")
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(unsupported_host_operation_error(
            "runtime.time.host.clock.now_nanos",
        ))
    }
}

/// Resolve one process CPU clock resolution in nanoseconds.
fn process_cpu_resolution() -> RuntimeResult<u64> {
    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
        target_os = "illumos",
        target_os = "solaris"
    ))]
    {
        clock_getres_nanos(libc::CLOCK_PROCESS_CPUTIME_ID, "clock_getres")
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
        target_os = "illumos",
        target_os = "solaris"
    )))]
    {
        Err(unsupported_host_operation_error(
            "runtime.time.host.clock.info",
        ))
    }
}

/// Resolve one thread CPU clock resolution in nanoseconds.
fn thread_cpu_resolution() -> RuntimeResult<u64> {
    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
        target_os = "illumos",
        target_os = "solaris"
    ))]
    {
        clock_getres_nanos(libc::CLOCK_THREAD_CPUTIME_ID, "clock_getres")
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
        target_os = "illumos",
        target_os = "solaris"
    )))]
    {
        Err(unsupported_host_operation_error(
            "runtime.time.host.clock.info",
        ))
    }
}

/// Resolve one boot clock resolution in nanoseconds.
fn boot_resolution() -> RuntimeResult<u64> {
    #[cfg(target_os = "linux")]
    {
        clock_getres_nanos(libc::CLOCK_BOOTTIME, "clock_getres")
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(unsupported_host_operation_error(
            "runtime.time.host.clock.info",
        ))
    }
}

/// Resolve one raw monotonic clock resolution in nanoseconds.
fn monotonic_raw_resolution() -> RuntimeResult<u64> {
    #[cfg(target_os = "linux")]
    {
        clock_getres_nanos(libc::CLOCK_MONOTONIC_RAW, "clock_getres")
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(unsupported_host_operation_error(
            "runtime.time.host.clock.info",
        ))
    }
}

/// Resolve one host wall clock sample in nanoseconds.
fn wall_nanos() -> RuntimeResult<u64> {
    clock_gettime_nanos(libc::CLOCK_REALTIME, "clock_gettime")
}

/// Resolve one host monotonic clock sample in nanoseconds.
fn mono_nanos() -> RuntimeResult<u64> {
    Ok(core_platform::monotonic_now_ns())
}

/// Query one host clock metadata snapshot.
pub(crate) fn host_clock_metadata(clock: ClockId) -> RuntimeResult<ClockProperties> {
    // route the selected clock id
    match clock {
        ClockId::Wall => {
            let resolution = clock_getres_nanos(libc::CLOCK_REALTIME, "clock_getres")?;
            Ok(ClockProperties {
                id: ClockId::Wall,
                source: ClockSource::Realtime,
                resolution_ns: resolution,
                is_monotonic: false,
            })
        }
        ClockId::Monotonic => {
            #[cfg(target_os = "macos")]
            let resolution = 1;

            #[cfg(not(target_os = "macos"))]
            let resolution = clock_getres_nanos(libc::CLOCK_MONOTONIC, "clock_getres")?;

            Ok(ClockProperties {
                id: ClockId::Monotonic,
                source: ClockSource::Monotonic,
                resolution_ns: resolution,
                is_monotonic: true,
            })
        }
        ClockId::ProcessCpu => {
            let resolution = process_cpu_resolution()?;
            Ok(ClockProperties {
                id: ClockId::ProcessCpu,
                source: ClockSource::Monotonic,
                resolution_ns: resolution,
                is_monotonic: true,
            })
        }
        ClockId::ThreadCpu => {
            let resolution = thread_cpu_resolution()?;
            Ok(ClockProperties {
                id: ClockId::ThreadCpu,
                source: ClockSource::Monotonic,
                resolution_ns: resolution,
                is_monotonic: true,
            })
        }
        ClockId::Boot => {
            let resolution = boot_resolution()?;
            Ok(ClockProperties {
                id: ClockId::Boot,
                source: ClockSource::Monotonic,
                resolution_ns: resolution,
                is_monotonic: true,
            })
        }
        ClockId::MonotonicRaw => {
            let resolution = monotonic_raw_resolution()?;
            Ok(ClockProperties {
                id: ClockId::MonotonicRaw,
                source: ClockSource::Monotonic,
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
        ClockId::Wall => wall_nanos(),
        ClockId::Monotonic => mono_nanos(),
        ClockId::ProcessCpu => process_cpu_nanos(),
        ClockId::ThreadCpu => thread_cpu_nanos(),
        ClockId::Boot => boot_nanos(),
        ClockId::MonotonicRaw => monotonic_raw_nanos(),
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
