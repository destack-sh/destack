use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::{fmt, mem, slice};

pub use destack_serde::SectionEntry;
use destack_serde::{Reflect, SchemaRef, SchemaRegistry};
use serde::{Deserialize, Serialize};

use crate::StringId;

const SECTION_CHUNK_BYTES: usize = mem::size_of::<u128>();
const SECTION_ALIGNMENT_BYTES: usize = mem::align_of::<u128>();

/// Invalid section image shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionImageError {
    /// Image bytes do not satisfy section alignment.
    Misaligned,
    /// Image bytes do not fit in the backing chunk storage.
    Truncated {
        /// Required initialized bytes.
        required: u64,
        /// Available initialized bytes.
        available: u64,
    },
}

impl fmt::Display for SectionImageError {
    /// Format one section image error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Misaligned => formatter.write_str("misaligned section image"),
            Self::Truncated {
                required,
                available,
            } => write!(
                formatter,
                "truncated section image: required {required} bytes, available {available}"
            ),
        }
    }
}

impl std::error::Error for SectionImageError {}

/// Typed entry slice stored in one image.
#[repr(C)]
#[derive(Debug, Serialize, Deserialize, Reflect)]
pub struct SectionSlice<T> {
    /// Byte offset from the image base.
    pub byte_offset: u64,
    /// Number of typed entries in the slice.
    pub len: u32,
    /// Reserved image word.
    reserved: u32,
    /// Typed entry marker.
    #[serde(skip)]
    marker: PhantomData<fn() -> T>,
}

impl<T> Clone for SectionSlice<T> {
    /// Clone this typed section slice.
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for SectionSlice<T> {}

impl<T> PartialEq for SectionSlice<T> {
    /// Compare image offsets and entry counts.
    fn eq(&self, other: &Self) -> bool {
        self.byte_offset == other.byte_offset
            && self.len == other.len
            && self.reserved == other.reserved
    }
}

impl<T> Eq for SectionSlice<T> {}

impl<T> Default for SectionSlice<T> {
    /// Create an empty typed section slice.
    fn default() -> Self {
        Self::empty()
    }
}

impl<T> SectionSlice<T> {
    /// Create one section-relative typed slice.
    #[inline]
    pub const fn new(byte_offset: u64, len: u32) -> Self {
        Self {
            byte_offset,
            len,
            reserved: 0,
            marker: PhantomData,
        }
    }

    /// Create an empty typed section slice.
    #[inline]
    pub const fn empty() -> Self {
        Self::new(0, 0)
    }

    /// Return the number of typed entries.
    #[inline]
    pub const fn len(self) -> usize {
        self.len as usize
    }

    /// Return whether this typed slice has no entries.
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }
}

/// Typed contiguous entry range inside one sibling entry slice.
#[repr(C)]
#[derive(Debug, Serialize, Deserialize, Reflect)]
pub struct EntryRange<T> {
    /// First entry in the sibling entry slice.
    pub start: u32,
    /// Number of entries in the range.
    pub len: u32,
    /// Typed entry marker.
    #[serde(skip)]
    marker: PhantomData<fn() -> T>,
}

impl<T> Clone for EntryRange<T> {
    /// Clone this typed entry range.
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for EntryRange<T> {}

impl<T> PartialEq for EntryRange<T> {
    /// Compare entry starts and counts.
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.len == other.len
    }
}

impl<T> Eq for EntryRange<T> {}

impl<T> Default for EntryRange<T> {
    /// Create an empty typed entry range.
    fn default() -> Self {
        Self::empty()
    }
}

impl<T> EntryRange<T> {
    /// Create one typed entry range.
    #[inline]
    pub const fn new(start: u32, len: u32) -> Self {
        Self {
            start,
            len,
            marker: PhantomData,
        }
    }

    /// Create an empty typed entry range.
    #[inline]
    pub const fn empty() -> Self {
        Self::new(0, 0)
    }

    /// Return the exclusive end entry.
    #[inline]
    pub const fn end(self) -> u32 {
        self.start + self.len
    }

    /// Return the number of entries.
    #[inline]
    pub const fn len(self) -> usize {
        self.len as usize
    }

    /// Return whether this range has no entries.
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// Borrow this entry range from one entry slice.
    #[inline]
    pub fn slice<'a>(&self, entries: &'a [T]) -> &'a [T] {
        let start = self.start as usize;
        let end = self.end() as usize;

        &entries[start..end]
    }
}

/// Append-only build-time entry storage.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct EntryStore<T> {
    /// Stored entries.
    entries: Vec<T>,
}

