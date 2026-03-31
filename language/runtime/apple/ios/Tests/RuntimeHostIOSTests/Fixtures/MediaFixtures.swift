import RuntimeHostAppleCore

/// One no-op media request handler for iOS tests.
@MainActor
final class IOSNoopMediaRequestHandler: MediaRequests {
  func list(
    _ request: RuntimeHostMediaListRequest
  ) -> RuntimeHostMediaListResponse {
    RuntimeHostMediaListResponse(status: hostStatusNotSupported)
  }

  func read(
    _ identifier: String
  ) -> RuntimeHostMediaReadResponse {
    RuntimeHostMediaReadResponse(status: hostStatusNotSupported)
  }

  func importPath(
    _ request: RuntimeHostMediaImportPathRequest
  ) -> RuntimeHostMediaImportPathResponse {
    RuntimeHostMediaImportPathResponse(status: hostStatusNotSupported)
  }

  func delete(
    _ request: RuntimeHostMediaDeleteRequest
  ) -> RuntimeHostMediaDeleteResponse {
    RuntimeHostMediaDeleteResponse(status: hostStatusNotSupported)
  }
}

/// One recording media request handler for iOS tests.
@MainActor
final class IOSRecordingMediaRequestHandler: MediaRequests {
  var listRequests: [RuntimeHostMediaListRequest] = []
  var readIdentifiers: [String] = []
  var importRequests: [RuntimeHostMediaImportPathRequest] = []
  var deleteRequests: [RuntimeHostMediaDeleteRequest] = []
  var listResponse = RuntimeHostMediaListResponse(status: hostStatusNotSupported)
  var readResponse = RuntimeHostMediaReadResponse(status: hostStatusNotSupported)
  var importResponse = RuntimeHostMediaImportPathResponse(status: hostStatusNotSupported)
  var deleteResponse = RuntimeHostMediaDeleteResponse(status: hostStatusNotSupported)

  func list(
    _ request: RuntimeHostMediaListRequest
  ) -> RuntimeHostMediaListResponse {
    listRequests.append(request)

    return listResponse
  }

  func read(
    _ identifier: String
  ) -> RuntimeHostMediaReadResponse {
    readIdentifiers.append(identifier)

    return readResponse
  }

  func importPath(
    _ request: RuntimeHostMediaImportPathRequest
  ) -> RuntimeHostMediaImportPathResponse {
    importRequests.append(request)

    return importResponse
  }

  func delete(
    _ request: RuntimeHostMediaDeleteRequest
  ) -> RuntimeHostMediaDeleteResponse {
    deleteRequests.append(request)

    return deleteResponse
  }
}
