use tspp_source::Span;

/// Trivia (comments, whitespace) in JSONC.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonTrivia {
    /// A line comment starting with `//`.
    LineComment {
        /// The comment content (without the `//` prefix).
        content: String,
        /// The span of the entire comment.
        span: Span,
    },
    /// A block comment enclosed in `/* */`.
    BlockComment {
        /// The comment content (without the delimiters).
        content: String,
        /// The span of the entire comment.
        span: Span,
    },
    /// Whitespace (spaces, tabs).
    Whitespace {
        /// The span of the whitespace.
        span: Span,
    },
    /// A newline character.
    Newline {
        /// The span of the newline.
        span: Span,
    },
}

impl JsonTrivia {
    /// Get the span of this trivia.
    pub fn span(&self) -> Span {
        match self {
            JsonTrivia::LineComment { span, .. } => *span,
            JsonTrivia::BlockComment { span, .. } => *span,
            JsonTrivia::Whitespace { span } => *span,
            JsonTrivia::Newline { span } => *span,
        }
    }

    /// Check if this trivia is a comment.
    pub fn is_comment(&self) -> bool {
        matches!(
            self,
            JsonTrivia::LineComment { .. } | JsonTrivia::BlockComment { .. }
        )
    }
}
