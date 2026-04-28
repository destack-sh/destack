use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::FileId;

/// A source range in bytes (in some File).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    /// The file that the Span belongs to.
    pub file: FileId,
    /// The start position of the Span in bytes (absolute, inclusive).
    pub start: u32,
    /// The end position of the Span in bytes (absolute, exclusive).
    pub end: u32,
}

impl Span {
    /// Create a new Span.
    pub fn new(file: FileId, start: u32, end: u32) -> Self {
        Self { file, start, end }
    }

    /// Create a new Span from a position and length.
    pub fn at(file: FileId, start: u32, length: u32) -> Self {
        Self {
            file,
            start,
            end: start + length,
        }
    }

    /// Create an empty Span.
    pub fn empty(file: FileId) -> Self {
        Self {
            file,
            start: 0,
            end: 0,
        }
    }

    /// Return this span with a different file id.
    #[inline]
    pub fn with_file(self, file: FileId) -> Self {
        Self {
            file,
            start: self.start,
            end: self.end,
        }
    }

    /// Merge two Spans.
    /// The resulting Span will be the smallest Span that contains both.
    pub fn merge(self, other: Self) -> Self {
        debug_assert_eq!(
            self.file, other.file,
            "span {self:?} and {other:?} are from different files"
        );
        Self {
            file: self.file,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Enlarge the Span to include the given position.
    pub fn extend(self, position: u32) -> Self {
        Self {
            file: self.file,
            start: self.start.min(position),
            end: self.end.max(position),
        }
    }

    /// Check if the Span is empty.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Compute the length of the Span in bytes.
    #[inline]
    pub fn len(self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    /// Check whether the Span contains the given absolute byte position.
    #[inline]
    pub fn contains(self, position: u32) -> bool {
        position >= self.start && position < self.end
    }

    /// Check whether the two spans overlap (on the same file).
    #[inline]
    pub fn intersects(self, other: Self) -> bool {
        if self.file != other.file {
            return false;
        }
        self.start < other.end && other.start < self.end
    }

    /// Compute the intersection of two spans (on the same file).
    pub fn intersection(self, other: Self) -> Option<Self> {
        if self.file != other.file {
            return None;
        }
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        if start < end {
            Some(Self {
                file: self.file,
                start,
                end,
            })
        } else {
            None
        }
    }

    /// Create a subspan from byte offsets relative to another span.
    pub fn subspan(self, range: Range<usize>) -> Self {
        debug_assert!(range.start <= range.end);
        debug_assert!(range.end <= self.len() as usize);

        Self {
            file: self.file,
            start: self.start + range.start as u32,
            end: self.start + range.end as u32,
        }
    }

    /// Compute the ordered trivia gap from this span to another span.
    /// Returns `None` when spans are from different files or overlap.
    /// Returns an empty span when the ranges are adjacent.
    pub fn gap_to(self, other: Self) -> Option<Self> {
        if self.file != other.file || self.end > other.start {
            return None;
        }

        Some(Self {
            file: self.file,
            start: self.end,
            end: other.start,
        })
    }
}

/// A MultiSpan is a collection of Spans, sorted for fast containment queries.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MultiSpan {
    /// The Spans, sorted by start position for binary search.
    pub spans: Vec<Span>,
}

impl MultiSpan {
    /// Create a new MultiSpan, sorting spans by start position.
    pub fn new(mut spans: Vec<Span>) -> Self {
        spans.sort_unstable_by_key(|s| s.start);
        Self { spans }
    }

    /// Check if the MultiSpan contains the given Span fully (start and end).
    /// Uses binary search for O(log n) lookup instead of O(n).
    #[inline]
    pub fn contains(&self, span: &Span) -> bool {
        if self.spans.is_empty() {
            return false;
        }

        // binary search for the rightmost span where span.start <= query.start
        let idx = self.spans.partition_point(|s| s.start <= span.start);
        if idx == 0 {
            return false;
        }

        // check if the span at idx-1 contains our query span
        let candidate = &self.spans[idx - 1];
        candidate.contains(span.start) && candidate.contains(span.end - 1)
    }
}

/// A Span with a message.
#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub struct LabeledSpan {
    /// The span of the labeled span.
    pub span: Span,
    /// The message of the labeled span.
    pub label: String,
}

impl LabeledSpan {
    /// Create a new LabeledSpan.
    pub fn new(span: Span, label: impl Into<String>) -> Self {
        Self {
            span,
            label: label.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FileId, Span};

    /// Return the span gap for ordered non-overlapping spans on the same file.
    #[test]
    fn test_gap_to_returns_gap_for_ordered_spans() {
        let left = Span::new(FileId::new(1), 10, 20);
        let right = Span::new(FileId::new(1), 30, 40);

        let gap = left.gap_to(right);

        assert_eq!(gap, Some(Span::new(FileId::new(1), 20, 30)));
    }

    /// Return an empty span gap for adjacent spans.
    #[test]
    fn test_gap_to_returns_empty_span_for_adjacent_spans() {
        let left = Span::new(FileId::new(1), 10, 20);
        let right = Span::new(FileId::new(1), 20, 40);

        let gap = left.gap_to(right);

        assert_eq!(gap, Some(Span::new(FileId::new(1), 20, 20)));
    }

    /// Return no gap when spans overlap or are from different files.
    #[test]
    fn test_gap_to_returns_none_for_overlapping_or_cross_file_spans() {
        let overlapping_left = Span::new(FileId::new(1), 10, 25);
        let overlapping_right = Span::new(FileId::new(1), 20, 40);
        let cross_file_right = Span::new(FileId::new(2), 30, 40);

        let overlap_gap = overlapping_left.gap_to(overlapping_right);
        let cross_file_gap = overlapping_left.gap_to(cross_file_right);

        assert_eq!(overlap_gap, None);
        assert_eq!(cross_file_gap, None);
    }

    /// Return a subspan from byte offsets relative to a parent span.
    #[test]
    fn test_subspan_returns_relative_span() {
        let parent = Span::new(FileId::new(1), 10, 30);

        let child = parent.subspan(3..12);

        assert_eq!(child, Span::new(FileId::new(1), 13, 22));
    }
}
