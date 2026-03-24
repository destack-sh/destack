import Foundation
import RuntimeHostAppleCore

/// The surface embedder for one macOS runtime host.
@MainActor
public final class SurfaceEmbedder {
  /// The macOS runtime host attached to this surface.
  public let runtimeHost: RuntimeHost

  /// The window delegate adapter installed for this embedder.
  public let windowDelegate: WindowDelegate
  /// Create one macOS surface embedder.
  public init(
    runtimeHost: RuntimeHost
  ) {
    self.runtimeHost = runtimeHost
    self.windowDelegate = WindowDelegate(runtimeHost: runtimeHost)
  }
}
