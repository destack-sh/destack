import Foundation

/// One document descriptor returned by the Apple host.
public struct RuntimeHostDocumentDescriptor: Sendable, Hashable, Codable {
  /// The stable URL or content identifier returned by the host.
  public let uri: String
  /// The normalized display name when available.
  public let displayName: String?
  /// The normalized content type when available.
  public let contentType: String?
  /// The optional host-local path when the host exposes one directly.
  public let localPath: String?

  /// Create one document descriptor.
  public init(
    uri: String,
    displayName: String? = nil,
    contentType: String? = nil,
    localPath: String? = nil
  ) {
    self.uri = uri
    self.displayName = displayName
    self.contentType = contentType
    self.localPath = localPath
  }
}

/// One Apple document request submitted by one runtime session.
public struct RuntimeHostDocumentRequest: Sendable, Hashable, Codable {
  /// The stable request identifier for this interactive host flow.
  public let requestID: HostRequestID
  /// Whether multiple documents may be selected.
  public let allowsMultipleSelection: Bool
  /// The accepted content types for the picker.
  public let contentTypes: [String]

  /// Create one document request.
  public init(
    requestID: HostRequestID,
    allowsMultipleSelection: Bool = false,
    contentTypes: [String] = []
  ) {
    self.requestID = requestID
    self.allowsMultipleSelection = allowsMultipleSelection
    self.contentTypes = contentTypes
  }
}

/// One Apple document result returned to one runtime session.
public struct RuntimeHostDocumentResult: Sendable, Hashable, Codable {
  /// The stable request identifier for this interactive host flow.
  public let requestID: HostRequestID
  /// The selected document descriptors.
  public let documents: [RuntimeHostDocumentDescriptor]

  /// Create one document result.
  public init(
    requestID: HostRequestID,
    documents: [RuntimeHostDocumentDescriptor]
  ) {
    self.requestID = requestID
    self.documents = documents
  }
}
