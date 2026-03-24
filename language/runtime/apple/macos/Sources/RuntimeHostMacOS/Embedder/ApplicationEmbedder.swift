import Foundation
import RuntimeHostAppleCore

/// The application embedder for one macOS runtime host.
@MainActor
public final class ApplicationEmbedder {
  /// The macOS runtime host attached to this application.
  public let runtimeHost: RuntimeHost

  /// The application delegate adapter installed for this embedder.
  public let applicationDelegate: ApplicationDelegate

  /// Create one macOS application embedder.
  public init(
    runtimeHost: RuntimeHost
  ) {
    self.runtimeHost = runtimeHost
    self.applicationDelegate = ApplicationDelegate(runtimeHost: runtimeHost)
  }
}
