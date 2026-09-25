use tspp_source::Span;

use crate::JsonTrivia;

/// A complete JSON document with optional leading/trailing trivia.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonDocument {
    /// The root value.
    pub value: JsonValue,
    /// Trivia before the value (comments, whitespace).
    pub trivia_before: Vec<JsonTrivia>,
    /// Trivia after the value (comments, whitespace).
    pub trivia_after: Vec<JsonTrivia>,
    /// The span of the entire document.
    pub span: Span,
}

/// A JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    /// The `null` literal.
    Null {
        /// The span of the null keyword.
        span: Span,
    },
    /// A boolean literal (`true` or `false`).
    Bool {
        /// The boolean value.
        value: bool,
        /// The span of the boolean keyword.
        span: Span,
    },
    /// A numeric literal.
    Number {
        /// The raw string representation (preserves formatting).
        raw: String,
        /// The span of the number.
        span: Span,
    },
    /// A string literal.
    String {
        /// The unescaped string value.
        value: String,
        /// The span including quotes.
        span: Span,
    },
    /// An array literal.
    Array {
        /// The array elements.
        elements: Vec<JsonElement>,
        /// The span including brackets.
        span: Span,
    },
    /// An object literal.
    Object {
        /// The object properties.
        properties: Vec<JsonProperty>,
        /// The span including braces.
        span: Span,
    },
}

impl JsonValue {
    /// Get the span of this value.
    pub fn span(&self) -> Span {
        match self {
            JsonValue::Null { span } => *span,
            JsonValue::Bool { span, .. } => *span,
            JsonValue::Number { span, .. } => *span,
            JsonValue::String { span, .. } => *span,
            JsonValue::Array { span, .. } => *span,
            JsonValue::Object { span, .. } => *span,
        }
    }
}

/// An element in a JSON array.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonElement {
    /// The element value.
    pub value: JsonValue,
    /// The trailing comma span (if present).
    pub comma: Option<Span>,
    /// Trivia before this element.
    pub trivia_before: Vec<JsonTrivia>,
    /// Trivia after this element (before comma or next element).
    pub trivia_after: Vec<JsonTrivia>,
}

/// A property in a JSON object.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonProperty {
    /// The property key.
    pub key: JsonString,
    /// The colon span.
    pub colon: Span,
    /// The property value.
    pub value: JsonValue,
    /// The trailing comma span (if present).
    pub comma: Option<Span>,
    /// Trivia before this property.
    pub trivia_before: Vec<JsonTrivia>,
    /// Trivia after the value (before comma or next property).
    pub trivia_after: Vec<JsonTrivia>,
}

/// A JSON string (used for keys and string values).
#[derive(Debug, Clone, PartialEq)]
pub struct JsonString {
    /// The unescaped string content.
    pub value: String,
    /// The span including quotes.
    pub span: Span,
}
