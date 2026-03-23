use crate::diagnostic::RuntimeResult;
use crate::host::android::ingress::core::android_host_queue;
use crate::host::core::HostSessionHandle;
use crate::host::{HostEvent, HostLifecycleEvent, HostLifecycleSourceKind, HostLifecycleState};

/// Android activity lifecycle transitions from native callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum AndroidActivityLifecycle {
    /// Activity was created (`onCreate`).
    Created,
    /// Activity moved to started state (`onStart`).
    Started,
    /// Activity moved to resumed state (`onResume`).
    Resumed,
    /// Activity moved to paused state (`onPause`).
    Paused,
    /// Activity moved to stopped state (`onStop`).
    Stopped,
    /// Activity was destroyed (`onDestroy`).
    Destroyed,
}

/// Submit one Android activity lifecycle callback.
pub(crate) fn android_notify_activity_lifecycle(
    session_handle: HostSessionHandle,
    lifecycle: AndroidActivityLifecycle,
) -> RuntimeResult<()> {
    let queue = android_host_queue(session_handle)?;
    let state = host_lifecycle_state_for_android_activity(lifecycle);

    queue.enqueue(HostEvent::Lifecycle(HostLifecycleEvent {
        source_kind: HostLifecycleSourceKind::Activity,
        state,
    }));

    Ok(())
}

/// Map one Android activity lifecycle transition to host lifecycle state.
pub(crate) fn host_lifecycle_state_for_android_activity(
    lifecycle: AndroidActivityLifecycle,
) -> HostLifecycleState {
    match lifecycle {
        AndroidActivityLifecycle::Created => HostLifecycleState::Initializing,
        AndroidActivityLifecycle::Started => HostLifecycleState::Running,
        AndroidActivityLifecycle::Resumed => HostLifecycleState::Running,
        AndroidActivityLifecycle::Paused => HostLifecycleState::Paused,
        AndroidActivityLifecycle::Stopped => HostLifecycleState::Stopped,
        AndroidActivityLifecycle::Destroyed => HostLifecycleState::Destroyed,
    }
}
