use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::time::{ClockId, ClockMetadata, ClockSource, SleepClock};
use crate::runtime::BindingCallContext;

#[cfg(unix)]
#[path = "../unix/mod.rs"]
mod unix;
#[cfg(not(any(unix, windows)))]
#[path = "../unsupported.rs"]
mod unsupported;
#[cfg(windows)]
#[path = "../windows/mod.rs"]
mod windows;

#[cfg(unix)]
use unix as host_time;
#[cfg(not(any(unix, windows)))]
use unsupported as host_time;
#[cfg(windows)]
use windows as host_time;

/// Return one virtual clock metadata payload when the runtime is virtualized.
fn virtual_clock_metadata(clock: ClockId) -> Option<ClockMetadata> {
    // return the virtualized metadata for virtual-clock domains
    match clock {
        ClockId::Wall => Some(ClockMetadata {
            id: ClockId::Wall,
            source: ClockSource::Virtual,
            resolution_ns: 1,
            is_monotonic: false,
        }),
        ClockId::Monotonic => Some(ClockMetadata {
            id: ClockId::Monotonic,
            source: ClockSource::Virtual,
            resolution_ns: 1,
            is_monotonic: true,
        }),
        ClockId::Boot => Some(ClockMetadata {
            id: ClockId::Boot,
            source: ClockSource::Virtual,
            resolution_ns: 1,
            is_monotonic: true,
        }),
        ClockId::MonotonicRaw => Some(ClockMetadata {
            id: ClockId::MonotonicRaw,
            source: ClockSource::Virtual,
            resolution_ns: 1,
            is_monotonic: true,
        }),
        ClockId::ProcessCpu | ClockId::ThreadCpu => None,
    }
}

/// Return one invalid-pointer error.
fn null_pointer_error(field: &str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::null_pointer(field)).boxed()
}

/// Query one selected clock metadata.
pub(crate) unsafe fn host_clock_metadata(
    context: &BindingCallContext,
    out: *mut ClockMetadata,
    clock: ClockId,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(null_pointer_error("out"));
    }

    // route metadata reads through runtime policy and host backend
    let info = if context.is_virtual_clock() {
        virtual_clock_metadata(clock).unwrap_or(host_time::host_clock_metadata(clock)?)
    } else {
        host_time::host_clock_metadata(clock)?
    };

    // write the metadata result
    unsafe {
        *out = info;
    }

    Ok(())
}

/// Return monotonic time in nanoseconds.
pub(crate) unsafe fn host_mono_nanos(
    context: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(null_pointer_error("out"));
    }

    // mark one explicit time-read operation
    context.on_time_read();

    // read monotonic time from the runtime clock service
    let value = context.mono_nanos();

    // write the monotonic timestamp
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read one selected clock in nanoseconds.
pub(crate) unsafe fn host_now_nanos(
    context: &BindingCallContext,
    out: *mut u64,
    clock: ClockId,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(null_pointer_error("out"));
    }

    // route the selected clock through runtime policy and host backend
    let value = match clock {
        ClockId::Wall => {
            context.on_time_read();
            context.wall_nanos()
        }
        ClockId::Monotonic => {
            context.on_time_read();
            context.mono_nanos()
        }
        ClockId::ProcessCpu => {
            context.on_time_read();
            host_time::host_process_cpu_nanos()?
        }
        ClockId::ThreadCpu => {
            context.on_time_read();
            host_time::host_thread_cpu_nanos()?
        }
        ClockId::Boot | ClockId::MonotonicRaw => {
            context.on_time_read();
            if context.is_virtual_clock() {
                context.mono_nanos()
            } else {
                host_time::host_now_nanos(clock)?
            }
        }
    };

    // write the sampled clock value
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Return process CPU time in nanoseconds.
pub(crate) unsafe fn host_process_cpu_nanos(
    context: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(null_pointer_error("out"));
    }

    // mark one explicit time-read operation
    context.on_time_read();

    // sample process cpu time from the host backend
    let value = host_time::host_process_cpu_nanos()?;

    // write the sampled cpu time
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Return current thread CPU time in nanoseconds.
pub(crate) unsafe fn host_thread_cpu_nanos(
    context: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(null_pointer_error("out"));
    }

    // mark one explicit time-read operation
    context.on_time_read();

    // sample thread cpu time from the host backend
    let value = host_time::host_thread_cpu_nanos()?;

    // write the sampled cpu time
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Return wall clock time in nanoseconds.
pub(crate) unsafe fn host_wall_nanos(
    context: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(null_pointer_error("out"));
    }

    // mark one explicit time-read operation
    context.on_time_read();

    // read wall time from the runtime clock service
    let value = context.wall_nanos();

    // write the wall timestamp
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Sleep for one duration in nanoseconds.
pub(crate) unsafe fn host_sleep_nanos(
    context: &BindingCallContext,
    duration: u64,
) -> RuntimeResult<()> {
    // route sleep through virtual or host mode behavior
    if context.is_virtual_clock() {
        let _ = duration;

        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.time.sleep.ns")).boxed(),
        );
    }

    // host mode: block the current thread directly
    context.sleep_nanos(duration);

    Ok(())
}

/// Sleep for one duration on one clock domain.
pub(crate) unsafe fn host_sleep_on_nanos(
    context: &BindingCallContext,
    duration: u64,
    clock: SleepClock,
) -> RuntimeResult<()> {
    // route clock-domain sleep through runtime policy
    if context.is_virtual_clock() {
        let _ = (duration, clock);

        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.time.sleepOn.ns")).boxed(),
        );
    }

    // host mode: delegate to the selected clock domain
    match clock {
        SleepClock::Wall | SleepClock::Monotonic => context.sleep_nanos(duration),
    }

    Ok(())
}

/// Sleep until one wall deadline in nanoseconds.
pub(crate) unsafe fn host_sleep_until_nanos(
    context: &BindingCallContext,
    deadline: u64,
) -> RuntimeResult<()> {
    // route wall-deadline sleep through runtime policy
    if context.is_virtual_clock() {
        let _ = deadline;

        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.time.sleepUntil.ns")).boxed(),
        );
    }

    // convert absolute wall deadline to one host sleep duration
    let now = context.wall_nanos();
    if deadline <= now {
        return Ok(());
    }

    let duration = deadline.saturating_sub(now);
    context.sleep_nanos(duration);

    Ok(())
}

/// Sleep until one deadline on one clock domain.
pub(crate) unsafe fn host_sleep_until_on_nanos(
    context: &BindingCallContext,
    deadline: u64,
    clock: SleepClock,
) -> RuntimeResult<()> {
    // route domain deadline sleep through runtime policy
    if context.is_virtual_clock() {
        let _ = (deadline, clock);

        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.time.sleepUntilOn.ns",
        ))
        .boxed());
    }

    // convert absolute domain deadline to one host sleep duration
    let now = match clock {
        SleepClock::Wall => context.wall_nanos(),
        SleepClock::Monotonic => context.mono_nanos(),
    };
    if deadline <= now {
        return Ok(());
    }

    let duration = deadline.saturating_sub(now);
    context.sleep_nanos(duration);

    Ok(())
}
