import Foundation

/// The notification ingress surface attached to one Apple runtime host.
@MainActor
public protocol NotificationEvents: AnyObject {
  /// Send one normalized notification event into the attached runtime session.
  func sendNotificationEvent(_ event: RuntimeHostNotificationEvent)
}
