use std::slice;
use std::sync::Arc;

/// Ordered readable segments for one DIR table.
#[derive(Debug, Clone)]
pub struct SegmentView<'a, T> {
    /// The committed segment storage.
    base: SegmentBase<'a, T>,
    /// The active phase segment appended to the committed base.
    tail: Option<&'a T>,
}

impl<T> Default for SegmentView<'_, T> {
    fn default() -> Self {
        Self {
            base: SegmentBase::Owned(Vec::new()),
            tail: None,
        }
    }
}

impl<T> SegmentView<'static, T> {
    /// Create a segment view from committed artifact segments.
    pub fn from_segments(segments: Vec<Arc<T>>) -> Self {
        Self {
            base: SegmentBase::Owned(segments),
            tail: None,
        }
    }
}

impl<'a, T> SegmentView<'a, T> {
    /// Create a segment view by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b T) -> SegmentView<'b, T> {
        assert!(
            self.tail.is_none(),
            "segment view already has an active tail"
        );

        SegmentView {
            base: SegmentBase::Borrowed(self.base.as_slice()),
            tail: Some(tail),
        }
    }

    /// Iterate segments in phase order.
    pub fn iter(&self) -> SegmentIter<'_, T> {
        SegmentIter {
            base: self.base.as_slice().iter(),
            tail: self.tail,
        }
    }

    /// Return the first visible segment.
    pub fn first(&self) -> Option<&T> {
        self.base.first().or(self.tail)
    }

    /// Return the last visible segment.
    pub fn last(&self) -> Option<&T> {
        self.tail.or_else(|| self.base.last())
    }
}

/// Segment view storage.
#[derive(Debug, Clone)]
enum SegmentBase<'a, T> {
    /// Owned committed artifact segments.
    Owned(Vec<Arc<T>>),
    /// Borrowed committed artifact segments.
    Borrowed(&'a [Arc<T>]),
}

impl<'a, T> SegmentBase<'a, T> {
    /// Return committed segments as a slice.
    fn as_slice(&self) -> &[Arc<T>] {
        match self {
            Self::Owned(segments) => segments,
            Self::Borrowed(segments) => segments,
        }
    }

    /// Return the first committed segment.
    fn first(&self) -> Option<&T> {
        self.as_slice().first().map(Arc::as_ref)
    }

    /// Return the last committed segment.
    fn last(&self) -> Option<&T> {
        self.as_slice().last().map(Arc::as_ref)
    }
}

/// Iterator over visible table segments.
#[derive(Debug)]
pub struct SegmentIter<'a, T> {
    /// The committed segment iterator.
    base: slice::Iter<'a, Arc<T>>,
    /// The pending active phase segment.
    tail: Option<&'a T>,
}

impl<'a, T> Iterator for SegmentIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(segment) = self.base.next() {
            return Some(segment.as_ref());
        }

        self.tail.take()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();

        (len, Some(len))
    }
}

impl<T> DoubleEndedIterator for SegmentIter<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some(tail) = self.tail.take() {
            return Some(tail);
        }

        self.base.next_back().map(Arc::as_ref)
    }
}

impl<T> ExactSizeIterator for SegmentIter<'_, T> {
    fn len(&self) -> usize {
        self.base.len() + usize::from(self.tail.is_some())
    }
}
