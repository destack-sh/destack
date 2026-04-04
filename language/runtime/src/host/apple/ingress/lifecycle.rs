use crate::diagnostic::RuntimeResult;
use crate::host::os::apple::ingress::core::ios_host_queue;
use crate::host::{
    HostEvent, HostLifecycleEvent, HostLifecycleSourceKind, HostLifecycleState, HostSessionHandle,
};

/// iOS application lifecycle transitions from native callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum IosApplicationLifecycle {
    /// App launch finished and normal processing can begin.
    DidFinishLaunching,
    /// App became active in the foreground.
    DidBecomeActive,
    /// App is resigning active foreground state.
    WillResignActive,
    /// App entered background execution state.
    DidEnterBackground,
    /// App is returning to the foreground.
    WillEnterForeground,
    /// App is terminating.
    WillTerminate,
}

/// Submit one iOS application lifecycle callback.
pub(crate) fn ios_notify_application_lifecycle(
    session_handle: HostSessionHandle,
    lifecycle: IosApplicationLifecycle,
) -> RuntimeResult<()> {
    let queue = ios_host_queue(session_handle)?;
    let state = host_lifecycle_state_for_application_lifecycle(lifecycle);

    queue.enqueue(HostEvent::Lifecycle(HostLifecycleEvent {
        source_kind: HostLifecycleSourceKind::Application,
        state,
    }));

    Ok(())
}

/// Map one iOS application lifecycle transition to host lifecycle state.
pub(crate) fn host_lifecycle_state_for_application_lifecycle(
    lifecycle: IosApplicationLifecycle,
) -> HostLifecycleState {
    match lifecycle {
        IosApplicationLifecycle::DidFinishLaunching => HostLifecycleState::Initializing,
        IosApplicationLifecycle::DidBecomeActive => HostLifecycleState::Running,
        IosApplicationLifecycle::WillResignActive => HostLifecycleState::Paused,
        IosApplicationLifecycle::DidEnterBackground => HostLifecycleState::Paused,
        IosApplicationLifecycle::WillEnterForeground => HostLifecycleState::Running,
        IosApplicationLifecycle::WillTerminate => HostLifecycleState::Destroyed,
    }
}
