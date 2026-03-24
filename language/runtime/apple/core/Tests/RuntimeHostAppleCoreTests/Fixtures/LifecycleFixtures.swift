import RuntimeHostAppleCore

/// One recording lifecycle sink for Apple core tests.
@MainActor
final class RecordingLifecycleSink: LifecycleEvents {
  var events: [RuntimeHostLifecycleEvent] = []

  func sendLifecycleEvent(_ event: RuntimeHostLifecycleEvent) {
    events.append(event)
  }
}

/// Build one lifecycle event fixture.
func lifecycleEvent(
  _ sourceKind: RuntimeHostLifecycleSourceKind,
  _ state: RuntimeHostLifecycleState
) -> RuntimeHostLifecycleEvent {
  RuntimeHostLifecycleEvent(
    sourceKind: sourceKind,
    state: state
  )
}
