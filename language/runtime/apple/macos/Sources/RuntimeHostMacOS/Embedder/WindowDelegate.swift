import Foundation
import RuntimeHostAppleCore

#if canImport(AppKit)
  import AppKit
#endif

/// The window-delegate adapter for one macOS runtime host.
@MainActor
public final class WindowDelegate: NSObject {
  /// The macOS runtime host receiving window callbacks.
  public let runtimeHost: RuntimeHost

  /// Create one window-delegate adapter.
  public init(
    runtimeHost: RuntimeHost
  ) {
    self.runtimeHost = runtimeHost
    super.init()
  }

  /// Forward one key-window callback into the runtime host.
  public func windowDidBecomeKey() {
    sendLifecycleState(.window, .running)
  }

  /// Forward one resign-key callback into the runtime host.
  public func windowDidResignKey() {
    sendLifecycleState(.window, .paused)
  }

  /// Forward one close callback into the runtime host.
  public func windowWillClose() {
    sendLifecycleState(.window, .destroyed)
  }

  /// Send one window lifecycle state through the runtime host.
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

#if canImport(AppKit)
  extension WindowDelegate: NSWindowDelegate {
    public func windowDidBecomeKey(
      _ notification: Notification
    ) {
      windowDidBecomeKey()
    }

    public func windowDidResignKey(
      _ notification: Notification
    ) {
      windowDidResignKey()
    }

    public func windowWillClose(
      _ notification: Notification
    ) {
      windowWillClose()
    }
  }
#endif
