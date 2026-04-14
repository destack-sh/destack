use super::image::SharedRegionImage;
use crate::alloc::PageMap;

/// One logical shared-memory region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SharedRegion {
    /// Whether this region id is currently allocated.
    pub(crate) is_allocated: bool,
    /// The logical byte length of this region.
    pub(crate) len: usize,
    /// The arena pages for this region.
    pub(crate) pages: PageMap,
}

impl SharedRegion {
    /// Create one live shared region from one frozen region root.
    pub(crate) fn from_image(image: &SharedRegionImage) -> Self {
        Self {
            is_allocated: image.is_allocated,
            len: image.len,
            pages: image.pages.clone(),
        }
    }
}
