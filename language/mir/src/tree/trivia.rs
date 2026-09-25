use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;
use tspp_source::Span;

/// One parsed source comment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CommentSpan {
    /// The exact source span of the comment text.
    pub span: Span,
    /// The exact comment text.
    pub text: String,
}

impl CommentSpan {
    /// Create one parsed source comment.
    pub fn new(span: Span, text: &str) -> Self {
        Self {
            span,
            text: text.to_string(),
        }
    }
}

/// One parsed typed value span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TypedValueSpan {
    /// The enclosing span of the typed value occurrence.
    pub span: Span,
    /// The optional name span inside the occurrence.
    pub name_span: Option<Span>,
    /// The type span inside the occurrence.
    pub type_span: Span,
}

impl TypedValueSpan {
    /// Create one parsed typed value span.
    pub const fn new(span: Span, name_span: Option<Span>, type_span: Span) -> Self {
        Self {
            span,
            name_span,
            type_span,
        }
    }
}

/// One parsed field declaration span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FieldSpan {
    /// The enclosing span of the field declaration.
    pub span: Span,
    /// The attribute spans on the field declaration.
    pub attribute_spans: Vec<Span>,
    /// The optional field name span.
    pub name_span: Option<Span>,
    /// The field type span.
    pub type_span: Span,
}

impl FieldSpan {
    /// Create one parsed field declaration span.
    pub fn new(
        span: Span,
        attribute_spans: Vec<Span>,
        name_span: Option<Span>,
        type_span: Span,
    ) -> Self {
        Self {
            span,
            attribute_spans,
            name_span,
            type_span,
        }
    }
}

/// Parsed function header delimiter spans.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionHeaderSpans {
    /// The opening parenthesis span.
    pub open_paren: Span,
    /// The closing parenthesis span.
    pub close_paren: Span,
    /// The return type colon span.
    pub return_colon: Span,
    /// The optional body opening brace span.
    pub open_brace: Option<Span>,
}

impl FunctionHeaderSpans {
    /// Create parsed function header delimiter spans.
    pub const fn new(
        open_paren: Span,
        close_paren: Span,
        return_colon: Span,
        open_brace: Option<Span>,
    ) -> Self {
        Self {
            open_paren,
            close_paren,
            return_colon,
            open_brace,
        }
    }
}

/// Parsed type declaration delimiter spans.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TypeDeclarationSpans {
    /// The optional equals span.
    pub equals: Option<Span>,
    /// The optional opening brace span.
    pub open_brace: Option<Span>,
    /// The optional closing brace span.
    pub close_brace: Option<Span>,
}

impl TypeDeclarationSpans {
    /// Create parsed type declaration delimiter spans.
    pub const fn new(
        equals: Option<Span>,
        open_brace: Option<Span>,
        close_brace: Option<Span>,
    ) -> Self {
        Self {
            equals,
            open_brace,
            close_brace,
        }
    }
}
