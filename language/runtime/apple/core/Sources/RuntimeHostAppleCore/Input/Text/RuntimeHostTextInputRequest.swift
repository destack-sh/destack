import Foundation

/// One host text-selection range.
public struct RuntimeHostTextInputRange: Sendable, Hashable, Codable {
  /// The inclusive selection start offset.
  public let startOffset: Int
  /// The exclusive selection end offset.
  public let endOffset: Int

  /// Create one text-selection range.
  public init(
    startOffset: Int,
    endOffset: Int
  ) {
    self.startOffset = startOffset
    self.endOffset = endOffset
  }
}

/// One host text-input rectangle in local logical units.
public struct RuntimeHostTextInputRectangle: Sendable, Hashable, Codable {
  /// The left edge in local logical units.
  public let x: Double
  /// The top edge in local logical units.
  public let y: Double
  /// The rectangle width in local logical units.
  public let width: Double
  /// The rectangle height in local logical units.
  public let height: Double

  /// Create one text-input rectangle.
  public init(
    x: Double,
    y: Double,
    width: Double,
    height: Double
  ) {
    self.x = x
    self.y = y
    self.width = width
    self.height = height
  }
}

/// One host text-input affine transform into target-local coordinates.
public struct RuntimeHostTextInputTransform2D: Sendable, Hashable, Codable {
  /// The first-row X coefficient.
  public let xx: Double
  /// The first-row Y coefficient.
  public let xy: Double
  /// The second-row X coefficient.
  public let yx: Double
  /// The second-row Y coefficient.
  public let yy: Double
  /// The translation X component.
  public let tx: Double
  /// The translation Y component.
  public let ty: Double

  /// Create one text-input transform.
  public init(
    xx: Double,
    xy: Double,
    yx: Double,
    yy: Double,
    tx: Double,
    ty: Double
  ) {
    self.xx = xx
    self.xy = xy
    self.yx = yx
    self.yy = yy
    self.tx = tx
    self.ty = ty
  }
}

/// One host text-input geometry hint.
public struct RuntimeHostTextInputGeometry: Sendable, Hashable, Codable {
  /// The local-to-target transform.
  public let localToTargetTransform: RuntimeHostTextInputTransform2D
  /// The full editor rectangle.
  public let editorRectangle: RuntimeHostTextInputRectangle
  /// The caret rectangle when one focused insertion point is known.
  public let caretRectangle: RuntimeHostTextInputRectangle?
  /// The composing rectangle when one active composition span is known.
  public let composingRectangle: RuntimeHostTextInputRectangle?

  /// Create one text-input geometry hint.
  public init(
    localToTargetTransform: RuntimeHostTextInputTransform2D,
    editorRectangle: RuntimeHostTextInputRectangle,
    caretRectangle: RuntimeHostTextInputRectangle? = nil,
    composingRectangle: RuntimeHostTextInputRectangle? = nil
  ) {
    self.localToTargetTransform = localToTargetTransform
    self.editorRectangle = editorRectangle
    self.caretRectangle = caretRectangle
    self.composingRectangle = composingRectangle
  }
}

/// One host text-input type hint.
public enum RuntimeHostTextInputType: Int, Sendable, Hashable, Codable {
  /// Plain text entry.
  case text
  /// Numeric entry.
  case number
  /// Email entry.
  case email
  /// URL entry.
  case url
  /// Password entry.
  case password
  /// Phone entry.
  case phone
  /// Search entry.
  case search
}

/// One host text-input state snapshot.
public struct RuntimeHostTextInputState: Sendable, Hashable, Codable {
  /// The full editor text.
  public let text: String
  /// The current selection range.
  public let selection: RuntimeHostTextInputRange
  /// The composing range when one IME composition is active.
  public let composing: RuntimeHostTextInputRange?

  /// Create one text-input state.
  public init(
    text: String,
    selection: RuntimeHostTextInputRange,
    composing: RuntimeHostTextInputRange? = nil
  ) {
    self.text = text
    self.selection = selection
    self.composing = composing
  }
}

/// One host text input configuration submitted by one runtime session.
public struct RuntimeHostTextInputConfiguration: Sendable, Hashable, Codable {
  /// The stable text session identifier.
  public let sessionID: UInt64
  /// The text input type hint.
  public let inputType: RuntimeHostTextInputType
  /// Whether the text input session is multiline.
  public let isMultiline: Bool
  /// Whether the text input session is secure or password-like.
  public let isSecure: Bool

  /// Create one text input configuration.
  public init(
    sessionID: UInt64,
    inputType: RuntimeHostTextInputType = .text,
    isMultiline: Bool = false,
    isSecure: Bool = false
  ) {
    self.sessionID = sessionID
    self.inputType = inputType
    self.isMultiline = isMultiline
    self.isSecure = isSecure
  }
}

/// One Apple text input open request submitted by one runtime session.
public struct RuntimeHostTextInputOpenRequest: Sendable, Hashable, Codable {
  /// The text input configuration for the requested attachment.
  public let configuration: RuntimeHostTextInputConfiguration
  /// The initial renderer-owned text state.
  public let state: RuntimeHostTextInputState
  /// Create one text input open request.
  public init(
    configuration: RuntimeHostTextInputConfiguration,
    state: RuntimeHostTextInputState
  ) {
    self.configuration = configuration
    self.state = state
  }
}

/// One Apple text input close request submitted by one runtime session.
public struct RuntimeHostTextInputCloseRequest: Sendable, Hashable, Codable {
  /// The stable text session identifier to close.
  public let sessionID: UInt64

  /// Create one text input close request.
  public init(sessionID: UInt64) {
    self.sessionID = sessionID
  }
}

/// One Apple text input geometry update submitted by one runtime session.
public struct RuntimeHostTextInputGeometryRequest: Sendable, Hashable, Codable {
  /// The stable text session identifier to update.
  public let sessionID: UInt64
  /// The next text input geometry hint.
  public let geometry: RuntimeHostTextInputGeometry

  /// Create one text input geometry request.
  public init(
    sessionID: UInt64,
    geometry: RuntimeHostTextInputGeometry
  ) {
    self.sessionID = sessionID
    self.geometry = geometry
  }
}

/// One Apple text input state update submitted by one runtime session.
public struct RuntimeHostTextInputStateRequest: Sendable, Hashable, Codable {
  /// The stable text session identifier to update.
  public let sessionID: UInt64
  /// The next renderer-owned text state.
  public let state: RuntimeHostTextInputState

  /// Create one text input state request.
  public init(
    sessionID: UInt64,
    state: RuntimeHostTextInputState
  ) {
    self.sessionID = sessionID
    self.state = state
  }
}
