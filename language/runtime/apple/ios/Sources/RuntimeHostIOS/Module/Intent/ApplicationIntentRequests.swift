import Foundation
import RuntimeHostAppleCore

#if canImport(UIKit)
  import UIKit
#endif

/// The iOS intent request surface backed by the shared application host.
@MainActor
public final class ApplicationIntentRequests: IntentRequests {

  public func canOpenURL(_ url: String) -> Bool {
    #if canImport(UIKit)
      guard let url = URL(string: url) else {
        return false
      }

      return UIApplication.shared.canOpenURL(url)
    #else
      return false
    #endif
  }

  public func openURL(_ url: String) -> UInt32 {
    #if canImport(UIKit)
      guard let url = URL(string: url) else {
        return hostStatusInvalidArgument
      }

      UIApplication.shared.open(url)

      return hostStatusOk
    #else
      return hostStatusNotSupported
    #endif
  }

  public func openPath(_ path: String) -> UInt32 {
    #if canImport(UIKit)
      let url = URL(fileURLWithPath: path)

      UIApplication.shared.open(url)

      return hostStatusOk
    #else
      return hostStatusNotSupported
    #endif
  }

  public func shareText(
    _ text: String,
    contentType: String?
  ) -> UInt32 {
    let _ = contentType

    return presentShareSheet(items: [text])
  }

  public func sharePaths(
    _ paths: [String],
    contentType: String?
  ) -> UInt32 {
    let _ = contentType
    let urls = paths.map { URL(fileURLWithPath: $0) }

    return presentShareSheet(items: urls)
  }
}

/// Present one iOS share sheet from the active scene.
@MainActor
private func presentShareSheet(items: [Any]) -> UInt32 {
  #if canImport(UIKit)
    guard let presenter = topViewController() else {
      return hostStatusNotSupported
    }

    let controller = UIActivityViewController(
      activityItems: items,
      applicationActivities: nil
    )

    presenter.present(controller, animated: true)

    return hostStatusOk
  #else
    let _ = items

    return hostStatusNotSupported
  #endif
}

#if canImport(UIKit)
  /// Return the top-most presenter from the active application scene.
  @MainActor
  private func topViewController() -> UIViewController? {
    let scenes = UIApplication.shared.connectedScenes
      .compactMap { $0 as? UIWindowScene }
    let window =
      scenes
      .flatMap(\.windows)
      .first { $0.isKeyWindow }

    var controller = window?.rootViewController

    while let presented = controller?.presentedViewController {
      controller = presented
    }

    return controller
  }
#endif
