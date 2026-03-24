import Foundation
import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

/// One C callback for one deferred permission request.
typealias PermissionRequestCallback =
  @convention(c) (UInt64, DestackRustPermissionRequest) -> UInt32

/// One C callback for one immediate permission-settings request.
typealias PermissionOpenSettingsCallback =
  @convention(c) (UInt64) -> UInt32

/// One low-level permission lane ABI surface for one iOS host bridge.
protocol PermissionAbi {
  /// Deliver one permission result into one runtime session.
  func notifyPermissionResult(
    sessionHandle: HostSessionHandle,
    requestID: HostRequestID?,
    permission: String,
    isGranted: Bool
  ) -> RuntimeAbiStatus
}

extension ProcessRuntimeAbi {
  /// Deliver one permission result into one runtime session.
  func notifyPermissionResult(
    sessionHandle: HostSessionHandle,
    requestID: HostRequestID?,
    permission: String,
    isGranted: Bool
  ) -> RuntimeAbiStatus {
    let status = permission.utf8CString.withUnsafeBufferPointer { buffer in
      let permissionPointer = buffer.baseAddress.map {
        UnsafeRawPointer($0).assumingMemoryBound(to: UInt8.self)
      }

      return destack_runtime_host_ios_notify_permission_result(
        sessionHandle.rawValue,
        requestID != nil,
        requestID?.rawValue ?? 0,
        permissionPointer,
        UInt32(buffer.count - 1),
        isGranted
      )
    }

    return RuntimeAbiStatus(
      code: status.code,
      errorID: status.error_id
    )
  }
}
