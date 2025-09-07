use dyst_language_source::Span;

/// A Span with a message.
#[derive(Debug, Clone)]
pub struct LabeledSpan {
    /// The span of the labeled span.
    pub span: Span,
    /// The message of the labeled span.
    pub message: String,
}
