import Foundation

/// One host text-selection range.
public struct RuntimeHostTextSelectionRange: Sendable, Hashable, Codable {
    /// The inclusive selection start offset.
    public let start: Int
    /// The exclusive selection end offset.
    public let end: Int

    /// Create one text-selection range.
    public init(
        start: Int,
        end: Int
    ) {
        self.start = start
        self.end = end
    }
}

/// One host text input configuration submitted by one runtime session.
public struct RuntimeHostTextInputConfiguration: Sendable, Hashable, Codable {
    /// The stable text input attachment identifier.
    public let identifier: String
    /// Whether the text input session is multiline.
    public let isMultiline: Bool
    /// Whether the text input session is secure or password-like.
    public let isSecure: Bool

    /// Create one text input configuration.
    public init(
        identifier: String,
        isMultiline: Bool = false,
        isSecure: Bool = false
    ) {
        self.identifier = identifier
        self.isMultiline = isMultiline
        self.isSecure = isSecure
    }
}

/// One Apple text input open request submitted by one runtime session.
public struct RuntimeHostTextInputOpenRequest: Sendable, Hashable, Codable {
    /// The text input configuration for the requested attachment.
    public let configuration: RuntimeHostTextInputConfiguration

    /// Create one text input open request.
    public init(configuration: RuntimeHostTextInputConfiguration) {
        self.configuration = configuration
    }
}

/// One Apple text input close request submitted by one runtime session.
public struct RuntimeHostTextInputCloseRequest: Sendable, Hashable, Codable {
    /// The stable text input attachment identifier to close.
    public let identifier: String

    /// Create one text input close request.
    public init(identifier: String) {
        self.identifier = identifier
    }
}
