import RuntimeHostAppleCore

/// One no-op media request handler for macOS tests.
@MainActor
final class MacOSNoopMediaRequestHandler: MediaRequests {
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
