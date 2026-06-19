use destack_source as source;

use crate::{FileId, SourceIdParseError, bridge};

/// Source byte span crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    /// File containing this span.
    pub file: FileId,
    /// Inclusive start byte offset.
    pub start: u32,
    /// Exclusive end byte offset.
    pub end: u32,
}

impl Span {
    /// Convert one source span into one bridge span.
    pub fn from_source(span: source::Span) -> Self {
        Self {
            file: span.file.into(),
            start: span.start,
            end: span.end,
        }
    }

    /// Convert this bridge span into one source span.
    pub fn into_source(self) -> Result<source::Span, SourceIdParseError> {
        Ok(source::Span::new(
            self.file.into_source()?,
            self.start,
            self.end,
        ))
    }
}

impl From<source::Span> for Span {
    /// Convert one source span into one bridge span.
    fn from(span: source::Span) -> Self {
        Self::from_source(span)
    }
}

impl TryFrom<Span> for source::Span {
    type Error = SourceIdParseError;

    /// Convert one bridge span into one source span.
    fn try_from(span: Span) -> Result<Self, Self::Error> {
        span.into_source()
    }
}