impl<T> Default for EntryStore<T> {
    /// Create an empty entry store.
    fn default() -> Self {
        Self::new()
    }
}

impl<T> EntryStore<T> {
    /// Create an empty entry store.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Append entries and return their typed entry range.
    pub fn append(&mut self, entries: impl IntoIterator<Item = T>) -> EntryRange<T> {
        let start = self.entries.len() as u32;
        self.entries.extend(entries);
        let len = self.entries.len() as u32 - start;

        EntryRange::new(start, len)
    }

    /// Borrow one typed entry range.
    pub fn get(&self, range: EntryRange<T>) -> &[T] {
        range.slice(&self.entries)
    }

    /// Consume this store into its entries.
    pub fn into_entries(self) -> Vec<T> {
        self.entries
    }

    /// Return all entries.
    pub fn entries(&self) -> &[T] {
        &self.entries
    }

    /// Return the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return whether this table has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// One immutable entry type that can be stored directly in a typed section.
///
/// Implementors must have stable fixed-width layout and no process-local ownership.
/// Their alignment must not exceed the section image alignment.
///
/// # Safety
///
/// Implementors must remain valid when copied to and from a trusted section image without running
/// constructors, destructors, pointer relocation, or validity repair.
pub unsafe trait SectionEntry: Copy + 'static {}

// SAFETY: primitive integers and string ids are fixed-width entry scalars.
unsafe impl SectionEntry for u8 {}
unsafe impl SectionEntry for u16 {}
unsafe impl SectionEntry for u32 {}
unsafe impl SectionEntry for u64 {}
unsafe impl SectionEntry for u128 {}
unsafe impl SectionEntry for i8 {}
unsafe impl SectionEntry for i16 {}
unsafe impl SectionEntry for i32 {}
unsafe impl SectionEntry for i64 {}
unsafe impl SectionEntry for i128 {}
unsafe impl SectionEntry for f32 {}
unsafe impl SectionEntry for f64 {}
unsafe impl SectionEntry for StringId {}
unsafe impl SectionEntry for NonZeroU32 {}

// SAFETY: fixed arrays preserve their entry layout and contain no additional state.
unsafe impl<T: SectionEntry, const N: usize> SectionEntry for [T; N] {}

// SAFETY: EntryRange<T> stores only section offsets and lengths.
unsafe impl<T: SectionEntry> SectionEntry for EntryRange<T> {}

// SAFETY: SectionSlice<T> stores only an image offset and entry count.
unsafe impl<T: SectionEntry> SectionEntry for SectionSlice<T> {}

/// Stable optional entry value.
#[repr(C)]
pub struct Optional<T> {
    /// Whether value is present.
    pub is_some: u32,
    /// Stored value when present.
    value: mem::MaybeUninit<T>,
}

impl<T: SectionEntry> Copy for Optional<T> {}

impl<T: SectionEntry> Clone for Optional<T> {
    /// Clone this optional entry.
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: SectionEntry> Default for Optional<T> {
    /// Create an empty optional entry value.
    fn default() -> Self {
        Self::none()
    }
}

impl<T: SectionEntry> Optional<T> {
    /// Create an empty optional entry value.
    pub const fn none() -> Self {
        let value = mem::MaybeUninit::zeroed();

        Self { is_some: 0, value }
    }

    /// Create a present optional entry value.
    pub const fn some(value: T) -> Self {
        Self {
            is_some: 1,
            value: mem::MaybeUninit::new(value),
        }
    }

    /// Convert into a Rust option.
    pub const fn get(self) -> Option<T> {
        if self.is_some == 0 {
            None
        } else {
            // SAFETY: present Optional entries are constructed with an initialized value.
            Some(unsafe { self.value.assume_init() })
        }
    }

    /// Borrow the present optional entry value.
    pub fn as_ref(&self) -> Option<&T> {
        if self.is_some == 0 {
            None
        } else {
            // SAFETY: present Optional entries are constructed with an initialized value.
            Some(unsafe { self.value.assume_init_ref() })
        }
    }
}

impl<T> fmt::Debug for Optional<T>
where
    T: fmt::Debug + SectionEntry,
{
    /// Format this optional entry value.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_ref() {
            Some(value) => formatter.debug_tuple("Some").field(value).finish(),
            None => formatter.write_str("None"),
        }
    }
}

