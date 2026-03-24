import Foundation
import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

let permissionRequestCallback: PermissionRequestCallback = {
  sessionHandle,
  request in
  handlePermissionRequest(
    sessionHandle: sessionHandle,
    bridgeRequest: request
  )
}

let permissionOpenSettingsCallback: PermissionOpenSettingsCallback = {
  sessionHandle in
  handleOpenPermissionSettings(sessionHandle: sessionHandle)
}

/// One permission bridge lane for one attached iOS runtime host.
@MainActor
final class PermissionBridge: PermissionEvents {
  /// The runtime session routed through this permission bridge lane.
  let sessionHandle: HostSessionHandle
  private let bindings: any RuntimeAbi

  /// Create one permission bridge lane.
  init(
    sessionHandle: HostSessionHandle,
    bindings: any RuntimeAbi
  ) {
    self.sessionHandle = sessionHandle
    self.bindings = bindings
  }

  /// Send one permission result into the runtime ingress path.
  func sendPermissionEvent(_ event: RuntimeHostPermissionEvent) {
    let status = bindings.notifyPermissionResult(
      sessionHandle: sessionHandle,
      requestID: event.requestID,
      permission: event.permission,
      isGranted: event.isGranted
    )

    precondition(
      status.code == hostStatusOk,
      "runtime bridge could not deliver permission event: code \(status.code), error \(status.errorID)"
    )
  }

  func submitRequest(
    runtimeHost: RuntimeHost,
    _ request: RuntimeHostPermissionRequest
  ) -> UInt32 {
    runtimeHost.permissionRequests.submitPermissionRequest(request)

    return hostStatusOk
  }

  func openSettings(
    runtimeHost: RuntimeHost
  ) -> UInt32 {
    return runtimeHost.permissionRequests.openPermissionSettings()
  }
}

/// Decode one permission request from the runtime callback path and submit it through the attached runtime host.
private func handlePermissionRequest(
  sessionHandle: UInt64,
  bridgeRequest: DestackRustPermissionRequest
) -> UInt32 {
  let bridge = guardPermissionBridge(sessionHandle: sessionHandle)
  let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle)
  guard let bridge, let runtimeHost else {
    return hostStatusNotFound
  }

  let request: RuntimeHostPermissionRequest
  do {
    request = try decodePermissionRequest(bridgeRequest)
  } catch {
    return hostStatusInvalidArgument
  }

  return runOnMainThread {
    bridge.submitRequest(
      runtimeHost: runtimeHost,
      request
    )
  }
}

/// Open the native permission settings surface for one runtime callback.
private func handleOpenPermissionSettings(
  sessionHandle: UInt64
) -> UInt32 {
  let bridge = guardPermissionBridge(sessionHandle: sessionHandle)
  let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle)
  guard let bridge, let runtimeHost else {
    return hostStatusNotFound
  }

  let status = runOnMainThread {
    bridge.openSettings(runtimeHost: runtimeHost)
  }

  if status == hostStatusNotFound {
    return hostStatusNotFound
  }

  return status
}

/// Decode one typed bridge payload for the permission bridge lane.
private func decodePermissionRequest(
  _ request: DestackRustPermissionRequest
) throws -> RuntimeHostPermissionRequest {
  let permissions = try tryDecodeNativeStringSlice(request.permissions)
  guard permissions.count == 1 else {
    throw BridgeStringError.invalidStringSlice
  }
  let permission = permissions[0]

  return RuntimeHostPermissionRequest(
    requestID: HostRequestID(rawValue: request.request_id),
    permission: permission
  )
}
