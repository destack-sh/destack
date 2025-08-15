//! destack.core.common.text@2025.08.15.1

#![destack::partial(destack.core.common.text, file)]

#[destack::generated(Text, , block)]
/// Text is a single paragraph composed of TextSpans with inline styling.
pub struct Text {
    spans: Vec<TextSpan>,
    style_flags: TextStyleFlag,
}

#[destack::generated(TextSpan, , block)]
/// Span of text with optional styling.
pub struct TextSpan {
    r#type: TextSpanType,
    content: Option<String>,
    node: Option<i64 /* TODO */>,
    url: Option<String>,
    style_flags: TextStyleFlag,
}

#[destack::generated(TextSpanType, , block)]
/// TextSpanType
pub enum TextSpanType {
    /// Formatted text
    Text = 1,
    /// Hard break
    HardBreak = 2,
    /// Reference to a Node
    Node = 10,
    /// Hyperlink
    Link = 11,
    /// TeX equation
    Equation = 20,
}

#[destack::generated(TextStyleFlag, , block)]
/// A flag that can be applied to a TextSpan.
pub enum TextStyleFlag {
    Default = 0,
    Bold = 1,
    Italic = 2,
    Strikethrough = 4,
    Underline = 8,
    Code = 16,
}