impl<T> PartialEq for Optional<T>
where
    T: PartialEq + SectionEntry,
{
    /// Compare optional entry values.
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl<T> Eq for Optional<T> where T: Eq + SectionEntry {}

impl<T> std::hash::Hash for Optional<T>
where
    T: std::hash::Hash + SectionEntry,
{
    /// Hash this optional entry value.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

impl<T> Serialize for Optional<T>
where
    T: Serialize + SectionEntry,
{
    /// Serialize this optional entry as a regular optional value.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_ref().serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for Optional<T>
where
    T: Deserialize<'de> + SectionEntry,
{
    /// Deserialize this optional entry from a regular optional value.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Option::<T>::deserialize(deserializer)?;

        Ok(Self::from(value))
    }
}

impl<T> Reflect for Optional<T>
where
    T: Reflect + SectionEntry,
{
    /// Reflect this optional entry as a regular optional value.
    fn reflect(registry: &mut SchemaRegistry) -> SchemaRef {
        Option::<T>::reflect(registry)
    }
}

impl<T: SectionEntry> From<Option<T>> for Optional<T> {
    /// Convert one Rust option into a stable optional entry value.
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => Self::some(value),
            None => Self::none(),
        }
    }
}

// SAFETY: Optional<T> is repr(C), Copy, and stores one initialized value only when present.
unsafe impl<T: SectionEntry> SectionEntry for Optional<T> {}

/// Shared immutable memory containing one section image.
pub trait SectionMemory: AsRef<[u8]> + fmt::Debug + Send + Sync {}

impl<T> SectionMemory for T where T: AsRef<[u8]> + fmt::Debug + Send + Sync {}

/// Immutable aligned section storage.
#[derive(Debug, Clone)]
pub enum SectionStorage {
    /// Owned aligned image chunks and initialized byte length.
    Owned {
        /// Aligned image chunks.
        chunks: Vec<u128>,
        /// Number of initialized bytes.
        byte_len: u64,
    },
    /// Prelinked static image chunks and initialized byte length.
    Static {
        /// Aligned image chunks.
        chunks: &'static [u128],
        /// Number of initialized bytes.
        byte_len: u64,
    },
    /// Shared immutable image memory.
    Shared(Arc<dyn SectionMemory>),
}

impl Serialize for SectionStorage {
    /// Serialize section storage as its logical aligned chunks.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.bytes().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SectionStorage {
    /// Deserialize section storage into owned aligned chunks.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;

        Ok(Self::from_bytes(&bytes))
    }
}

impl Reflect for SectionStorage {
    /// Reflect section storage as its logical aligned chunks.
    fn reflect(registry: &mut SchemaRegistry) -> SchemaRef {
        Vec::<u8>::reflect(registry)
    }
}

impl SectionStorage {
    /// Create prelinked static section storage.
    pub fn from_static(chunks: &'static [u128], byte_len: u64) -> Result<Self, SectionImageError> {
        Self::check_len(chunks, byte_len)?;

        Ok(Self::Static { chunks, byte_len })
    }

    /// Retain shared immutable section memory without copying it.
    pub fn from_shared(memory: Arc<dyn SectionMemory>) -> Result<Self, SectionImageError> {
        let bytes = memory.as_ref().as_ref();
        if !(bytes.as_ptr() as usize).is_multiple_of(SECTION_ALIGNMENT_BYTES) {
            return Err(SectionImageError::Misaligned);
        }

        Ok(Self::Shared(memory))
    }

    /// Copy raw bytes into owned aligned section storage.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let byte_len = bytes.len();
        let chunk_len = byte_len.div_ceil(SECTION_CHUNK_BYTES);
        let mut chunks = vec![0; chunk_len];
        let destination = chunks.as_mut_ptr().cast::<u8>();

        // SAFETY: chunks has enough initialized byte storage for byte_len bytes.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), destination, byte_len);
        }

        Self::Owned {
            chunks,
            byte_len: byte_len as u64,
        }
    }

    /// Return initialized image bytes.
    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::Owned { chunks, byte_len } => {
                let bytes = chunks.as_ptr().cast::<u8>();

                // SAFETY: constructors require byte_len to fit in the backing chunks.
                unsafe { slice::from_raw_parts(bytes, *byte_len as usize) }
            }
            Self::Static { chunks, byte_len } => {
                let bytes = chunks.as_ptr().cast::<u8>();

                // SAFETY: constructors require byte_len to fit in the backing chunks.
                unsafe { slice::from_raw_parts(bytes, *byte_len as usize) }
            }
            Self::Shared(memory) => memory.as_ref().as_ref(),
        }
    }

    /// Return the number of initialized bytes.
    pub fn byte_len(&self) -> u64 {
        match self {
            Self::Owned { byte_len, .. } | Self::Static { byte_len, .. } => *byte_len,
            Self::Shared(memory) => memory.as_ref().as_ref().len() as u64,
        }
    }

    /// Require the initialized byte length to fit in the backing chunks.
    fn check_len(chunks: &[u128], byte_len: u64) -> Result<(), SectionImageError> {
        let available = (chunks.len() * SECTION_CHUNK_BYTES) as u64;
        if byte_len > available {
            return Err(SectionImageError::Truncated {
                required: byte_len,
                available,
            });
        }

        Ok(())
    }
}

