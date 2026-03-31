import Foundation
import RuntimeHostAppleCore

#if canImport(AppKit)
  import AppKit
#endif

/// The application-delegate adapter for one macOS runtime host.
@MainActor
public final class ApplicationDelegate: NSObject {
  /// The macOS runtime host receiving application callbacks.
  public let runtimeHost: RuntimeHost

  /// Create one application-delegate adapter.
  public init(
    runtimeHost: RuntimeHost
  ) {
    self.runtimeHost = runtimeHost
    super.init()
  }

  /// Forward one application launch callback into the runtime host.
  public func applicationDidFinishLaunching() {
    sendLifecycleState(.application, .initializing)
  }

  /// Forward one application active callback into the runtime host.
  public func applicationDidBecomeActive() {
    sendLifecycleState(.application, .running)
  }

  /// Forward one application resign-active callback into the runtime host.
  public func applicationWillResignActive() {
    sendLifecycleState(.application, .paused)
  }

  /// Forward one application terminate callback into the runtime host.
  public func applicationWillTerminate() {
    sendLifecycleState(.application, .destroyed)
  }

  /// Send one application lifecycle state through the runtime host.
  private func sendLifecycleState(
    _ sourceKind: RuntimeHostLifecycleSourceKind,
    _ state: RuntimeHostLifecycleState
  ) {
    runtimeHost.lifecycle.sendLifecycleEvent(
      RuntimeHostLifecycleEvent(
        sourceKind: sourceKind,
        state: state
      )
    )
  }
}

#if canImport(AppKit)
  extension ApplicationDelegate: NSApplicationDelegate {
    public func applicationDidFinishLaunching(
      _ notification: Notification
    ) {
      applicationDidFinishLaunching()
    }

    public func applicationDidBecomeActive(
      _ notification: Notification
    ) {
      applicationDidBecomeActive()
    }

    public func applicationWillResignActive(
      _ notification: Notification
    ) {
      applicationWillResignActive()
    }

    public func applicationWillTerminate(
      _ notification: Notification
    ) {
      applicationWillTerminate()
    }
  }
#endif
