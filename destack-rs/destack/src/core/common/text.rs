//! destack.core.common.text@2025.08.14.0

#![destack::partial(destack.core.common.text, file)]

#[destack::generated(Text, struct, block)]
/// Text is a single paragraph composed of TextSpans with inline styling.
pub struct Text {

}

#[destack::generated(TextSpan, struct, block)]
/// Span of text with optional styling.
pub struct TextSpan {

}

#[destack::generated(TextSpanType, enum, block)]
/// TextSpanType
pub enum TextSpanType {
    /// Formatted text
    TEXT = 1,
    /// Hard break
    HARD_BREAK = 2,
    /// Reference to a Node
    NODE = 10,
    /// Hyperlink
    LINK = 11,
    /// TeX equation
    EQUATION = 20
}

#[destack::generated(TextStyleFlag, enum, block)]
/// A flag that can be applied to a TextSpan.
pub enum TextStyleFlag {
    DEFAULT = 0,
    BOLD = 1,
    ITALIC = 2,
    STRIKETHROUGH = 4,
    UNDERLINE = 8,
    CODE = 16
}