/// Mutable writer that packs typed sections into one section image.
#[derive(Debug, Clone)]
pub struct SectionBuilder {
    /// Mutable aligned image storage being packed.
    storage: Vec<u128>,
    /// Number of initialized image bytes.
    byte_len: usize,
}

/// Borrowed read-only section view.
#[derive(Clone, Copy, Debug)]
pub struct SectionImage<'a> {
    /// Exact immutable image bytes.
    bytes: &'a [u8],
}

impl Default for SectionBuilder {
    /// Create an empty section builder.
    fn default() -> Self {
        Self::new()
    }
}

impl SectionBuilder {
    /// Create an empty section builder.
    pub fn new() -> Self {
        Self {
            storage: Vec::new(),
            byte_len: 0,
        }
    }

    /// Borrow this builder as a read-only section image.
    pub fn view(&self) -> SectionImage<'_> {
        // SAFETY: SectionBuilder creates offsets and storage together through insert.
        unsafe { SectionImage::from_chunks_unchecked(&self.storage, self.byte_len) }
    }

    /// Build immutable aligned section storage.
    pub fn build(self) -> SectionStorage {
        SectionStorage::Owned {
            chunks: self.storage,
            byte_len: self.byte_len as u64,
        }
    }

    /// Insert one typed entry section.
    pub fn insert<T: SectionEntry>(&mut self, entries: impl AsRef<[T]>) -> SectionSlice<T> {
        assert_ne!(
            mem::size_of::<T>(),
            0,
            "section entries must not be zero-sized"
        );
        assert!(
            mem::align_of::<T>() <= SECTION_ALIGNMENT_BYTES,
            "section entry alignment exceeds image alignment"
        );

        let entries = entries.as_ref();
        let byte_len = mem::size_of_val(entries);
        let alignment = mem::align_of::<T>().max(1);
        let byte_offset = align_usize(self.byte_len, alignment);

        // copy typed entries into the aligned section image
        let section_end = byte_offset + byte_len;
        let chunk_len = section_end.div_ceil(SECTION_CHUNK_BYTES);
        self.storage.resize(chunk_len, 0);

        let source = entries.as_ptr().cast::<u8>();

        // SAFETY: storage was resized to contain section_end bytes above.
        let destination = unsafe { self.storage.as_mut_ptr().cast::<u8>().add(byte_offset) };

        // SAFETY: source points to byte_len initialized entry bytes and destination
        // points to distinct table storage with enough initialized capacity.
        unsafe {
            std::ptr::copy_nonoverlapping(source, destination, byte_len);
        }

        self.byte_len = section_end;

        SectionSlice::new(byte_offset as u64, entries.len() as u32)
    }

    /// Insert raw bytes at one explicit power-of-two alignment.
    pub fn insert_bytes(&mut self, bytes: impl AsRef<[u8]>, alignment: usize) -> SectionSlice<u8> {
        assert!(
            alignment.is_power_of_two(),
            "section alignment must be a power of two"
        );
        assert!(
            alignment <= SECTION_ALIGNMENT_BYTES,
            "section byte alignment exceeds image alignment"
        );

        let bytes = bytes.as_ref();
        let byte_offset = align_usize(self.byte_len, alignment);
        let byte_end = byte_offset + bytes.len();
        let chunk_len = byte_end.div_ceil(SECTION_CHUNK_BYTES);
        self.storage.resize(chunk_len, 0);

        // copy bytes into the explicitly aligned image range
        let destination = unsafe { self.storage.as_mut_ptr().cast::<u8>().add(byte_offset) };

        // SAFETY: storage was resized to contain byte_end bytes above.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), destination, bytes.len());
        }

        self.byte_len = byte_end;

        SectionSlice::new(byte_offset as u64, bytes.len() as u32)
    }

    /// Replace one previously inserted section with the same number of entries.
    pub fn replace<T: SectionEntry>(&mut self, section: SectionSlice<T>, entries: impl AsRef<[T]>) {
        let entries = entries.as_ref();
        assert_eq!(
            section.len(),
            entries.len(),
            "replacement section length must remain unchanged"
        );

        let byte_offset = section.byte_offset as usize;
        let byte_len = mem::size_of_val(entries);
        let byte_end = byte_offset + byte_len;
        assert!(
            byte_end <= self.byte_len,
            "replacement section exceeds image bytes"
        );

        // overwrite the complete existing section
        let source = entries.as_ptr().cast::<u8>();
        let destination = unsafe { self.storage.as_mut_ptr().cast::<u8>().add(byte_offset) };

        // SAFETY: source and destination name distinct initialized ranges of equal length.
        unsafe {
            std::ptr::copy_nonoverlapping(source, destination, byte_len);
        }
    }
}

