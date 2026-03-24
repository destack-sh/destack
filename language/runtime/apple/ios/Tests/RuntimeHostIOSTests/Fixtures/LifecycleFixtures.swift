import RuntimeHostAppleCore

/// One recording lifecycle sink for iOS tests.
@MainActor
final class IOSRecordingLifecycleSink: LifecycleEvents {
    var events: [RuntimeHostLifecycleEvent] = []

    func sendLifecycleEvent(_ event: RuntimeHostLifecycleEvent) {
        events.append(event)
    }
}

/// Build one application lifecycle event fixture for iOS tests.
@MainActor
func applicationLifecycleEvent(
    _ state: RuntimeHostLifecycleState
) -> RuntimeHostLifecycleEvent {
    RuntimeHostLifecycleEvent(
        sourceKind: .application,
        state: state
    )
}

/// Build one scene lifecycle event fixture for iOS tests.
@MainActor
func sceneLifecycleEvent(
    _ state: RuntimeHostLifecycleState
) -> RuntimeHostLifecycleEvent {
    RuntimeHostLifecycleEvent(
        sourceKind: .scene,
        state: state
    )
}
