import Foundation
import RuntimeHostAppleCore

/// One bridge attach failure.
public struct BridgeError: Error, Equatable {
  /// The failure message.
  public let message: String

  /// Create one bridge error message.
  public init(
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
public final class RuntimeBridge: BackgroundEvents, IntentEvents, NotificationEvents,
  TextInputEvents
{
  /// The runtime session routed through this bridge.
  public let sessionHandle: HostSessionHandle
  private let bindings: any RuntimeAbi
  private let notificationTimestampNs: () -> UInt64
  let documentBridge: DocumentBridge
  let permissionBridge: PermissionBridge
  let contactBridge: ContactBridge
  let calendarBridge: CalendarBridge
  let intentBridge: IntentBridge
  let locationBridge: LocationBridge
  let mediaBridge: MediaBridge
  let notificationBridge: NotificationBridge
  let textBridge: TextBridge
  let backgroundBridge: BackgroundBridge
  fileprivate nonisolated(unsafe) weak var runtimeHost: RuntimeHost?

  /// Create one runtime bridge for one runtime session using one injected bindings surface.
  init(
    sessionHandle: HostSessionHandle,
    bindings: any RuntimeAbi,
    notificationTimestampNs: @escaping () -> UInt64 = {
      DispatchTime.now().uptimeNanoseconds
    }
  ) {
    self.sessionHandle = sessionHandle
    self.bindings = bindings
    self.notificationTimestampNs = notificationTimestampNs
    self.documentBridge = DocumentBridge(
      sessionHandle: sessionHandle,
      bindings: bindings
    )
    self.permissionBridge = PermissionBridge(
      sessionHandle: sessionHandle,
      bindings: bindings
    )
    self.contactBridge = ContactBridge()
    self.calendarBridge = CalendarBridge()
    self.intentBridge = IntentBridge(
      sessionHandle: sessionHandle,
      bindings: bindings
    )
    self.locationBridge = LocationBridge(
      sessionHandle: sessionHandle,
      bindings: bindings
    )
    self.mediaBridge = MediaBridge(
      sessionHandle: sessionHandle,
      bindings: bindings
    )
    self.notificationBridge = NotificationBridge(
      sessionHandle: sessionHandle,
      bindings: bindings,
      timestampNs: notificationTimestampNs
    )
    self.textBridge = TextBridge(
      sessionHandle: sessionHandle,
      bindings: bindings
    )
    self.backgroundBridge = BackgroundBridge(
      sessionHandle: sessionHandle,
      bindings: bindings
    )
  }

  /// Create one runtime bridge for one runtime session using the live process ABI.
  public convenience init(
    sessionHandle: HostSessionHandle
  ) {
    self.init(
      sessionHandle: sessionHandle,
      bindings: ProcessRuntimeAbi(),
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
    let attachStatus = bindings.attachBridge(
      sessionHandle: sessionHandle,
      bridge: self
    )
    guard attachStatus == hostStatusOk else {
      RuntimeBridgeRegistry.remove(sessionHandle: sessionHandle)
      self.runtimeHost = nil
      throw BridgeError(
        "runtime bridge could not attach runtime abi bindings: \(attachStatus)"
      )
    }
  }

  /// Remove the bridge registration for this runtime session.
  public func detach() {
    bindings.detachBridge(sessionHandle: sessionHandle)
    runtimeHost = nil
    RuntimeBridgeRegistry.remove(sessionHandle: sessionHandle)
  }

  /// Send one permission result into the runtime ingress path.
  public func sendPermissionEvent(
    _ event: RuntimeHostPermissionEvent
  ) {
    permissionBridge.sendPermissionEvent(event)
  }

  /// Send one document result into the runtime ingress path.
  public func sendDocumentResult(
    _ result: RuntimeHostDocumentResult
  ) {
    documentBridge.sendDocumentResult(result)
  }

  /// Send one intent event into the runtime ingress path.
  public func sendIntentEvent(
    _ event: RuntimeHostIntentEvent
  ) {
    intentBridge.sendIntentEvent(event)
  }

  /// Send one location sample into the runtime ingress path.
  public func sendLocationSample(
    watchID: String,
    sample: RuntimeHostLocationSample
  ) {
    locationBridge.sendLocationSample(
      watchID: watchID,
      sample: sample
    )
  }

  /// Send one notification event into the runtime ingress path.
  public func sendNotificationEvent(
    _ event: RuntimeHostNotificationEvent
  ) {
    notificationBridge.sendNotificationEvent(event)
  }

  /// Send one background event into the runtime ingress path.
  public func sendBackgroundEvent(
    _ event: RuntimeHostBackgroundEvent
  ) {
    backgroundBridge.sendBackgroundEvent(event)
  }

  /// Send one text-input event into the runtime ingress path.
  public func sendTextInputEvent(
    _ event: RuntimeHostTextInputEvent
  ) {
    textBridge.sendTextInputEvent(event)
  }
}

/// Resolve one registered document bridge for one runtime session.
func guardDocumentBridge(
  sessionHandle: UInt64
) -> DocumentBridge? {
  RuntimeBridgeRegistry.resolve(sessionHandle: sessionHandle)?.documentBridge
}

/// Resolve one attached runtime host for one runtime session.
func resolveRuntimeHost(
  sessionHandle: UInt64
) -> RuntimeHost? {
  RuntimeBridgeRegistry.resolve(sessionHandle: sessionHandle)?.runtimeHost
}

/// Resolve one registered permission bridge for one runtime session.
func guardPermissionBridge(
  sessionHandle: UInt64
) -> PermissionBridge? {
  RuntimeBridgeRegistry.resolve(sessionHandle: sessionHandle)?.permissionBridge
}
