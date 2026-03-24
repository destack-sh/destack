import Foundation

/// The intent ingress surface attached to one Apple runtime host.
@MainActor
public protocol IntentEvents: AnyObject {
  /// Send one normalized intent event into the attached runtime session.
  func sendIntentEvent(_ event: RuntimeHostIntentEvent)
}
