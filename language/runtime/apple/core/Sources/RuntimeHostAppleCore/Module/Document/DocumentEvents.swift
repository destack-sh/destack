import Foundation

/// The document event surface attached to one Apple runtime host.
@MainActor
public protocol DocumentEvents: AnyObject {
  /// Send one normalized document result into the attached runtime session.
  func sendDocumentResult(_ result: RuntimeHostDocumentResult)
}
