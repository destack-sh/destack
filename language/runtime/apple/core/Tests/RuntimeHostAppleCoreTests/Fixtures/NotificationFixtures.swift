import RuntimeHostAppleCore

/// One recording notification request handler for Apple core tests.
@MainActor
final class RecordingNotificationRequestHandler: NotificationRequests {
  var postedRequests: [RuntimeHostNotificationRequest] = []
  var cancelledIdentifiers: [String] = []
  var cancelAllCalls: Int = 0
  var status: UInt32 = hostStatusOk

  func postNotification(
    _ request: RuntimeHostNotificationRequest
  ) -> UInt32 {
    postedRequests.append(request)

    return status
  }

  func cancelNotification(
    identifier: String
  ) -> UInt32 {
    cancelledIdentifiers.append(identifier)

    return status
  }

  func cancelAllNotifications() -> UInt32 {
    cancelAllCalls += 1

    return status
  }
}

/// One recording notification event sink for Apple core tests.
@MainActor
final class RecordingNotificationEventSink: NotificationEvents {
  var events: [RuntimeHostNotificationEvent] = []

  func sendNotificationEvent(_ event: RuntimeHostNotificationEvent) {
    events.append(event)
  }
}
