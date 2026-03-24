import Foundation

/// The intent request surface attached to one Apple runtime host.
@MainActor
public protocol IntentRequests: AnyObject {
  /// Return whether one outbound URL can be opened by the Apple host.
  func canOpenURL(_ url: String) -> Bool

  /// Open one outbound URL through the Apple host.
  func openURL(_ url: String) -> UInt32

  /// Open one outbound file path through the Apple host.
  func openPath(_ path: String) -> UInt32

  /// Share one outbound text payload through the Apple host.
  func shareText(
    _ text: String,
    contentType: String?
  ) -> UInt32

  /// Share one outbound file path list through the Apple host.
  func sharePaths(
    _ paths: [String],
    contentType: String?
  ) -> UInt32
}