impl<'a> SectionImage<'a> {
    /// Borrow one aligned section image.
    pub fn new(storage: &'a SectionStorage) -> Self {
        // SAFETY: SectionStorage constructors retain aligned immutable image bytes.
        unsafe { Self::from_bytes_unchecked(storage.bytes()) }
    }

    /// Create one read-only section image without checking its storage length.
    ///
    /// # Safety
    ///
    /// The byte length must fit in the storage and slices must only be read as their original
    /// SectionEntry types.
    pub unsafe fn new_unchecked(storage: &'a SectionStorage) -> Self {
        unsafe { Self::from_bytes_unchecked(storage.bytes()) }
    }

    /// Create one read-only section image over aligned bytes.
    ///
    /// # Safety
    ///
    /// The bytes must remain immutable and every accessed slice must contain its original
    /// SectionEntry values at the recorded alignment.
    pub unsafe fn from_bytes_unchecked(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    /// Create one read-only section image from aligned chunks without checking coherence.
    ///
    /// # Safety
    ///
    /// The byte length must fit in the chunks and slices must only be read as their original
    /// SectionEntry types.
    unsafe fn from_chunks_unchecked(chunks: &'a [u128], byte_len: usize) -> Self {
        let bytes = chunks.as_ptr().cast::<u8>();

        // SAFETY: the caller requires byte_len to fit in the backing chunks.
        let bytes = unsafe { slice::from_raw_parts(bytes, byte_len) };

        Self { bytes }
    }

    /// Borrow one typed entry slice from the image.
    pub fn entries<T: SectionEntry>(&self, slice: SectionSlice<T>) -> &'a [T] {
        assert_ne!(
            mem::size_of::<T>(),
            0,
            "section entries must not be zero-sized"
        );

        if slice.is_empty() {
            return &[];
        }

        let byte_offset = slice.byte_offset as usize;
        let byte_len = slice.len as usize * mem::size_of::<T>();
        let byte_end = byte_offset + byte_len;
        assert!(
            byte_end <= self.bytes.len(),
            "section slice exceeds image bytes"
        );
        assert_eq!(
            byte_offset % mem::align_of::<T>(),
            0,
            "section slice is misaligned"
        );

        let bytes = self.bytes.as_ptr();
        let entries = unsafe { bytes.add(byte_offset).cast::<T>() };

        // SAFETY: entries enter the image only through insert<T>.
        // insert<T> copies SectionEntry values into an aligned chunk buffer and records an
        // aligned section offset, so this slice is aligned, initialized, and immutable.
        unsafe { slice::from_raw_parts(entries, slice.len as usize) }
    }

    /// Borrow one typed entry by index.
    pub fn entry<T: SectionEntry>(&self, slice: SectionSlice<T>, index: usize) -> Option<&'a T> {
        self.entries(slice).get(index)
    }

    /// Borrow one typed entry range from an image slice.
    pub fn range<T: SectionEntry>(&self, slice: SectionSlice<T>, range: EntryRange<T>) -> &'a [T] {
        range.slice(self.entries(slice))
    }

    /// Iterate one typed entry slice.
    pub fn iter<T: SectionEntry>(&self, slice: SectionSlice<T>) -> slice::Iter<'a, T> {
        self.entries(slice).iter()
    }

    /// Return immutable image bytes.
    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// Return the number of initialized image bytes.
    pub const fn byte_len(&self) -> usize {
        self.bytes.len()
    }
}

impl PartialEq for SectionImage<'_> {
    /// Compare section images by logical image content.
    fn eq(&self, other: &Self) -> bool {
        self.bytes() == other.bytes()
    }
}

impl Eq for SectionImage<'_> {}

fn align_usize(value: usize, alignment: usize) -> usize {
    let mask = alignment - 1;

    (value + mask) & !mask
}
