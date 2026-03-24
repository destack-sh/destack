import Foundation

/// One stable interactive host request identifier.
public struct HostRequestID: Sendable, Hashable, Codable {
    /// The raw request identifier value.
    public let rawValue: UInt64

    /// Create one request identifier.
    public init(rawValue: UInt64) {
        self.rawValue = rawValue
    }
}
