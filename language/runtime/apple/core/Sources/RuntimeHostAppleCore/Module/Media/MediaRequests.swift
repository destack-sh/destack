import Foundation

/// The Apple media request surface for one runtime host embedder.
@MainActor
public protocol MediaRequests: AnyObject {
  /// List one page of media assets for one query.
  func listMedia(
    _ request: RuntimeHostMediaListRequest
  ) -> RuntimeHostMediaListResponse

  /// Read one media asset descriptor by stable identifier.
  func readMedia(
    identifier: String
  ) -> RuntimeHostMediaReadResponse

  /// Import one local path into the media library.
  func importMediaPath(
    _ request: RuntimeHostMediaImportPathRequest
  ) -> RuntimeHostMediaImportPathResponse

  /// Delete one batch of media assets and return the deleted count.
  func deleteMedia(
    _ request: RuntimeHostMediaDeleteRequest
  ) -> RuntimeHostMediaDeleteResponse
}
