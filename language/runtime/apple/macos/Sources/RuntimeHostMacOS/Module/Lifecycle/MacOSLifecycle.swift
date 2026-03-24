import Foundation
import RuntimeHostAppleCore

/// Create one application lifecycle event for the shared runtime host.
public func applicationLifecycleEvent(
  _ state: RuntimeHostLifecycleState
) -> RuntimeHostLifecycleEvent {
  RuntimeHostLifecycleEvent(
    sourceKind: .application,
    state: state
  )
}

/// Create one window lifecycle event for the shared runtime host.
public func windowLifecycleEvent(
  _ state: RuntimeHostLifecycleState
) -> RuntimeHostLifecycleEvent {
  RuntimeHostLifecycleEvent(
    sourceKind: .window,
    state: state
  )
}
