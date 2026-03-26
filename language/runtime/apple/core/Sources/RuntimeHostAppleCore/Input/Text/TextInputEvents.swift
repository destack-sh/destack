import Foundation

/// The text-input ingress surface attached to one Apple runtime host.
@MainActor
public protocol TextInputEvents: AnyObject {
  /// Send one host text-input event into the attached runtime session.
  func sendTextInputEvent(_ event: RuntimeHostTextInputEvent)
}

/// The explicit no-op text-input ingress surface for one Apple runtime host.
@MainActor
public final class NoopTextInputEvents: TextInputEvents {
  /// Create one no-op text-input ingress surface.
  public init() {}

  public func sendTextInputEvent(_ event: RuntimeHostTextInputEvent) {
    let _ = event
  }
}
