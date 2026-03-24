import Foundation
import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

@testable import RuntimeHostIOS

/// One process-local registry for runtime ABI spy callbacks.
private enum RuntimeAbiSpyBridgeRegistry {
  private static let lock = NSLock()
  private nonisolated(unsafe) static var bridges: [UInt64: RuntimeBridge] = [:]

  /// Register one bridge for one session handle.
  static func insert(_ bridge: RuntimeBridge, sessionHandle: HostSessionHandle) {
    lock.lock()
    defer { lock.unlock() }

    bridges[sessionHandle.rawValue] = bridge
  }

  /// Remove one bridge for one session handle.
  static func remove(sessionHandle: HostSessionHandle) {
    lock.lock()
    defer { lock.unlock() }

    bridges.removeValue(forKey: sessionHandle.rawValue)
  }

  /// Resolve one bridge for one session handle.
  static func resolve(_ sessionHandle: UInt64) -> RuntimeBridge? {
    lock.lock()
    defer { lock.unlock() }

    return bridges[sessionHandle]
  }
}

/// Submit one document request through the registered runtime bridge.
private let runtimeAbiSpyDocumentCallback: DocumentCallback = { sessionHandle, request in
  guard let bridge = RuntimeAbiSpyBridgeRegistry.resolve(sessionHandle) else {
    return hostStatusNotFound
  }
  guard let mimeTypes = decodeBridgeStrings(request.mime_types) else {
    return hostStatusInvalidArgument
  }
  guard let extensions = decodeBridgeStrings(request.extensions) else {
    return hostStatusInvalidArgument
  }

  let contentTypes = mimeTypes + extensions.map { "application/x.destack-extension.\($0)" }
  let request = RuntimeHostDocumentRequest(
    requestID: HostRequestID(rawValue: request.request_id),
    allowsMultipleSelection: request.allows_multiple_selection,
    contentTypes: contentTypes
  )

  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  return runOnMainThread {
    bridge.documentBridge.submitRequest(
      runtimeHost: runtimeHost,
      request
    )
  }
}

