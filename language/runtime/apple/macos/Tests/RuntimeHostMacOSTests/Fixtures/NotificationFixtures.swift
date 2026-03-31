import RuntimeHostAppleCore

/// One no-op notification request handler for macOS tests.
@MainActor
final class MacOSNoopNotificationRequestHandler: NotificationRequests {
  func post(
    _ request: RuntimeHostNotificationRequest
  ) -> UInt32 {
    hostStatusNotSupported
  }

  func cancel(
    _ identifier: String
  ) -> UInt32 {
    hostStatusNotSupported
  }

  func cancelAll() -> UInt32 {
    hostStatusNotSupported
  }
}

/// One recording notification event sink for macOS tests.
@MainActor
final class MacOSRecordingNotificationEventSink: NotificationEvents {
  var events: [RuntimeHostNotificationEvent] = []

  func notifyNotificationEvent(_ event: RuntimeHostNotificationEvent) {
    events.append(event)
  }
}
