import Foundation
import RuntimeHostAppleCore

#if canImport(AppKit)
  import AppKit
#endif

/// The macOS intent request surface backed by the shared application host.
@MainActor
public final class WorkspaceIntentRequests: IntentRequests {

  /// Return whether one outbound URL can be opened by the Apple host.
  public func canOpenURL(_ url: String) -> Bool {
    #if canImport(AppKit)
      guard let url = URL(string: url) else {
        return false
      }

      return NSWorkspace.shared.urlForApplication(toOpen: url) != nil
    #else
      return false
    #endif
  }

  /// Open one outbound URL through the Apple host.
  public func openURL(_ url: String) -> UInt32 {
    #if canImport(AppKit)
      guard let url = URL(string: url) else {
        return hostStatusInvalidArgument
      }

      return NSWorkspace.shared.open(url) ? hostStatusOk : hostStatusNotSupported
    #else
      return hostStatusNotSupported
    #endif
  }

  /// Open one outbound file path through the Apple host.
  public func openPath(_ path: String) -> UInt32 {
    #if canImport(AppKit)
      let url = URL(fileURLWithPath: path)

      return NSWorkspace.shared.open(url) ? hostStatusOk : hostStatusNotSupported
    #else
      return hostStatusNotSupported
    #endif
  }

  /// Share one outbound text payload through the Apple host.
  public func shareText(
    _ text: String,
    contentType: String?
  ) -> UInt32 {
    let _ = text
    let _ = contentType

    return hostStatusNotSupported
  }

  /// Share one outbound file path list through the Apple host.
  public func sharePaths(
    _ paths: [String],
    contentType: String?
  ) -> UInt32 {
    let _ = paths
    let _ = contentType

    return hostStatusNotSupported
  }
}
