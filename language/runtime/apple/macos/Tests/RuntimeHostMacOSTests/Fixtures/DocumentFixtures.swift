import RuntimeHostAppleCore

/// One no-op document request handler for macOS tests.
@MainActor
final class MacOSNoopDocumentRequestHandler: DocumentRequests {
  func pick(_ request: RuntimeHostDocumentRequest) -> UInt32 { hostStatusOk }
}

/// One recording document request handler for macOS tests.
@MainActor
final class MacOSRecordingDocumentRequestHandler: DocumentRequests {
  var requests: [RuntimeHostDocumentRequest] = []

  func pick(_ request: RuntimeHostDocumentRequest) -> UInt32 {
    requests.append(request)

    return hostStatusOk
  }
}

/// One recording document result sink for macOS tests.
@MainActor
final class MacOSRecordingDocumentEventSink: DocumentEvents {
  var results: [RuntimeHostDocumentResult] = []

  func notifyDocumentResult(
    _ result: RuntimeHostDocumentResult
  ) {
    results.append(result)
  }
}
