use crate::HeapResult;
use crate::allocator::{Allocator, PageView};

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

    /// Initialize this payload into one allocated page view.
    pub(crate) fn initialize(
        &self,
        allocator: &Allocator,
        pages: &mut PageView,
        byte_offset: usize,
    ) -> HeapResult<()> {
        match self {
            Self::Bytes(bytes) => allocator.set_bytes(pages, byte_offset, bytes),
            Self::Zeroed => Ok(()),
            Self::PageView {
                page_view,
                start,
                byte_len,
            } => allocator.copy_bytes_between_page_views(
                page_view,
                *start,
                pages,
                byte_offset,
                *byte_len,
            ),
        }
    }
}
