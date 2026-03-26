mod event;
mod launch;
mod state;

pub(crate) use self::event::{
    background_expired_event, background_ready_event, complete_execution, next_background_sequence,
    publish_background_event,
};
pub(crate) use self::launch::{
    DESKTOP_BACKGROUND_DEADLINE_UNIX_NS_ENV, desktop_background_execution_deadline_ns,
    enqueue_test_background_launch, wall_clock_now_ns,
};
#[cfg(test)]
pub(crate) use self::state::with_background_test_mode;
pub(crate) use self::state::{
    DESKTOP_BACKGROUND_TASK_IDENTIFIER_ENV, DesktopBackgroundExecutionState,
    DesktopBackgroundLaunchMarker, desktop_background_runtime_service,
    desktop_background_test_mode_enabled,
};
