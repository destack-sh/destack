import Foundation

/// The background ingress surface attached to one Apple runtime host.
@MainActor
public protocol BackgroundEvents: AnyObject {
  /// Send one host background event into the attached runtime session.
  func sendBackgroundEvent(_ event: RuntimeHostBackgroundEvent)
}

/// The explicit no-op background ingress surface for one Apple runtime host.
@MainActor
public final class NoopBackgroundEvents: BackgroundEvents {
  /// Create one no-op background ingress surface.
  public init() {}

  /// Send one host background event into the attached runtime session.
  public func sendBackgroundEvent(_ event: RuntimeHostBackgroundEvent) {
    let _ = event
  }
}
