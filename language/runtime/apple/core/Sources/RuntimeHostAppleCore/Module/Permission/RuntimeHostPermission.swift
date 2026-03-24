import Foundation

/// One Apple permission request submitted by one runtime session.
public struct RuntimeHostPermissionRequest: Sendable, Hashable, Codable {
  /// The stable request identifier for this interactive host flow.
  public let requestID: HostRequestID
  /// The normalized permission name requested by the runtime.
  public let permission: String

  /// Create one permission request.
  public init(
    requestID: HostRequestID,
    permission: String
  ) {
    self.requestID = requestID
    self.permission = permission
  }
}

/// One Apple permission result event delivered into one runtime session.
public struct RuntimeHostPermissionEvent: Sendable, Hashable, Codable {
  /// The stable request identifier for this interactive host flow.
  public let requestID: HostRequestID
  /// The normalized permission name associated with this result.
  public let permission: String
  /// Whether the Apple host granted the permission.
  public let isGranted: Bool

  /// Create one permission event.
  public init(
    requestID: HostRequestID,
    permission: String,
    isGranted: Bool
  ) {
    self.requestID = requestID
    self.permission = permission
    self.isGranted = isGranted
  }
}
