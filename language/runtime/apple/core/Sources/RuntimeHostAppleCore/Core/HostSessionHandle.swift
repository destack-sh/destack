import Foundation

/// The raw host ABI session handle for one attached runtime session.
public struct HostSessionHandle: Sendable, Hashable, Codable {
  /// The raw session handle value.
  public let rawValue: UInt64

  /// Create one session handle from one raw ABI value.
  public init(rawValue: UInt64) {
    self.rawValue = rawValue
  }
}
