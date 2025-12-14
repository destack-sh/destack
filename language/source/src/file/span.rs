use crate::FileId;

/// A source range in bytes (in some File).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
}

/// A MultiSpan is a collection of Spans.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MultiSpan {
    /// The Spans.
    pub spans: Vec<Span>,
}

impl MultiSpan {
    /// Create a new MultiSpan.
    pub fn new(spans: Vec<Span>) -> Self {
        Self { spans }
    }

    /// Check if the MultiSpan contains the given Span fully (start and end).
    pub fn contains(&self, span: &Span) -> bool {
        self.spans
            .iter()
            .any(|s| s.contains(span.start) && s.contains(span.end - 1))
    }
}

/// A Span with a message.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct LabeledSpan {
    /// The span of the labeled span.
    pub span: Span,
    /// The message of the labeled span.
    pub label: String,
}

impl LabeledSpan {
    /// Create a new LabeledSpan.
    pub fn new(span: Span, label: String) -> Self {
        Self { span, label }
    }
}
