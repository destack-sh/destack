use crate::diagnostic::RuntimeResult;
use crate::platform::{ResourceTable, resource};
use crate::runtime::time::Clock;
use destack_workspace::TimeMode;

/// Update runtime state when one event-loop timer fires.
pub(crate) fn on_event_loop_timer_fire(
    _resources: &ResourceTable,
    _clock: &Clock,
    _time_mode: TimeMode,
    _handle: resource::TimerHandle,
) -> RuntimeResult<bool> {
    Ok(true)
}
