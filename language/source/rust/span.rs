//! Spans and Multi-Spans into SourceFiles.

use crate::SourceId;

/// A source range in bytes (in some SourceFile).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// The file that the Span belongs to.
    pub source: SourceId,
    /// The start position of the Span in bytes (absolute, inclusive).
    pub start: u32,
    /// The end position of the Span in bytes (absolute, exclusive).
    pub end: u32,
}

impl Span {
    /// Create a new Span.
    pub fn new(source: SourceId, start: u32, end: u32) -> Self {
        Self { source, start, end }
    }

    /// Create an empty Span.
    pub fn empty(source: SourceId) -> Self {
        Self {
            source,
            start: 0,
            end: 0,
        }
    }

    /// Merge two Spans.
    /// The resulting Span will be the smallest Span that contains both.
    pub fn merge(self, other: Self) -> Self {
        debug_assert_eq!(
            self.source, other.source,
            "span {self:?} and {other:?} are from different sources"
        );
        Self {
            source: self.source,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Check if the Span is empty.
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Compute the length of the Span in bytes.
    pub fn len(self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    /// Check whether the Span contains the given absolute byte position.
    pub fn contains(self, position: u32) -> bool {
        position >= self.start && position < self.end
    }

    /// Check whether the two spans overlap (on the same source).
    pub fn intersects(self, other: Self) -> bool {
        if self.source != other.source {
            return false;
        }
        self.start < other.end && other.start < self.end
    }

    /// Compute the intersection of two spans (on the same source).
    pub fn intersection(self, other: Self) -> Option<Self> {
        if self.source != other.source {
            return None;
        }
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        if start < end {
            Some(Self {
                source: self.source,
                start,
                end,
            })
        } else {
            None
        }
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

/// A Span with a message.
#[derive(Debug, Clone)]
pub struct LabeledSpan {
    /// The span of the labeled span.
    pub span: Span,
    /// The message of the labeled span.
    pub label: String,
}
