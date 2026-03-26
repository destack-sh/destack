import Foundation

/// One Apple text input ingress event delivered into one runtime session.
public struct RuntimeHostTextInputEvent: Sendable, Hashable, Codable {
  /// The stable text session identifier.
  public let sessionID: UInt64
  /// The current text state reported by the Apple host.
  public let state: RuntimeHostTextInputState

  /// Create one text input event.
  public init(
    sessionID: UInt64,
    state: RuntimeHostTextInputState
  ) {
    self.sessionID = sessionID
    self.state = state
  }
}
