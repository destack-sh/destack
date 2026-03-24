import Foundation

/// The Apple media asset kind returned by one host surface.
public enum RuntimeHostMediaAssetKind: Sendable, Hashable, Codable {
  /// One image asset.
  case image
  /// One video asset.
  case video
  /// One audio asset.
  case audio
  /// One non-standard asset.
  case other
}

/// One media-asset descriptor returned by the Apple host.
public struct RuntimeHostMediaAssetDescriptor: Sendable, Hashable, Codable {
  /// The stable host media identifier.
  public let identifier: String
  /// The host URI for this asset.
  public let uri: String
  /// The asset filename payload.
  public let filename: String
  /// The asset MIME type payload when available.
  public let mimeType: String
  /// The asset class.
  public let kind: RuntimeHostMediaAssetKind
  /// The asset width in pixels when available.
  public let width: UInt32
  /// The asset height in pixels when available.
  public let height: UInt32
  /// The asset duration in milliseconds for time-based assets.
  public let durationMs: UInt64
  /// The asset size in bytes when available.
  public let sizeBytes: UInt64
  /// The asset creation timestamp in UTC nanoseconds when available.
  public let createdUnixNs: UInt64
  /// The asset modification timestamp in UTC nanoseconds when available.
  public let modifiedUnixNs: UInt64

  /// Create one media-asset descriptor.
  public init(
    identifier: String,
    uri: String,
    filename: String,
    mimeType: String,
    kind: RuntimeHostMediaAssetKind,
    width: UInt32 = 0,
    height: UInt32 = 0,
    durationMs: UInt64 = 0,
    sizeBytes: UInt64 = 0,
    createdUnixNs: UInt64 = 0,
    modifiedUnixNs: UInt64 = 0
  ) {
    self.identifier = identifier
    self.uri = uri
    self.filename = filename
    self.mimeType = mimeType
    self.kind = kind
    self.width = width
    self.height = height
    self.durationMs = durationMs
    self.sizeBytes = sizeBytes
    self.createdUnixNs = createdUnixNs
    self.modifiedUnixNs = modifiedUnixNs
  }
}

/// One Apple media-list request submitted by one runtime session.
public struct RuntimeHostMediaListRequest: Sendable, Hashable, Codable {
  /// The opaque next-page cursor from one prior media-list call.
  public let cursor: String?
  /// The maximum number of returned assets when available.
  public let limit: UInt32?
  /// The requested asset kinds, empty means all kinds.
  public let kinds: [RuntimeHostMediaAssetKind]
  /// Whether hidden assets should be included.
  public let includeHidden: Bool

  /// Create one media-list request.
  public init(
    cursor: String? = nil,
    limit: UInt32? = nil,
    kinds: [RuntimeHostMediaAssetKind] = [],
    includeHidden: Bool = false
  ) {
    self.cursor = cursor
    self.limit = limit
    self.kinds = kinds
    self.includeHidden = includeHidden
  }
}

/// One Apple media page returned to one runtime session.
public struct RuntimeHostMediaListResult: Sendable, Hashable, Codable {
  /// The listed media assets.
  public let assets: [RuntimeHostMediaAssetDescriptor]
  /// The opaque next-page cursor when available.
  public let nextCursor: String
  /// Whether more assets are available.
  public let hasMore: Bool

  /// Create one media-list result.
  public init(
    assets: [RuntimeHostMediaAssetDescriptor],
    nextCursor: String = "",
    hasMore: Bool = false
  ) {
    self.assets = assets
    self.nextCursor = nextCursor
    self.hasMore = hasMore
  }
}

/// One Apple media-list response returned by the host.
public struct RuntimeHostMediaListResponse: Sendable, Hashable, Codable {
  /// The host status code.
  public let status: UInt32
  /// The returned media page when available.
  public let page: RuntimeHostMediaListResult?

  /// Create one media-list response.
  public init(
    status: UInt32,
    page: RuntimeHostMediaListResult? = nil
  ) {
    self.status = status
    self.page = page
  }
}

/// One Apple media-read response returned by the host.
public struct RuntimeHostMediaReadResponse: Sendable, Hashable, Codable {
  /// The host status code.
  public let status: UInt32
  /// The returned asset descriptor when available.
  public let descriptor: RuntimeHostMediaAssetDescriptor?

  /// Create one media-read response.
  public init(
    status: UInt32,
    descriptor: RuntimeHostMediaAssetDescriptor? = nil
  ) {
    self.status = status
    self.descriptor = descriptor
  }
}

/// One Apple media import request submitted by one runtime session.
public struct RuntimeHostMediaImportPathRequest: Sendable, Hashable, Codable {
  /// The local path to import into the host media library.
  public let path: String
  /// The requested asset kind.
  public let kind: RuntimeHostMediaAssetKind

  /// Create one media import request.
  public init(
    path: String,
    kind: RuntimeHostMediaAssetKind
  ) {
    self.path = path
    self.kind = kind
  }
}

/// One Apple media import response returned by the host.
public struct RuntimeHostMediaImportPathResponse: Sendable, Hashable, Codable {
  /// The host status code.
  public let status: UInt32
  /// The imported asset identifier when available.
  public let identifier: String?

  /// Create one media import response.
  public init(
    status: UInt32,
    identifier: String? = nil
  ) {
    self.status = status
    self.identifier = identifier
  }
}

/// One Apple media delete request submitted by one runtime session.
public struct RuntimeHostMediaDeleteRequest: Sendable, Hashable, Codable {
  /// The stable host media identifiers to delete.
  public let identifiers: [String]

  /// Create one media delete request.
  public init(identifiers: [String]) {
    self.identifiers = identifiers
  }
}

/// One Apple media delete response returned by the host.
public struct RuntimeHostMediaDeleteResponse: Sendable, Hashable, Codable {
  /// The host status code.
  public let status: UInt32
  /// The number of deleted assets.
  public let deletedCount: UInt32

  /// Create one media delete response.
  public init(
    status: UInt32,
    deletedCount: UInt32 = 0
  ) {
    self.status = status
    self.deletedCount = deletedCount
  }
}
