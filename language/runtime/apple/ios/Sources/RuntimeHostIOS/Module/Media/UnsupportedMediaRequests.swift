import Foundation
import RuntimeHostAppleCore

/// The iOS media request surface backed by the application host.
@MainActor
public final class UnsupportedMediaRequests: MediaRequests {

  public func listMedia(
    _ request: RuntimeHostMediaListRequest
  ) -> RuntimeHostMediaListResponse {
    let _ = request

    return RuntimeHostMediaListResponse(status: hostStatusNotSupported)
  }

  public func readMedia(
    identifier: String
  ) -> RuntimeHostMediaReadResponse {
    let _ = identifier

    return RuntimeHostMediaReadResponse(status: hostStatusNotSupported)
  }

  public func importMediaPath(
    _ request: RuntimeHostMediaImportPathRequest
  ) -> RuntimeHostMediaImportPathResponse {
    let _ = request

    return RuntimeHostMediaImportPathResponse(status: hostStatusNotSupported)
  }

  public func deleteMedia(
    _ request: RuntimeHostMediaDeleteRequest
  ) -> RuntimeHostMediaDeleteResponse {
    let _ = request

    return RuntimeHostMediaDeleteResponse(status: hostStatusNotSupported)
  }
}
