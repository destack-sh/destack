import Foundation
import RuntimeHostAppleCore

#if canImport(UIKit)
  import UIKit
#endif

/// The scene-delegate adapter for one iOS runtime host.
@MainActor
public final class SceneDelegate: NSObject {
  /// The iOS runtime host receiving scene callbacks.
  public let runtimeHost: RuntimeHost

  /// Create one scene-delegate adapter.
  public init(
    runtimeHost: RuntimeHost
  ) {
    self.runtimeHost = runtimeHost
    super.init()
  }

  /// Forward one scene connect callback into the runtime host.
  public func sceneWillConnect() {
    sendLifecycleState(.scene, .initializing)
  }

  /// Forward one scene active callback into the runtime host.
  public func sceneDidBecomeActive() {
    sendLifecycleState(.scene, .running)
  }

  /// Forward one scene resign-active callback into the runtime host.
  public func sceneWillResignActive() {
    sendLifecycleState(.scene, .paused)
  }

  /// Forward one scene background callback into the runtime host.
  public func sceneDidEnterBackground() {
    sendLifecycleState(.scene, .paused)
  }

  /// Forward one scene foreground callback into the runtime host.
  public func sceneWillEnterForeground() {
    sendLifecycleState(.scene, .running)
  }

  /// Forward one scene disconnect callback into the runtime host.
  public func sceneDidDisconnect() {
    sendLifecycleState(.scene, .destroyed)
  }

  /// Send one scene lifecycle state through the runtime host.
  private func sendLifecycleState(
    _ sourceKind: RuntimeHostLifecycleSourceKind,
    _ state: RuntimeHostLifecycleState
  ) {
    runtimeHost.lifecycleEvents.sendLifecycleEvent(
      RuntimeHostLifecycleEvent(
        sourceKind: sourceKind,
        state: state
      )
    )
  }
}

#if canImport(UIKit)
  extension SceneDelegate: UIWindowSceneDelegate {
    public func scene(
      _ scene: UIScene,
      willConnectTo session: UISceneSession,
      options connectionOptions: UIScene.ConnectionOptions
    ) {
      sceneWillConnect()
    }

    public func sceneDidBecomeActive(
      _ scene: UIScene
    ) {
      sceneDidBecomeActive()
    }

    public func sceneWillResignActive(
      _ scene: UIScene
    ) {
      sceneWillResignActive()
    }

    public func sceneDidEnterBackground(
      _ scene: UIScene
    ) {
      sceneDidEnterBackground()
    }

    public func sceneWillEnterForeground(
      _ scene: UIScene
    ) {
      sceneWillEnterForeground()
    }

    public func sceneDidDisconnect(
      _ scene: UIScene
    ) {
      sceneDidDisconnect()
    }
  }
#endif
