use crate::allocator::PageView;

/// The source bytes used to initialize one allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Payload<'a> {
    /// Caller-provided bytes.
    Bytes(&'a [u8]),
    /// Zeroed bytes.
    Zeroed,
    /// Bytes copied from an existing page view.
    PageView {
        /// The source logical page view.
        page_view: &'a PageView,
        /// The source byte offset.
        start: usize,
        /// The copied byte length.
        byte_len: usize,
    },
}

impl<'a> Payload<'a> {
    /// Return the explicit payload byte length when this payload has one.
    pub(crate) fn byte_len(&self) -> Option<usize> {
        match self {
            Self::Bytes(bytes) => Some(bytes.len()),
            Self::Zeroed => None,
            Self::PageView { byte_len, .. } => Some(*byte_len),
        }
    }
}
