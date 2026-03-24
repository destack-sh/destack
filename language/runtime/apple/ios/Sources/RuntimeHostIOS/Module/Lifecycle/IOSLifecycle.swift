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

/// Create one scene lifecycle event for the shared runtime host.
public func sceneLifecycleEvent(
  _ state: RuntimeHostLifecycleState
) -> RuntimeHostLifecycleEvent {
  RuntimeHostLifecycleEvent(
    sourceKind: .scene,
    state: state
  )
}
