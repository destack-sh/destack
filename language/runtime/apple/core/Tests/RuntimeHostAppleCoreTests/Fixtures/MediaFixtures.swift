import RuntimeHostAppleCore

/// One recording media request handler for Apple core tests.
@MainActor
final class RecordingMediaRequestHandler: MediaRequests {
    var listRequests: [RuntimeHostMediaListRequest] = []
    var readIdentifiers: [String] = []
    var importRequests: [RuntimeHostMediaImportPathRequest] = []
    var deleteRequests: [RuntimeHostMediaDeleteRequest] = []
    var listResponse = RuntimeHostMediaListResponse(status: hostStatusNotSupported)
    var readResponse = RuntimeHostMediaReadResponse(status: hostStatusNotSupported)
    var importResponse = RuntimeHostMediaImportPathResponse(status: hostStatusNotSupported)
    var deleteResponse = RuntimeHostMediaDeleteResponse(status: hostStatusNotSupported)

    func listMedia(
        _ request: RuntimeHostMediaListRequest
    ) -> RuntimeHostMediaListResponse {
        listRequests.append(request)

        return listResponse
    }

    func readMedia(
        identifier: String
    ) -> RuntimeHostMediaReadResponse {
        readIdentifiers.append(identifier)

        return readResponse
    }

    func importMediaPath(
        _ request: RuntimeHostMediaImportPathRequest
    ) -> RuntimeHostMediaImportPathResponse {
        importRequests.append(request)

        return importResponse
    }

    func deleteMedia(
        _ request: RuntimeHostMediaDeleteRequest
    ) -> RuntimeHostMediaDeleteResponse {
        deleteRequests.append(request)

        return deleteResponse
    }
}
