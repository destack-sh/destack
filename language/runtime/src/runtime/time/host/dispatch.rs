use crate::diagnostic::RuntimeResult;
use crate::platform::time::{ClockId, ClockMetadata, ClockSource, SleepClock};
use crate::runtime::time::core as time_core;
use crate::runtime::{BindingCallContext, RuntimeHookState};

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
fn virtual_clock_info(clock: ClockId) -> Option<ClockMetadata> {
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

/// Query one selected clock metadata.
pub(crate) unsafe fn host_clock_info(
    context: &BindingCallContext,
    out: *mut ClockMetadata,
    clock: ClockId,
) -> RuntimeResult<()> {
    // route metadata reads through runtime policy and host backend
    let info = if time_core::is_virtual_clock(context) {
        virtual_clock_info(clock).unwrap_or(host_time::host_clock_info(clock)?)
    } else {
        host_time::host_clock_info(clock)?
    };

    // write the metadata result
    unsafe { time_core::write_out_clock_info(out, info) }
}

/// Return monotonic time in nanoseconds.
pub(crate) unsafe fn host_mono_nanos(
    context: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    // read monotonic time from the runtime clock service
    let value = time_core::runtime_mono_nanos(context);

    // write the monotonic timestamp
    unsafe { time_core::write_out_u64(out, value) }
}

/// Read one selected clock in nanoseconds.
pub(crate) unsafe fn host_now_nanos(
    context: &BindingCallContext,
    out: *mut u64,
    clock: ClockId,
) -> RuntimeResult<()> {
    // route the selected clock through runtime policy and host backend
    let value = match clock {
        ClockId::Wall => time_core::runtime_wall_nanos(context),
        ClockId::Monotonic => time_core::runtime_mono_nanos(context),
        ClockId::ProcessCpu => host_time::host_process_cpu_nanos()?,
        ClockId::ThreadCpu => host_time::host_thread_cpu_nanos()?,
        ClockId::Boot | ClockId::MonotonicRaw => {
            if time_core::is_virtual_clock(context) {
                time_core::runtime_mono_nanos(context)
            } else {
                host_time::host_now_nanos(clock)?
            }
        }
    };

    // write the sampled clock value
    unsafe { time_core::write_out_u64(out, value) }
}

/// Return process CPU time in nanoseconds.
pub(crate) unsafe fn host_process_cpu_nanos(
    context: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    context
        .hooks()
        .on_time_read(RuntimeHookState::from_engine(Some(context.engine())));

    // sample process cpu time from the host backend
    let value = host_time::host_process_cpu_nanos()?;

    // write the sampled cpu time
    unsafe { time_core::write_out_u64(out, value) }
}

/// Return current thread CPU time in nanoseconds.
pub(crate) unsafe fn host_thread_cpu_nanos(
    context: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    context
        .hooks()
        .on_time_read(RuntimeHookState::from_engine(Some(context.engine())));

    // sample thread cpu time from the host backend
    let value = host_time::host_thread_cpu_nanos()?;

    // write the sampled cpu time
    unsafe { time_core::write_out_u64(out, value) }
}

/// Return wall clock time in nanoseconds.
pub(crate) unsafe fn host_wall_nanos(
    context: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    // read wall time from the runtime clock service
    let value = time_core::runtime_wall_nanos(context);

    // write the wall timestamp
    unsafe { time_core::write_out_u64(out, value) }
}

/// Sleep for one duration in nanoseconds.
pub(crate) unsafe fn host_sleep_nanos(
    context: &BindingCallContext,
    duration: u64,
) -> RuntimeResult<()> {
    // route sleep through virtual or host mode behavior
    if time_core::is_virtual_clock(context) {
        time_core::runtime_sleep_nanos(context, duration);
        return Ok(());
    }

    host_time::host_sleep_nanos(duration)
}

/// Sleep for one duration on one clock domain.
pub(crate) unsafe fn host_sleep_on_nanos(
    context: &BindingCallContext,
    duration: u64,
    clock: SleepClock,
) -> RuntimeResult<()> {
    // route clock-domain sleep through runtime policy
    if time_core::is_virtual_clock(context) {
        match clock {
            SleepClock::Wall | SleepClock::Monotonic => {
                time_core::runtime_sleep_nanos(context, duration)
            }
        }

        return Ok(());
    }

    host_time::host_sleep_nanos(duration)
}

/// Sleep until one wall deadline in nanoseconds.
pub(crate) unsafe fn host_sleep_until_nanos(
    context: &BindingCallContext,
    deadline: u64,
) -> RuntimeResult<()> {
    // route wall-deadline sleep through runtime policy
    if time_core::is_virtual_clock(context) {
        time_core::runtime_sleep_until_wall_nanos(context, deadline);
        return Ok(());
    }

    // convert absolute wall deadline to one host sleep duration
    let now = time_core::runtime_wall_nanos(context);
    if deadline <= now {
        return Ok(());
    }

    let duration = deadline.saturating_sub(now);
    host_time::host_sleep_nanos(duration)
}

/// Sleep until one deadline on one clock domain.
pub(crate) unsafe fn host_sleep_until_on_nanos(
    context: &BindingCallContext,
    deadline: u64,
    clock: SleepClock,
) -> RuntimeResult<()> {
    // route domain deadline sleep through runtime policy
    if time_core::is_virtual_clock(context) {
        match clock {
            SleepClock::Wall => time_core::runtime_sleep_until_wall_nanos(context, deadline),
            SleepClock::Monotonic => time_core::runtime_sleep_until_mono_nanos(context, deadline),
        }

        return Ok(());
    }

    // convert absolute domain deadline to one host sleep duration
    let now = match clock {
        SleepClock::Wall => time_core::runtime_wall_nanos(context),
        SleepClock::Monotonic => time_core::runtime_mono_nanos(context),
    };
    if deadline <= now {
        return Ok(());
    }

    let duration = deadline.saturating_sub(now);
    host_time::host_sleep_nanos(duration)
}
