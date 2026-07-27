use destack_dir as dir;
use destack_source::{File, Span};

/// A marker token recognized in pattern source.
pub(super) struct Marker {
    /// The token span.
    pub(super) span: Span,
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
}

impl Marker {
    /// Parse an identifier token as a marker.
    pub(super) fn parse(file: &File, token: dir::TokenSpan) -> Result<Option<Self>, MarkerError> {
        if token.token.ty() != dir::TokenType::Identifier || token.token.is_identifier_escaped() {
            return Ok(None);
        }
        let text = file
            .get_span_str(token.span)
            .ok_or(MarkerError::InvalidSource { span: token.span })?;
        let (name, is_nodes) = match text.strip_prefix("$$$") {
            Some(name) => (name, true),
            None => match text.strip_prefix('$') {
                Some(name) => (name, false),
                None => return Ok(None),
            },
        };
        let Some(first) = name.as_bytes().first().copied() else {
            return Ok(None);
        };
        if first != b'_' && !first.is_ascii_uppercase() {
            return Ok(None);
        }
        if !name
            .bytes()
            .all(|byte| byte == b'_' || byte.is_ascii_uppercase() || byte.is_ascii_digit())
        {
            return Ok(None);
        }

        Ok(Some(Self {
            span: token.span,
            name: name.to_string(),
            is_nodes,
        }))
    }
}
