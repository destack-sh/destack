import Foundation

/// The normalized lifecycle state delivered from Apple hosts into one runtime session.
public enum RuntimeHostLifecycleState: String, Sendable, Codable {
  /// The runtime host has not finished start-up yet.
  case initializing
  /// The runtime host is active and may process host interaction.
  case running
  /// The runtime host is paused by the Apple shell.
  case paused
  /// The runtime host is stopped by the Apple shell.
  case stopped
  /// The runtime host is being destroyed by the Apple shell.
  case destroyed
}

/// The lifecycle source embedder that originated one lifecycle event.
public enum RuntimeHostLifecycleSourceKind: String, Sendable, Codable {
  /// The event came from one application embedder.
  case application
  /// The event came from one scene embedder.
  case scene
  /// The event came from one window embedder.
  case window
}

/// One Apple lifecycle ingress event delivered into one runtime session.
public struct RuntimeHostLifecycleEvent: Sendable, Hashable, Codable {
  /// The lifecycle source embedder that originated this event.
  public let sourceKind: RuntimeHostLifecycleSourceKind
  /// The next normalized lifecycle state for the runtime session.
  public let state: RuntimeHostLifecycleState

  /// Create one lifecycle event.
  public init(
    sourceKind: RuntimeHostLifecycleSourceKind,
    state: RuntimeHostLifecycleState
  ) {
    self.sourceKind = sourceKind
    self.state = state
  }
}
