import Foundation
import RuntimeHostAppleCore

/// The surface embedder for one iOS runtime host.
@MainActor
public final class SurfaceEmbedder {
  /// The iOS runtime host attached to this surface.
  public let runtimeHost: RuntimeHost

  /// The scene delegate adapter installed for this embedder.
  public let sceneDelegate: SceneDelegate
  /// The location surface attached to this scene.
  public let location: any LocationRequests
  /// The text surface attached to this scene.
  public let text: any TextRequests

  /// Create one iOS surface embedder.
  public init(
    runtimeHost: RuntimeHost
  ) {
    let text = UIKitTextRequests(
      events: runtimeHost.text
    )

    self.runtimeHost = runtimeHost
    self.sceneDelegate = SceneDelegate(runtimeHost: runtimeHost)
    self.location = CoreLocationRequests(
      events: runtimeHost.location
    )
    self.text = text
    runtimeHost.updateTextRequests(text)
  }
}
