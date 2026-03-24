import RuntimeHostAppleCore

/// One no-op notification request handler for macOS tests.
@MainActor
final class MacOSNoopNotificationRequestHandler: NotificationRequests {
  func postNotification(
    _ request: RuntimeHostNotificationRequest
  ) -> UInt32 {
    hostStatusNotSupported
  }

  func cancelNotification(
    identifier: String
  ) -> UInt32 {
    hostStatusNotSupported
  }

  func cancelAllNotifications() -> UInt32 {
    hostStatusNotSupported
  }
}

/// One recording notification event sink for macOS tests.
@MainActor
final class MacOSRecordingNotificationEventSink: NotificationEvents {
  var events: [RuntimeHostNotificationEvent] = []

  func sendNotificationEvent(_ event: RuntimeHostNotificationEvent) {
    events.append(event)
  }
}
