import Foundation

/// One Apple text input ingress event delivered into one runtime session.
public struct RuntimeHostTextInputEvent: Sendable, Hashable, Codable {
    /// The stable text input attachment identifier.
    public let identifier: String
    /// The current text state reported by the Apple host.
    public let text: String
    /// The current selection range.
    public let selection: RuntimeHostTextSelectionRange
    /// The optional composing range when one IME composition is active.
    public let composing: RuntimeHostTextSelectionRange?

    /// Create one text input event.
    public init(
        identifier: String,
        text: String,
        selection: RuntimeHostTextSelectionRange,
        composing: RuntimeHostTextSelectionRange? = nil
    ) {
        self.identifier = identifier
        self.text = text
        self.selection = selection
        self.composing = composing
    }
}
