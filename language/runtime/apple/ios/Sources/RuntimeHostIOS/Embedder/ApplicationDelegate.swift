import RuntimeHostAppleCore
import Foundation

#if canImport(UIKit)
import UIKit
#endif

/// The application-delegate adapter for one iOS runtime host.
@MainActor
public final class ApplicationDelegate: NSObject {
    /// The iOS runtime host receiving application callbacks.
    public let runtimeHost: RuntimeHost

    /// Create one application-delegate adapter.
    public init(
        runtimeHost: RuntimeHost
    ) {
        self.runtimeHost = runtimeHost
        super.init()
    }

    /// Forward one application launch callback into the runtime host.
    @discardableResult
    public func applicationDidFinishLaunching() -> Bool {
        sendLifecycleState(.application, .initializing)
        return true
    }

    /// Forward one application active callback into the runtime host.
    public func applicationDidBecomeActive() {
        sendLifecycleState(.application, .running)
    }

    /// Forward one application resign-active callback into the runtime host.
    public func applicationWillResignActive() {
        sendLifecycleState(.application, .paused)
    }

    /// Forward one application background callback into the runtime host.
    public func applicationDidEnterBackground() {
        sendLifecycleState(.application, .paused)
    }

    /// Forward one application foreground callback into the runtime host.
    public func applicationWillEnterForeground() {
        sendLifecycleState(.application, .running)
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
        runtimeHost.lifecycleEvents.sendLifecycleEvent(
            RuntimeHostLifecycleEvent(
                sourceKind: sourceKind,
                state: state
            )
        )
    }
}

#if canImport(UIKit)
extension ApplicationDelegate: UIApplicationDelegate {
    public func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil
    ) -> Bool {
        applicationDidFinishLaunching()
    }

    public func applicationDidBecomeActive(
        _ application: UIApplication
    ) {
        applicationDidBecomeActive()
    }

    public func applicationWillResignActive(
        _ application: UIApplication
    ) {
        applicationWillResignActive()
    }

    public func applicationDidEnterBackground(
        _ application: UIApplication
    ) {
        applicationDidEnterBackground()
    }

    public func applicationWillEnterForeground(
        _ application: UIApplication
    ) {
        applicationWillEnterForeground()
    }

    public func applicationWillTerminate(
        _ application: UIApplication
    ) {
        applicationWillTerminate()
    }
}
#endif
