import Foundation

/// The notification request surface attached to one Apple runtime host.
@MainActor
public protocol NotificationRequests: AnyObject {
  /// Post one notification through the Apple host.
  func postNotification(
    _ request: RuntimeHostNotificationRequest
  ) -> UInt32

  /// Cancel one posted notification through the Apple host.
  func cancelNotification(
    identifier: String
  ) -> UInt32

  /// Cancel every posted notification through the Apple host.
  func cancelAllNotifications() -> UInt32
}
