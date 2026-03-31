import RuntimeHostAppleCore

/// One no-op document request handler for iOS tests.
@MainActor
final class IOSNoopDocumentRequestHandler: DocumentRequests {
  func pick(_ request: RuntimeHostDocumentRequest) -> UInt32 { hostStatusOk }
}

/// One recording document request handler for iOS tests.
@MainActor
final class IOSRecordingDocumentRequestHandler: DocumentRequests {
  var requests: [RuntimeHostDocumentRequest] = []

  func pick(_ request: RuntimeHostDocumentRequest) -> UInt32 {
    requests.append(request)

    return hostStatusOk
  }
}

/// One recording document result sink for iOS tests.
@MainActor
final class IOSRecordingDocumentEventSink: DocumentEvents {
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
