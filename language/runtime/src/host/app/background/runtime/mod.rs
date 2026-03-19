mod event;
mod launch;
mod state;

pub(super) use self::event::{
    background_expired_event, background_ready_event, complete_execution, next_background_sequence,
    publish_background_event,
};
pub(super) use self::launch::{
    enqueue_test_background_launch, trigger_test_execution, wall_clock_now_ns,
};
#[cfg(test)]
pub(crate) use self::state::with_background_test_mode;
pub(super) use self::state::{
    DESKTOP_BACKGROUND_TASK_IDENTIFIER_ENV, DesktopBackgroundExecutionState,
    desktop_background_runtime_service, desktop_background_test_mode_enabled,
};
