use tspp_dir as dir;
use tspp_parser::PatternMarker;
use tspp_source::{File, Span};

/// A marker token recognized in pattern source.
pub(super) struct Marker {
    /// The exact marker span.
    pub(super) span: Span,
    /// The complete token occupied by the marker.
    pub(super) token_span: Span,
    /// The name without dollar signs.
    pub(super) name: String,
    /// Whether the marker captures repeated nodes.
    pub(super) is_nodes: bool,
}

/// A marker lowering failure before its source role is known.
pub(super) enum MarkerError {
    /// A parser-owned token span is outside its source.
    InvalidSource {
        /// The invalid token span.
        span: Span,
    },
    /// A marker does not occupy a capturable DIR position.
    Invalid {
        /// The invalid marker span.
        span: Span,
    },
    /// A repeated marker does not occupy a repeated DIR list.
    InvalidRepeated {
        /// The invalid repeated marker span.
        span: Span,
    },
    /// A repeated placeholder element has no source span.
    MissingNodeSpan {
        /// The repeated marker span.
        span: Span,
    },
}

impl Marker {
    /// Parse an identifier token as a marker.
    pub(super) fn parse(file: &File, token: dir::TokenSpan) -> Result<Option<Self>, MarkerError> {
        let is_identifier =
            token.token.ty() == dir::TokenType::Identifier && !token.token.is_identifier_escaped();
        let is_tree_text = token.token.literal() == Some(dir::TokenLiteral::TreeString);
        if !is_identifier && !is_tree_text {
            return Ok(None);
        }
        let text = file
            .get_span_str(token.span)
            .ok_or(MarkerError::InvalidSource { span: token.span })?;
        let (text, span) = if is_tree_text {
            let leading = text.len() - text.trim_start().len();
            let text = text.trim();
            let start = token.span.start + leading as u32;
            let span = Span::new(token.span.file, start, start + text.len() as u32);

            (text, span)
        } else {
            (text, token.span)
        };
        let Some(marker) = PatternMarker::parse(text) else {
            return Ok(None);
        };
        let is_nodes = matches!(marker, PatternMarker::Nodes { .. });

        Ok(Some(Self {
            span,
            token_span: token.span,
            name: marker.name().to_string(),
            is_nodes,
        }))
    }
}
