import Foundation

/// The document request surface attached to one Apple runtime host.
@MainActor
public protocol DocumentRequests: AnyObject {
  /// Submit one document request to the Apple host.
  func submitDocumentRequest(_ request: RuntimeHostDocumentRequest)
}
