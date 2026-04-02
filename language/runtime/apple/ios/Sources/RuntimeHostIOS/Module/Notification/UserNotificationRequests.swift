import Foundation
import RuntimeHostAppleCore

#if canImport(UserNotifications)
  import UserNotifications
#endif

/// The iOS notification request surface backed by UserNotifications.
@MainActor
public final class UserNotificationRequests: NotificationRequests {

  public func post(
    _ request: RuntimeHostNotificationRequest
  ) -> UInt32 {
    #if canImport(UserNotifications)
      guard !request.tag.isEmpty else {
        return hostStatusInvalidArgument
      }

      if request.subtitle != nil
        || request.channelId != nil
        || request.badgeCount != nil
        || request.sound != nil
        || request.categoryId != nil
        || request.threadId != nil
        || request.actionId != nil
        || request.priority != .normal
        || request.trigger.kind != .immediate
      {
        return hostStatusNotSupported
      }

      let content = UNMutableNotificationContent()
      content.title = request.title
      content.body = request.body

      let trigger = UNTimeIntervalNotificationTrigger(
        timeInterval: 0.1,
        repeats: false
      )
      let notificationRequest = UNNotificationRequest(
        identifier: request.tag,
        content: content,
        trigger: trigger
      )

      UNUserNotificationCenter.current().add(notificationRequest)

      return hostStatusOk
    #else
      let _ = request

      return hostStatusNotSupported
    #endif
  }

  public func cancel(
    _ identifier: String
  ) -> UInt32 {
    #if canImport(UserNotifications)
      guard !identifier.isEmpty else {
        return hostStatusInvalidArgument
      }

      let center = UNUserNotificationCenter.current()
      center.removeDeliveredNotifications(withIdentifiers: [identifier])
      center.removePendingNotificationRequests(withIdentifiers: [identifier])

      return hostStatusOk
    #else
      let _ = identifier

      return hostStatusNotSupported
    #endif
  }

  public func cancelAll() -> UInt32 {
    #if canImport(UserNotifications)
      let center = UNUserNotificationCenter.current()
      center.removeAllDeliveredNotifications()
      center.removeAllPendingNotificationRequests()

      return hostStatusOk
    #else
      return hostStatusNotSupported
    #endif
  }
}
