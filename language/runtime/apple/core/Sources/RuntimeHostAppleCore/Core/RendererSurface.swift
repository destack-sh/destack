import Foundation

/// The primary renderer surface family for one host embedder.
public enum RendererSurfaceKind: String, Sendable, Codable {
  /// One Core Animation backed surface.
  case metalLayer
  /// One Metal view backed surface.
  case metalView
  /// One platform view backed surface.
  case platformView
}

/// The primary renderer surface for one host embedder.
public struct RendererSurface: Sendable, Hashable, Codable {
  /// The primary renderer surface kind.
  public let kind: RendererSurfaceKind
  /// The stable logical identifier for this host surface.
  public let identifier: String

  /// Create one renderer surface.
  public init(
    kind: RendererSurfaceKind,
    identifier: String
  ) {
    self.kind = kind
    self.identifier = identifier
  }
}
