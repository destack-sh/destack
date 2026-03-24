import RuntimeHostAppleCore

/// One recording lifecycle sink for macOS tests.
@MainActor
final class MacOSRecordingLifecycleSink: LifecycleEvents {
  var events: [RuntimeHostLifecycleEvent] = []

  func sendLifecycleEvent(_ event: RuntimeHostLifecycleEvent) {
    events.append(event)
  }
}

/// Build one application lifecycle event fixture for macOS tests.
@MainActor
func macOSApplicationLifecycleEvent(
  _ state: RuntimeHostLifecycleState
) -> RuntimeHostLifecycleEvent {
  RuntimeHostLifecycleEvent(
    sourceKind: .application,
    state: state
  )
}

/// Build one window lifecycle event fixture for macOS tests.
@MainActor
func windowLifecycleEvent(
  _ state: RuntimeHostLifecycleState
) -> RuntimeHostLifecycleEvent {
  RuntimeHostLifecycleEvent(
    sourceKind: .window,
    state: state
  )
}
