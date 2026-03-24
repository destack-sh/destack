import RuntimeHostAppleCore

/// One no-op media request handler for macOS tests.
@MainActor
final class MacOSNoopMediaRequestHandler: MediaRequests {
    func listMedia(
        _ request: RuntimeHostMediaListRequest
    ) -> RuntimeHostMediaListResponse {
        RuntimeHostMediaListResponse(status: hostStatusNotSupported)
    }

    func readMedia(
        identifier: String
    ) -> RuntimeHostMediaReadResponse {
        RuntimeHostMediaReadResponse(status: hostStatusNotSupported)
    }

    func importMediaPath(
        _ request: RuntimeHostMediaImportPathRequest
    ) -> RuntimeHostMediaImportPathResponse {
        RuntimeHostMediaImportPathResponse(status: hostStatusNotSupported)
    }

    func deleteMedia(
        _ request: RuntimeHostMediaDeleteRequest
    ) -> RuntimeHostMediaDeleteResponse {
        RuntimeHostMediaDeleteResponse(status: hostStatusNotSupported)
    }
}
