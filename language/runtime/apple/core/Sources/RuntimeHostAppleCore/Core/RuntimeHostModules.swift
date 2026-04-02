import Foundation

/// The lifecycle host surface attached to one Apple runtime host.
@MainActor
public final class LifecycleHost: LifecycleEvents {
  private let events: any LifecycleEvents

  /// Create one lifecycle host surface.
  public init(
    events: any LifecycleEvents
  ) {
    self.events = events
  }

  /// Send one lifecycle event into the attached runtime session.
  public func sendLifecycleEvent(_ event: RuntimeHostLifecycleEvent) {
    events.sendLifecycleEvent(event)
  }
}
