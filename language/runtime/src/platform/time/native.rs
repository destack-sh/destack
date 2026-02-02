use crate::diagnostic::RuntimeError;
use crate::platform::bindings::native_call;
use crate::platform::time::bindings_generated as bindings;
use crate::platform::{PlatformError, RuntimeStatus};
use crate::replay::{ReplayEvent, TimeEvent, TimeEventKind};

/// Return wall clock time in nanoseconds for native code.
#[unsafe(export_name = "destack.time.wallNs")]
pub unsafe extern "C" fn destack_time_wall_ns(out: *mut u64) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::WALL_NS)?;

        if out.is_null() {
            return Err(RuntimeError::platform(PlatformError::null_pointer("out")).boxed());
        }

        let value = context.runtime().time.wall_nanos();
        context
            .runtime()
            .replay
            .record_event(ReplayEvent::TimeEvent(TimeEvent {
                kind: TimeEventKind::WallClockRead,
                time_nanos: value,
                interval_nanos: None,
                timer_id: None,
            }));
        unsafe {
            *out = value;
        }

        Ok(())
    })
}

/// Return monotonic time in nanoseconds for native code.
#[unsafe(export_name = "destack.time.monoNs")]
pub unsafe extern "C" fn destack_time_mono_ns(out: *mut u64) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::MONO_NS)?;

        if out.is_null() {
            return Err(RuntimeError::platform(PlatformError::null_pointer("out")).boxed());
        }

        let value = context.runtime().time.mono_nanos();
        context
            .runtime()
            .replay
            .record_event(ReplayEvent::TimeEvent(TimeEvent {
                kind: TimeEventKind::MonotonicSample,
                time_nanos: value,
                interval_nanos: None,
                timer_id: None,
            }));
        unsafe {
            *out = value;
        }

        Ok(())
    })
}

/// Sleep for the given duration for native code.
#[unsafe(export_name = "destack.time.sleepNs")]
pub unsafe extern "C" fn destack_time_sleep_ns(duration_nanos: u64) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::SLEEP_NS)?;
        let start = context.runtime().time.wall_nanos();
        let deadline = start.saturating_add(duration_nanos);
        context
            .runtime()
            .replay
            .record_event(ReplayEvent::TimeEvent(TimeEvent {
                kind: TimeEventKind::SleepScheduled,
                time_nanos: deadline,
                interval_nanos: None,
                timer_id: None,
            }));
        context.runtime().time.sleep_nanos(duration_nanos);
        let wake = context.runtime().time.wall_nanos();
        context
            .runtime()
            .replay
            .record_event(ReplayEvent::TimeEvent(TimeEvent {
                kind: TimeEventKind::SleepWake,
                time_nanos: wake,
                interval_nanos: None,
                timer_id: None,
            }));
        Ok(())
    })
}

/// Sleep until the given deadline for native code.
#[unsafe(export_name = "destack.time.sleepUntilNs")]
pub unsafe extern "C" fn destack_time_sleep_until_ns(deadline_nanos: u64) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::SLEEP_UNTIL_NS)?;
        context
            .runtime()
            .replay
            .record_event(ReplayEvent::TimeEvent(TimeEvent {
                kind: TimeEventKind::SleepScheduled,
                time_nanos: deadline_nanos,
                interval_nanos: None,
                timer_id: None,
            }));
        context.runtime().time.sleep_until_nanos(deadline_nanos);
        let wake = context.runtime().time.wall_nanos();
        context
            .runtime()
            .replay
            .record_event(ReplayEvent::TimeEvent(TimeEvent {
                kind: TimeEventKind::SleepWake,
                time_nanos: wake,
                interval_nanos: None,
                timer_id: None,
            }));
        Ok(())
    })
}
