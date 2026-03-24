import RuntimeHostAppleCore

/// One no-op document request handler for macOS tests.
@MainActor
final class MacOSNoopDocumentRequestHandler: DocumentRequests {
    func submitDocumentRequest(_ request: RuntimeHostDocumentRequest) {}
}

/// One recording document request handler for macOS tests.
@MainActor
final class MacOSRecordingDocumentRequestHandler: DocumentRequests {
    var requests: [RuntimeHostDocumentRequest] = []

    func submitDocumentRequest(_ request: RuntimeHostDocumentRequest) {
        requests.append(request)
    }
}

/// One recording document result sink for macOS tests.
@MainActor
final class MacOSRecordingDocumentEventSink: DocumentEvents {
    var results: [RuntimeHostDocumentResult] = []

    func sendDocumentResult(_ result: RuntimeHostDocumentResult) {
        results.append(result)
    }
}
