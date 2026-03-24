import Foundation

/// The permission request surface attached to one Apple runtime host.
@MainActor
public protocol PermissionRequests {
  /// Submit one permission request to the Apple host.
  func submitPermissionRequest(_ request: RuntimeHostPermissionRequest)

  /// Open the native permission settings surface through the Apple host.
  func openPermissionSettings() -> UInt32
}
