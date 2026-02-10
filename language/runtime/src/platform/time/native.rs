use crate::diagnostic::RuntimeResult;
use crate::platform::time::{ClockId, ClockInfo, ClockSource, SleepClock};
use crate::runtime::RuntimeCallContext;

/// Return wall clock time in nanoseconds for native code.
pub unsafe fn destack_time_wall_ns(
    context: &RuntimeCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    let value = context.runtime().time.wall_nanos();
    unsafe {
        *out = value;
    }
    Ok(())
}

/// Return monotonic time in nanoseconds for native code.
pub unsafe fn destack_time_mono_ns(
    context: &RuntimeCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    let value = context.runtime().time.mono_nanos();
    unsafe {
        *out = value;
    }
    Ok(())
}

/// Return clock metadata for native code.
pub unsafe fn destack_time_clock_info(
    context: &RuntimeCallContext,
    out: *mut ClockInfo,
    clock: ClockId,
) -> RuntimeResult<()> {
    let _ = context;

    // map each clock to a stable metadata payload
    let info = match clock {
        ClockId::Wall => ClockInfo {
            id: ClockId::Wall,
            source: ClockSource::Realtime,
            resolution_ns: 1,
            is_monotonic: false,
        },
        ClockId::Monotonic => ClockInfo {
            id: ClockId::Monotonic,
            source: ClockSource::Monotonic,
            resolution_ns: 1,
            is_monotonic: true,
        },
        ClockId::ProcessCpu => ClockInfo {
            id: ClockId::ProcessCpu,
            source: ClockSource::PerformanceCounter,
            resolution_ns: 1,
            is_monotonic: true,
        },
        ClockId::ThreadCpu => ClockInfo {
            id: ClockId::ThreadCpu,
            source: ClockSource::PerformanceCounter,
            resolution_ns: 1,
            is_monotonic: true,
        },
        ClockId::Boot => ClockInfo {
            id: ClockId::Boot,
            source: ClockSource::Monotonic,
            resolution_ns: 1,
            is_monotonic: true,
        },
        ClockId::MonotonicRaw => ClockInfo {
            id: ClockId::MonotonicRaw,
            source: ClockSource::Monotonic,
            resolution_ns: 1,
            is_monotonic: true,
        },
    };

    // write the clock info to the output pointer
    unsafe {
        *out = info;
    }

    Ok(())
}

/// Return time in nanoseconds for the selected clock.
pub unsafe fn destack_time_now_ns(
    context: &RuntimeCallContext,
    out: *mut u64,
    clock: ClockId,
) -> RuntimeResult<()> {
    // dispatch by logical clock id
    let value = match clock {
        ClockId::Wall => context.runtime().time.wall_nanos(),
        ClockId::Monotonic => context.runtime().time.mono_nanos(),
        // NOTE #Incomplete: implement process cpu clock source
        ClockId::ProcessCpu => context.runtime().time.mono_nanos(),
        // NOTE #Incomplete: implement thread cpu clock source
        ClockId::ThreadCpu => context.runtime().time.mono_nanos(),
        // NOTE #Incomplete: implement boot clock source
        ClockId::Boot => context.runtime().time.mono_nanos(),
        // NOTE #Incomplete: implement monotonic raw clock source
        ClockId::MonotonicRaw => context.runtime().time.mono_nanos(),
    };

    // write the selected clock value to the output pointer
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Return process cpu time in nanoseconds.
pub unsafe fn destack_time_process_cpu_ns(
    context: &RuntimeCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement process cpu timer source
    let value = context.runtime().time.mono_nanos();
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Return thread cpu time in nanoseconds.
pub unsafe fn destack_time_thread_cpu_ns(
    context: &RuntimeCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement thread cpu timer source
    let value = context.runtime().time.mono_nanos();
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Sleep for the given duration for native code.
pub unsafe fn destack_time_sleep_ns(
    context: &RuntimeCallContext,
    duration_nanos: u64,
) -> RuntimeResult<()> {
    context.runtime().time.sleep_nanos(duration_nanos);
    Ok(())
}

/// Sleep for the given duration on the selected clock.
pub unsafe fn destack_time_sleep_on_ns(
    context: &RuntimeCallContext,
    duration_nanos: u64,
    clock: SleepClock,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: honor sleep clock selection
    let _ = clock;
    context.runtime().time.sleep_nanos(duration_nanos);

    Ok(())
}

/// Sleep until the given deadline for native code.
pub unsafe fn destack_time_sleep_until_ns(
    context: &RuntimeCallContext,
    deadline_nanos: u64,
) -> RuntimeResult<()> {
    context.runtime().time.sleep_until_nanos(deadline_nanos);
    Ok(())
}

/// Sleep until the given deadline on the selected clock.
pub unsafe fn destack_time_sleep_until_on_ns(
    context: &RuntimeCallContext,
    deadline_nanos: u64,
    clock: SleepClock,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: honor sleep clock selection
    let _ = clock;
    context.runtime().time.sleep_until_nanos(deadline_nanos);

    Ok(())
}
