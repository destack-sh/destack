use crate::diagnostic::RuntimeError;
use crate::platform::timer::bindings_generated as bindings;
use crate::platform::{PlatformError, ResourceEntry, ResourceId, ResourceKind, RuntimeStatus};
use crate::runtime::with_runtime_call_context;
use crate::scheduler::Timer;

/// Schedule a one-shot timer for native code.
#[unsafe(export_name = "destack.timer.once")]
pub unsafe extern "C" fn destack_timer_once(out: *mut u64, delay_nanos: u64) -> RuntimeStatus {
    let status = with_runtime_call_context(|context| {
        let result = (|| {
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

            context.scheduler().schedule_timer(timer)?;

            unsafe {
                *out = handle.0;
            }

            Ok(())
        })();

        Ok(RuntimeStatus::from_result(result, Some(context)))
    });

    match status {
        Ok(status) => status,
        Err(error) => RuntimeStatus::from_error(error, None),
    }
}

/// Schedule a repeating timer for native code.
#[unsafe(export_name = "destack.timer.interval")]
pub unsafe extern "C" fn destack_timer_interval(out: *mut u64, period_nanos: u64) -> RuntimeStatus {
    let status = with_runtime_call_context(|context| {
        let result = (|| {
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

            context.scheduler().schedule_timer(timer)?;

            unsafe {
                *out = handle.0;
            }

            Ok(())
        })();

        Ok(RuntimeStatus::from_result(result, Some(context)))
    });

    match status {
        Ok(status) => status,
        Err(error) => RuntimeStatus::from_error(error, None),
    }
}

/// Cancel a scheduled timer for native code.
#[unsafe(export_name = "destack.timer.cancel")]
pub unsafe extern "C" fn destack_timer_cancel(handle: u64) -> RuntimeStatus {
    let status = with_runtime_call_context(|context| {
        let result = (|| {
            context.check_policy(bindings::CANCEL)?;

            context.scheduler().cancel_timer(ResourceId(handle))?;
            Ok(())
        })();

        Ok(RuntimeStatus::from_result(result, Some(context)))
    });

    match status {
        Ok(status) => status,
        Err(error) => RuntimeStatus::from_error(error, None),
    }
}