/// Open permission settings through the registered runtime bridge.
private let runtimeAbiSpyPermissionOpenSettingsCallback: PermissionOpenSettingsCallback = {
  sessionHandle in
  guard let bridge = RuntimeAbiSpyBridgeRegistry.resolve(sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  return runOnMainThread {
    bridge.permissionBridge.openSettings(runtimeHost: runtimeHost)
  }
}

/// Submit one permission request through the registered runtime bridge.
private let runtimeAbiSpyPermissionRequestCallback: PermissionRequestCallback = {
  sessionHandle,
  request in
  guard let bridge = RuntimeAbiSpyBridgeRegistry.resolve(sessionHandle) else {
    return hostStatusNotFound
  }
  guard let permissions = decodeBridgeStrings(request.permissions) else {
    return hostStatusInvalidArgument
  }
  guard permissions.count == 1 else {
    return hostStatusInvalidArgument
  }

  let request = RuntimeHostPermissionRequest(
    requestID: HostRequestID(rawValue: request.request_id),
    permission: permissions[0]
  )

  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  return runOnMainThread {
    bridge.permissionBridge.submitRequest(
      runtimeHost: runtimeHost,
      request
    )
  }
}

/// Record runtime ABI interactions for one bridge test.
final class RuntimeAbiSpy: RuntimeAbi, @unchecked Sendable {
  var attachedSessionHandles: [HostSessionHandle] = []
  var detachedSessionHandles: [HostSessionHandle] = []
  var documentResults: [(HostSessionHandle, HostRequestID, [RuntimeHostDocumentDescriptor])] = []
  var permissionEvents: [(HostSessionHandle, HostRequestID?, String, Bool)] = []
  var intentEvents: [(HostSessionHandle, RuntimeHostIntentEvent)] = []
  var locationSamples: [(HostSessionHandle, String, RuntimeHostLocationSample)] = []
  var notificationEvents: [(HostSessionHandle, RuntimeHostNotificationEvent)] = []
  var attachStatus: UInt32 = hostStatusOk
  var documentCallback: DocumentCallback?
  var permissionRequestCallback: PermissionRequestCallback?
  var permissionOpenSettingsCallback: PermissionOpenSettingsCallback?
  var calendarListCallback: CalendarListCallback?
  var calendarEventListCallback: CalendarEventListCallback?
  var calendarEventReadCallback: CalendarEventReadCallback?
  var calendarEventCreateCallback: CalendarEventCreateCallback?
  var calendarEventUpdateCallback: CalendarEventUpdateCallback?
  var calendarEventDeleteCallback: CalendarEventDeleteCallback?
  var contactListCallback: ContactListCallback?
  var contactSearchCallback: ContactSearchCallback?
  var contactReadCallback: ContactReadCallback?
  var contactCreateCallback: ContactCreateCallback?
  var contactUpdateCallback: ContactUpdateCallback?
  var contactDeleteCallback: ContactDeleteCallback?
  var intentCanOpenURLCallback: IntentCanOpenURLCallback?
  var intentOpenURLCallback: IntentOpenURLCallback?
  var intentOpenPathCallback: IntentOpenPathCallback?
  var intentShareTextCallback: IntentShareTextCallback?
  var intentSharePathsCallback: IntentSharePathsCallback?
  var locationServicesEnabledCallback: LocationServicesEnabledCallback?
  var locationLastKnownCallback: LocationLastKnownCallback?
  var locationWatchOpenCallback: LocationWatchOpenCallback?
  var locationWatchCloseCallback: LocationWatchCloseCallback?
  var mediaListCallback: MediaListCallback?
  var mediaReadCallback: MediaReadCallback?
  var mediaImportPathCallback: MediaImportPathCallback?
  var mediaDeleteCallback: MediaDeleteCallback?
  var notificationPostCallback: NotificationPostCallback?
  var notificationCancelCallback: NotificationCancelCallback?
  var notificationCancelAllCallback: NotificationCancelAllCallback?

  func attachBridge(
    sessionHandle: HostSessionHandle,
    bridge: RuntimeBridge
  ) -> HostAbiStatus {
    attachedSessionHandles.append(sessionHandle)
    documentCallback = runtimeAbiSpyDocumentCallback
    permissionOpenSettingsCallback = runtimeAbiSpyPermissionOpenSettingsCallback
    permissionRequestCallback = runtimeAbiSpyPermissionRequestCallback
    calendarListCallback = { _, _ in hostStatusOk }
    calendarEventListCallback = { _, _, _ in hostStatusOk }
    calendarEventReadCallback = { _, _, _ in hostStatusOk }
    calendarEventCreateCallback = { _, _, _ in hostStatusOk }
    calendarEventUpdateCallback = { _, _, _ in hostStatusOk }
    calendarEventDeleteCallback = { _, _ in hostStatusOk }
    contactListCallback = { _, _, _ in hostStatusOk }
    contactSearchCallback = { _, _, _, _ in hostStatusOk }
    contactReadCallback = { _, _, _ in hostStatusOk }
    contactCreateCallback = { _, _, _ in hostStatusOk }
    contactUpdateCallback = { _, _, _ in hostStatusOk }
    contactDeleteCallback = { _, _ in hostStatusOk }
    intentCanOpenURLCallback = { _, _, _ in hostStatusOk }
    intentOpenURLCallback = { _, _ in hostStatusOk }
    intentOpenPathCallback = { _, _ in hostStatusOk }
    intentShareTextCallback = { _, _, _, _ in hostStatusOk }
    intentSharePathsCallback = { _, _, _, _ in hostStatusOk }
    locationServicesEnabledCallback = { _, enabled in
      guard let enabled else {
        return hostStatusInvalidArgument
      }
      enabled.pointee = true

      return hostStatusOk
    }
    locationLastKnownCallback = { _, sample in
      guard let sample else {
        return hostStatusInvalidArgument
      }
      sample.pointee = DestackRustLocationSample(
        latitude_degrees: 47.3769,
        longitude_degrees: 8.5417,
        altitude_meters: 408.0,
        horizontal_accuracy_meters: 5.0,
        vertical_accuracy_meters: 8.0,
        speed_meters_per_second: 1.25,
        heading_degrees: 180.0,
        timestamp_unix_ns: 1_700_000_000_000_000_000
      )

      return hostStatusOk
    }
    locationWatchOpenCallback = { _, _, _ in hostStatusOk }
    locationWatchCloseCallback = { _, _ in hostStatusOk }
    mediaListCallback = { _, _, _ in hostStatusOk }
    mediaReadCallback = { _, _, _ in hostStatusOk }
    mediaImportPathCallback = { _, _, _, _ in hostStatusOk }
    mediaDeleteCallback = { _, _, _ in hostStatusOk }
    notificationPostCallback = { _, _ in hostStatusOk }
    notificationCancelCallback = { _, _ in hostStatusOk }
    notificationCancelAllCallback = { _ in hostStatusOk }

    if attachStatus == hostStatusOk {
      RuntimeAbiSpyBridgeRegistry.insert(bridge, sessionHandle: sessionHandle)
    }

    return attachStatus
  }

  func detachBridge(
    sessionHandle: HostSessionHandle
  ) {
    detachedSessionHandles.append(sessionHandle)
    RuntimeAbiSpyBridgeRegistry.remove(sessionHandle: sessionHandle)
  }

  func notifyNotificationEvent(
    sessionHandle: HostSessionHandle,
    event: RuntimeHostNotificationEvent,
    sequence: UInt64,
    timestampNs: UInt64
  ) -> RuntimeAbiStatus {
    notificationEvents.append((sessionHandle, event))

    return RuntimeAbiStatus(code: hostStatusOk, errorID: 0)
  }

  func notifyDocumentResult(
    sessionHandle: HostSessionHandle,
    requestID: HostRequestID,
    documents: [RuntimeHostDocumentDescriptor]
  ) -> RuntimeAbiStatus {
    documentResults.append((sessionHandle, requestID, documents))

    return RuntimeAbiStatus(code: hostStatusOk, errorID: 0)
  }

  func notifyPermissionResult(
    sessionHandle: HostSessionHandle,
    requestID: HostRequestID?,
    permission: String,
    isGranted: Bool
  ) -> RuntimeAbiStatus {
    permissionEvents.append((sessionHandle, requestID, permission, isGranted))

    return RuntimeAbiStatus(code: hostStatusOk, errorID: 0)
  }

  func notifyIntentEvent(
    sessionHandle: HostSessionHandle,
    event: RuntimeHostIntentEvent
  ) -> RuntimeAbiStatus {
    intentEvents.append((sessionHandle, event))

    return RuntimeAbiStatus(code: hostStatusOk, errorID: 0)
  }

  func notifyLocationSample(
    sessionHandle: HostSessionHandle,
    watchID: String,
    sample: RuntimeHostLocationSample
  ) -> RuntimeAbiStatus {
    locationSamples.append((sessionHandle, watchID, sample))

    return RuntimeAbiStatus(code: hostStatusOk, errorID: 0)
  }
}

/// Decode one bridge string slice into one Swift string array.
private func decodeBridgeStrings(
  _ values: DestackRustStringSlice
) -> [String]? {
  guard let data = values.data else {
    return values.len == 0 ? [] : nil
  }

  let buffer = UnsafeBufferPointer(start: data, count: Int(values.len))

  return buffer.map { value in
    guard let data = value.data else {
      return ""
    }

    let bytes = UnsafeBufferPointer(start: data, count: Int(value.len))

    return String(decoding: bytes, as: UTF8.self)
  }
}
