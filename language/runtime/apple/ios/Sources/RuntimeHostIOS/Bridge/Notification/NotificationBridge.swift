import Foundation
import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

let notificationPostCallback: NotificationPostCallback = {
  sessionHandle,
  request in
  handleNotificationPost(
    sessionHandle: sessionHandle,
    request: request
  )
}

let notificationCancelCallback: NotificationCancelCallback = {
  sessionHandle,
  identifier in
  handleNotificationCancel(
    sessionHandle: sessionHandle,
    identifier: identifier
  )
}

let notificationCancelAllCallback: NotificationCancelAllCallback = {
  sessionHandle in
  handleNotificationCancelAll(
    sessionHandle: sessionHandle
  )
}

/// One notification bridge lane for one attached iOS runtime host.
@MainActor
final class NotificationBridge: NotificationEvents {
  /// The runtime session routed through this notification bridge lane.
  let sessionHandle: HostSessionHandle
  private let bindings: any RuntimeAbi
  private let timestampNs: () -> UInt64
  private var nextSequence: UInt64 = 1

  /// Create one notification bridge lane.
  init(
    sessionHandle: HostSessionHandle,
    bindings: any RuntimeAbi,
    timestampNs: @escaping () -> UInt64
  ) {
    self.sessionHandle = sessionHandle
    self.bindings = bindings
    self.timestampNs = timestampNs
  }

  /// Send one notification event into the runtime ingress path.
  func sendNotificationEvent(
    _ event: RuntimeHostNotificationEvent
  ) {
    let sequence = nextSequence
    nextSequence += 1
    let status = bindings.notifyNotificationEvent(
      sessionHandle: sessionHandle,
      event: event,
      sequence: sequence,
      timestampNs: timestampNs()
    )

    precondition(
      status.code == hostStatusOk,
      "runtime bridge could not deliver notification event: code \(status.code), error \(status.errorID)"
    )
  }

  func postNotification(
    runtimeHost: RuntimeHost,
    _ request: RuntimeHostNotificationRequest
  ) -> UInt32 {
    runtimeHost.notificationRequests.postNotification(request)
  }

  func cancelNotification(
    runtimeHost: RuntimeHost,
    identifier: String
  ) -> UInt32 {
    runtimeHost.notificationRequests.cancelNotification(identifier: identifier)
  }

  func cancelAllNotifications(
    runtimeHost: RuntimeHost
  ) -> UInt32 {
    runtimeHost.notificationRequests.cancelAllNotifications()
  }
}

/// Resolve one registered notification bridge for one runtime session.
func guardNotificationBridge(
  sessionHandle: UInt64
) -> NotificationBridge? {
  RuntimeBridgeRegistry.resolve(sessionHandle: sessionHandle)?.notificationBridge
}

/// Handle one runtime callback asking to post one notification.
private func handleNotificationPost(
  sessionHandle: UInt64,
  request: DestackRustNotificationRequest
) -> UInt32 {
  guard let bridge = guardNotificationBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  let request = RuntimeHostNotificationRequest(
    identifier: decodeNativeString(request.identifier),
    title: decodeNativeString(request.title),
    body: decodeNativeString(request.body)
  )

  return runOnMainThread {
    bridge.postNotification(
      runtimeHost: runtimeHost,
      request
    )
  }
}

/// Handle one runtime callback asking to cancel one notification.
private func handleNotificationCancel(
  sessionHandle: UInt64,
  identifier: DestackRustStringRef
) -> UInt32 {
  guard let bridge = guardNotificationBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  let identifier = decodeNativeString(identifier)

  return runOnMainThread {
    bridge.cancelNotification(
      runtimeHost: runtimeHost,
      identifier: identifier
    )
  }
}

/// Handle one runtime callback asking to cancel every notification.
private func handleNotificationCancelAll(
  sessionHandle: UInt64
) -> UInt32 {
  guard let bridge = guardNotificationBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  return runOnMainThread {
    bridge.cancelAllNotifications(runtimeHost: runtimeHost)
  }
}
