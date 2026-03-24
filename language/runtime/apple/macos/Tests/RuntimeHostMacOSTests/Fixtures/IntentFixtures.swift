import RuntimeHostAppleCore

/// One no-op intent request handler for macOS tests.
@MainActor
final class MacOSNoopIntentRequestHandler: IntentRequests {
  func canOpenURL(_ url: String) -> Bool { false }

  func openURL(_ url: String) -> UInt32 { hostStatusNotSupported }

  func openPath(_ path: String) -> UInt32 { hostStatusNotSupported }

  func shareText(
    _ text: String,
    contentType: String?
  ) -> UInt32 {
    hostStatusNotSupported
  }

  func sharePaths(
    _ paths: [String],
    contentType: String?
  ) -> UInt32 {
    hostStatusNotSupported
  }
}

/// One recording intent request handler for macOS tests.
@MainActor
final class MacOSRecordingIntentRequestHandler: IntentRequests {
  var canOpenURLCalls: [String] = []
  var openURLCalls: [String] = []
  var openPathCalls: [String] = []

  func canOpenURL(_ url: String) -> Bool {
    canOpenURLCalls.append(url)

    return true
  }

  func openURL(_ url: String) -> UInt32 {
    openURLCalls.append(url)

    return hostStatusOk
  }

  func openPath(_ path: String) -> UInt32 {
    openPathCalls.append(path)

    return hostStatusOk
  }

  func shareText(
    _ text: String,
    contentType: String?
  ) -> UInt32 {
    hostStatusNotSupported
  }

  func sharePaths(
    _ paths: [String],
    contentType: String?
  ) -> UInt32 {
    hostStatusNotSupported
  }
}

/// One recording intent event sink for macOS tests.
@MainActor
final class MacOSRecordingIntentEventSink: IntentEvents {
  var events: [RuntimeHostIntentEvent] = []

  func sendIntentEvent(_ event: RuntimeHostIntentEvent) {
    events.append(event)
  }
}
