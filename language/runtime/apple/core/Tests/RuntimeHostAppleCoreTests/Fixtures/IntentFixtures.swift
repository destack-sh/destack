import RuntimeHostAppleCore

/// One recording intent request handler for Apple core tests.
@MainActor
final class RecordingIntentRequestHandler: IntentRequests {
  var canOpenURLCalls: [String] = []
  var openURLCalls: [String] = []
  var openPathCalls: [String] = []
  var shareTextCalls: [(String, String?)] = []
  var sharePathCalls: [([String], String?)] = []
  var isOpenURLSupported: Bool = false
  var status: UInt32 = hostStatusOk

  func canOpenURL(_ url: String) -> Bool {
    canOpenURLCalls.append(url)

    return isOpenURLSupported
  }

  func openURL(_ url: String) -> UInt32 {
    openURLCalls.append(url)

    return status
  }

  func openPath(_ path: String) -> UInt32 {
    openPathCalls.append(path)

    return status
  }

  func shareText(
    _ text: String,
    mimeType: String?
  ) -> UInt32 {
    shareTextCalls.append((text, mimeType))

    return status
  }

  func sharePaths(
    _ paths: [String],
    mimeType: String?
  ) -> UInt32 {
    sharePathCalls.append((paths, mimeType))

    return status
  }
}

/// One recording intent event sink for Apple core tests.
@MainActor
final class RecordingIntentEventSink: IntentEvents {
  var events: [RuntimeHostIntentEvent] = []

  func notifyIntentEvent(_ event: RuntimeHostIntentEvent) {
    events.append(event)
  }
}
