import Foundation

/// The stable host embedder identifier for one attached runtime session.
public struct HostEmbedderID: Sendable, Hashable, Codable {
    /// The raw embedder identifier value.
    public let rawValue: UInt64

    /// Create one embedder identifier from one raw value.
    public init(rawValue: UInt64) {
        self.rawValue = rawValue
    }
}
