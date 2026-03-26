import Foundation
import RuntimeHostAppleCore

/// The surface embedder for one iOS runtime host.
@MainActor
public final class SurfaceEmbedder {
  /// The iOS runtime host attached to this surface.
  public let runtimeHost: RuntimeHost

  /// The scene delegate adapter installed for this embedder.
  public let sceneDelegate: SceneDelegate
  /// The location request surface attached to this scene.
  public let locationRequests: any LocationRequests
  /// The text-input request surface attached to this scene.
  public let textInputRequests: any TextInputRequests

  /// Create one iOS surface embedder.
  public init(
    runtimeHost: RuntimeHost
  ) {
    let textInputRequests = UIKitTextInputRequests(
      events: runtimeHost.textInputEvents
    )

    self.runtimeHost = runtimeHost
    self.sceneDelegate = SceneDelegate(runtimeHost: runtimeHost)
    self.locationRequests = CoreLocationRequests(
      events: runtimeHost.locationEvents
    )
    self.textInputRequests = textInputRequests
    runtimeHost.textInputRequests = textInputRequests
  }
}
