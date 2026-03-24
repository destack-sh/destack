import RuntimeHostAppleCore

/// One recording document request handler for Apple core tests.
@MainActor
final class RecordingDocumentRequestHandler: DocumentRequests {
  var requests: [RuntimeHostDocumentRequest] = []

  func submitDocumentRequest(_ request: RuntimeHostDocumentRequest) {
    requests.append(request)
  }
}

/// One recording document result sink for Apple core tests.
@MainActor
final class RecordingDocumentEventSink: DocumentEvents {
  var results: [RuntimeHostDocumentResult] = []

  func sendDocumentResult(_ result: RuntimeHostDocumentResult) {
    results.append(result)
  }
}
