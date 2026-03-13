use super::image::PageImage;
use super::live::Page;
use crate::value::Value;
use crate::{Bitmap, RawAllocation};

/// The number of entries in one heap page.
///
/// A single shared page width keeps page classes aligned and lets stable
/// page-slot coordinates stay compact across managed, value, and raw storage.
/// This is a structural representation choice, not a runtime tuning option.
pub const PAGE_CAPACITY: usize = 1024;

/// The number of managed allocations in one managed page.
pub const MANAGED_PAGE_CAPACITY: usize = PAGE_CAPACITY;

/// The number of values in one value page.
pub const VALUE_PAGE_CAPACITY: usize = PAGE_CAPACITY;

/// The number of raw allocations in one raw page.
pub const RAW_PAGE_CAPACITY: usize = PAGE_CAPACITY;

/// The number of bitmap words for one managed page.
pub const MANAGED_PAGE_BITMAP_WORDS: usize = MANAGED_PAGE_CAPACITY / (u64::BITS as usize);

/// The number of bitmap words for one value page.
pub const VALUE_PAGE_BITMAP_WORDS: usize = VALUE_PAGE_CAPACITY / (u64::BITS as usize);

/// The number of bitmap words for one raw page.
pub const RAW_PAGE_BITMAP_WORDS: usize = RAW_PAGE_CAPACITY / (u64::BITS as usize);

/// The bitmap type used by page classes.
pub(crate) type PageBitmap<const CAPACITY: usize, const WORDS: usize> = Bitmap<CAPACITY, WORDS>;

/// One immutable value page image.
pub type ValuePageImage = PageImage<Value, VALUE_PAGE_CAPACITY, VALUE_PAGE_BITMAP_WORDS>;

/// One immutable raw page image.
pub type RawPageImage = PageImage<RawAllocation, RAW_PAGE_CAPACITY, RAW_PAGE_BITMAP_WORDS>;

/// One live value page.
pub(crate) type ValuePage = Page<Value, VALUE_PAGE_CAPACITY, VALUE_PAGE_BITMAP_WORDS>;

/// One live raw page.
pub(crate) type RawPage = Page<RawAllocation, RAW_PAGE_CAPACITY, RAW_PAGE_BITMAP_WORDS>;
