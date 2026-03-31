import Foundation

/// One document descriptor returned by the Apple host.
public struct RuntimeHostDocumentDescriptor: Sendable, Hashable, Codable {
  /// The stable URL or content identifier returned by the host.
  public let uri: String
  /// The normalized display name when available.
  public let displayName: String?
  /// The normalized content type when available.
  public let contentType: String?
  /// The optional document size in bytes.
  public let sizeBytes: UInt64?
  /// The optional modification timestamp in UTC nanoseconds.
  public let modifiedUnixNs: UInt64?
  /// Whether this descriptor identifies one directory.
  public let isDirectory: Bool
  /// The optional host-local path when the host exposes one directly.
  public let localPath: String?

  /// Create one document descriptor.
  public init(
    uri: String,
    displayName: String? = nil,
    contentType: String? = nil,
    sizeBytes: UInt64? = nil,
    modifiedUnixNs: UInt64? = nil,
    isDirectory: Bool = false,
    localPath: String? = nil
  ) {
    self.uri = uri
    self.displayName = displayName
    self.contentType = contentType
    self.sizeBytes = sizeBytes
    self.modifiedUnixNs = modifiedUnixNs
    self.isDirectory = isDirectory
    self.localPath = localPath
  }

  /// The bridge-facing document name.
  public var name: String {
    displayName ?? ""
  }

  /// The bridge-facing MIME type.
  public var mimeType: String? {
    contentType
  }

  /// Create one document descriptor from the generated bridge field names.
  public init(
    uri: String,
    name: String,
    mimeType: String? = nil,
    sizeBytes: UInt64? = nil,
    modifiedUnixNs: UInt64? = nil,
    isDirectory: Bool = false,
    localPath: String? = nil
  ) {
    self.init(
      uri: uri,
      displayName: name.isEmpty ? nil : name,
      contentType: mimeType,
      sizeBytes: sizeBytes,
      modifiedUnixNs: modifiedUnixNs,
      isDirectory: isDirectory,
      localPath: localPath
    )
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
  /// The accepted MIME types for the picker.
  public let mimeTypes: [String]
  /// The accepted file extensions for the picker.
  public let extensions: [String]
  /// Whether directory selection is allowed.
  public let allowsDirectorySelection: Bool
  /// Whether the host should copy results into one sandbox path.
  public let copiesToSandbox: Bool

  /// Create one document request.
  public init(
    requestID: HostRequestID,
    allowsMultipleSelection: Bool = false,
    contentTypes: [String] = [],
    mimeTypes: [String] = [],
    extensions: [String] = [],
    allowsDirectorySelection: Bool = false,
    copiesToSandbox: Bool = false
  ) {
    self.requestID = requestID
    self.allowsMultipleSelection = allowsMultipleSelection
    self.contentTypes = contentTypes
    self.mimeTypes = mimeTypes.isEmpty ? contentTypes : mimeTypes
    self.extensions = extensions
    self.allowsDirectorySelection = allowsDirectorySelection
    self.copiesToSandbox = copiesToSandbox
  }

  /// Create one document request from the generated bridge field order.
  public init(
    requestID: HostRequestID,
    mimeTypes: [String],
    extensions: [String],
    allowsMultipleSelection: Bool = false,
    allowsDirectorySelection: Bool = false,
    copiesToSandbox: Bool = false
  ) {
    self.init(
      requestID: requestID,
      allowsMultipleSelection: allowsMultipleSelection,
      contentTypes: mimeTypes,
      mimeTypes: mimeTypes,
      extensions: extensions,
      allowsDirectorySelection: allowsDirectorySelection,
      copiesToSandbox: copiesToSandbox
    )
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
