import Foundation

/// The Apple host intent payload delivered into one runtime session.
public enum RuntimeHostIntentPayload: Sendable, Hashable {
  /// One request to open one URL.
  case openURL(url: String)
  /// One request to open one file path.
  case openFile(path: String, contentType: String?)
  /// One request to deliver shared text.
  case shareText(text: String, contentType: String?)
  /// One request to deliver shared file paths.
  case shareFiles(paths: [String], contentType: String?)
  /// One custom action payload delivered by the Apple host.
  case customAction(
    action: String,
    url: String?,
    paths: [String],
    text: String?,
    contentType: String?
  )
}

/// One Apple intent ingress event delivered into one runtime session.
public struct RuntimeHostIntentEvent: Sendable, Hashable {
  /// The source bundle or process identifier when available.
  public let source: String?
  /// The intent payload delivered by the Apple host.
  public let payload: RuntimeHostIntentPayload

  /// Create one intent event.
  public init(
    source: String? = nil,
    payload: RuntimeHostIntentPayload
  ) {
    self.source = source
    self.payload = payload
  }
}
