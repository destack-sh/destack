//! Spans and Multi-Spans into SourceFiles.

use crate::SourceId;

/// A source range in bytes (in some SourceFile).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// The file that the Span belongs to.
    pub file: SourceId,
    /// The start position of the Span in bytes (absolute, inclusive).
    pub start: u32,
    /// The end position of the Span in bytes (absolute, exclusive).
    pub end: u32,
}

impl Span {
    /// Create a new Span.
    pub fn new(file: SourceId, start: u32, end: u32) -> Self {
        Self { file, start, end }
    }

    /// Create an empty Span.
    pub fn empty(file: SourceId) -> Self {
        Self {
            file,
            start: 0,
            end: 0,
        }
    }

    /// Merge two Spans.
    /// The resulting Span will be the smallest Span that contains both.
    pub fn merge(self, other: Self) -> Self {
        debug_assert_eq!(
            self.file, other.file,
            "span {self:?} and {other:?} are in different files"
        );
        Self {
            file: self.file,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Check if the Span is empty.
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// A MultiSpan is a collection of Spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiSpan {
    /// The Spans.
    pub spans: Vec<Span>,
}

impl MultiSpan {
    /// Create a new MultiSpan.
    pub fn new(spans: Vec<Span>) -> Self {
        Self { spans }
    }
}
