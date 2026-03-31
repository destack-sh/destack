import Foundation
import RuntimeHostAppleCore

/// One bridge attach failure.
public struct BridgeError: Error, Equatable {
  /// The failure message.
  public let message: String

  /// Create one bridge error message.
  init(
    _ message: String
  ) {
    self.message = message
  }
}

final class WeakRuntimeBridge {
  /// The weak bridge reference.
  weak var value: RuntimeBridge?

  /// Create one weak bridge box.
  init(value: RuntimeBridge) {
    self.value = value
  }
}

/// One process-local registry for live runtime bridges.
enum RuntimeBridgeRegistry {
  private static let lock = NSLock()
  private nonisolated(unsafe) static var bridges: [UInt64: WeakRuntimeBridge] = [:]

  /// Register one live bridge for one runtime session.
  static func insert(_ bridge: RuntimeBridge, sessionHandle: HostSessionHandle) {
    lock.lock()
    defer { lock.unlock() }

    bridges[sessionHandle.rawValue] = WeakRuntimeBridge(value: bridge)
  }

  /// Remove one bridge registration for one runtime session.
  static func remove(sessionHandle: HostSessionHandle) {
    lock.lock()
    defer { lock.unlock() }

    bridges.removeValue(forKey: sessionHandle.rawValue)
  }

  /// Resolve one bridge for one runtime session.
  static func resolve(sessionHandle: UInt64) -> RuntimeBridge? {
    lock.lock()
    defer { lock.unlock() }

    return bridges[sessionHandle]?.value
  }
}

/// One runtime-backed bridge for one attached iOS runtime host.
@MainActor
public final class RuntimeBridge: GeneratedRuntimeBridge, BackgroundEvents, IntentEvents,
  NotificationEvents, TextEvents
{
  fileprivate nonisolated(unsafe) weak var runtimeHost: RuntimeHost?

  /// Create one runtime bridge for one runtime session using one injected runtime session API.
  init(
    sessionHandle: HostSessionHandle,
    runtimeApi: any RuntimeIngress,
    notificationTimestampNs: @escaping () -> UInt64 = {
      DispatchTime.now().uptimeNanoseconds
    }
  ) {
    super.init(
      sessionHandle: sessionHandle,
      runtimeApi: runtimeApi,
      timestampNs: notificationTimestampNs
    )
  }

  /// Create one runtime bridge for one runtime session using the live process ABI.
  public convenience init(
    sessionHandle: HostSessionHandle
  ) {
    self.init(
      sessionHandle: sessionHandle,
      runtimeApi: ProcessRuntimeIngress(),
      notificationTimestampNs: {
        DispatchTime.now().uptimeNanoseconds
      }
    )
  }

  deinit {
    RuntimeBridgeRegistry.remove(sessionHandle: sessionHandle)
  }

  /// Attach one runtime host and register the bridge callback lanes.
  public func attach(
    runtimeHost: RuntimeHost
  ) throws {
    guard runtimeHost.sessionHandle == sessionHandle else {
      throw BridgeError(
        "runtime bridge session handle does not match the attached runtime host"
      )
    }

    self.runtimeHost = runtimeHost
    RuntimeBridgeRegistry.insert(self, sessionHandle: sessionHandle)

    // register one session callback table after every lane is attached
    let attachStatus = runtimeApi.attachBridge(
      sessionHandle: sessionHandle,
      bridge: self
    )
    guard attachStatus == hostStatusOk else {
      RuntimeBridgeRegistry.remove(sessionHandle: sessionHandle)
      self.runtimeHost = nil
      throw BridgeError(
        "runtime bridge could not attach runtime session api: \(attachStatus)"
      )
    }
  }

  /// Remove the bridge registration for this runtime session.
  public func detach() {
    runtimeApi.detachBridge(sessionHandle: sessionHandle)
    runtimeHost = nil
    RuntimeBridgeRegistry.remove(sessionHandle: sessionHandle)
  }
}

/// Resolve one attached runtime host for one runtime session.
func resolveRuntimeHost(
  sessionHandle: UInt64
) -> RuntimeHost? {
  RuntimeBridgeRegistry.resolve(sessionHandle: sessionHandle)?.runtimeHost
}

/// Resolve one attached runtime bridge and runtime host for one session.
@MainActor
func withResolvedRuntimeBridge<T>(
  sessionHandle: UInt64,
  onMissing: () -> T,
  body: (RuntimeBridge, RuntimeHost) -> T
) -> T {
  guard let bridge = RuntimeBridgeRegistry.resolve(sessionHandle: sessionHandle) else {
    return onMissing()
  }

  guard let runtimeHost = bridge.runtimeHost else {
    return onMissing()
  }

  return body(bridge, runtimeHost)
}
