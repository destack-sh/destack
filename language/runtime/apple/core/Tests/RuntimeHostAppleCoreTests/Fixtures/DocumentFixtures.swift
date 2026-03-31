import RuntimeHostAppleCore

/// One recording document request handler for Apple core tests.
@MainActor
final class RecordingDocumentRequestHandler: DocumentRequests {
  var requests: [RuntimeHostDocumentRequest] = []

  func pick(_ request: RuntimeHostDocumentRequest) -> UInt32 {
    requests.append(request)

    return hostStatusOk
  }
}

/// One recording document result sink for Apple core tests.
@MainActor
final class RecordingDocumentEventSink: DocumentEvents {
  var results: [RuntimeHostDocumentResult] = []

  func notifyDocumentResult(
    _ requestID: HostRequestID,
    documents: [RuntimeHostDocumentDescriptor]
  ) {
    results.append(
      RuntimeHostDocumentResult(
        requestID: requestID,
        documents: documents
      )
    )
  }
}
