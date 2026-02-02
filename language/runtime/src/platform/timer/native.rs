use crate::diagnostic::RuntimeError;
use crate::platform::bindings::native_call;
use crate::platform::timer::bindings_generated as bindings;
use crate::platform::{PlatformError, ResourceEntry, ResourceId, ResourceKind, RuntimeStatus};
use crate::replay::{ReplayEvent, TimeEvent, TimeEventKind};
use crate::scheduler::Timer;

/// Schedule a oneshot timer for native code.
#[unsafe(export_name = "destack.timer.once")]
pub unsafe extern "C" fn destack_timer_once(out: *mut u64, delay_nanos: u64) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::ONCE)?;

        if out.is_null() {
            return Err(RuntimeError::platform(PlatformError::null_pointer("out")).boxed());
        }

        let handle = context
            .runtime()
            .resources
            .insert(ResourceEntry::new(ResourceKind::Timer));
        let fire_at = context
            .runtime()
            .time
            .wall_nanos()
            .saturating_add(delay_nanos);

        let timer = Timer {
            handle,
            fire_at_nanos: fire_at,
            interval_nanos: None,
        };

        context
            .runtime()
            .replay
            .record_event(ReplayEvent::TimeEvent(TimeEvent {
                kind: TimeEventKind::TimerScheduled,
                time_nanos: fire_at,
                interval_nanos: None,
                timer_id: Some(handle.0),
            }));
        context.scheduler().schedule_timer(timer)?;

        unsafe {
            *out = handle.0;
        }

        Ok(())
    })
}

/// Schedule a repeating timer for native code.
#[unsafe(export_name = "destack.timer.interval")]
pub unsafe extern "C" fn destack_timer_interval(out: *mut u64, period_nanos: u64) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::INTERVAL)?;

        if out.is_null() {
            return Err(RuntimeError::platform(PlatformError::null_pointer("out")).boxed());
        }

        let handle = context
            .runtime()
            .resources
            .insert(ResourceEntry::new(ResourceKind::Timer));
        let fire_at = context
            .runtime()
            .time
            .wall_nanos()
            .saturating_add(period_nanos);

        let timer = Timer {
            handle,
            fire_at_nanos: fire_at,
            interval_nanos: Some(period_nanos),
        };

        context
            .runtime()
            .replay
            .record_event(ReplayEvent::TimeEvent(TimeEvent {
                kind: TimeEventKind::TimerScheduled,
                time_nanos: fire_at,
                interval_nanos: Some(period_nanos),
                timer_id: Some(handle.0),
            }));
        context.scheduler().schedule_timer(timer)?;

        unsafe {
            *out = handle.0;
        }

        Ok(())
    })
}

/// Cancel a scheduled timer for native code.
#[unsafe(export_name = "destack.timer.cancel")]
pub unsafe extern "C" fn destack_timer_cancel(handle: u64) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::CANCEL)?;

        context.scheduler().cancel_timer(ResourceId(handle))?;
        context
            .runtime()
            .replay
            .record_event(ReplayEvent::TimeEvent(TimeEvent {
                kind: TimeEventKind::TimerCanceled,
                time_nanos: context.runtime().time.wall_nanos(),
                interval_nanos: None,
                timer_id: Some(handle),
            }));
        Ok(())
    })
}
