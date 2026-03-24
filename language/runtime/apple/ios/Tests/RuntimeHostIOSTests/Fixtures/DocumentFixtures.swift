import RuntimeHostAppleCore

/// One no-op document request handler for iOS tests.
@MainActor
final class IOSNoopDocumentRequestHandler: DocumentRequests {
  func submitDocumentRequest(_ request: RuntimeHostDocumentRequest) {}
}

/// One recording document request handler for iOS tests.
@MainActor
final class IOSRecordingDocumentRequestHandler: DocumentRequests {
  var requests: [RuntimeHostDocumentRequest] = []

  func submitDocumentRequest(_ request: RuntimeHostDocumentRequest) {
    requests.append(request)
  }
}

/// One recording document result sink for iOS tests.
@MainActor
final class IOSRecordingDocumentEventSink: DocumentEvents {
  var results: [RuntimeHostDocumentResult] = []

  func sendDocumentResult(_ result: RuntimeHostDocumentResult) {
    results.append(result)
  }
}
