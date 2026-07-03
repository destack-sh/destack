use std::marker::PhantomData;
use std::{fmt, mem, slice};

use destack_serde::{Reflect, SchemaRef, SchemaRegistry};
use serde::{Deserialize, Serialize};

use crate::StringId;

const SECTION_TABLE_CHUNK_BYTES: usize = mem::size_of::<u128>();
const SECTION_TABLE_ALIGNMENT_BYTES: usize = mem::align_of::<u128>();

/// Invalid section image shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionImageError {
    /// Table segment id does not name a segment.
    MissingTableSegment {
        /// Missing table segment id.
        segment: SegmentId,
    },
    /// Segment has invalid byte bounds.
    InvalidSegment {
        /// Invalid segment id.
        segment: SegmentId,
        /// Explanation for the invalid segment.
        reason: &'static str,
    },
    /// Section has invalid byte bounds.
    InvalidSection {
        /// Invalid section id.
        section: SectionId,
        /// Explanation for the invalid section.
        reason: &'static str,
    },
    /// Table bytes do not fit in the backing chunk storage.
    TruncatedTable {
        /// Required initialized table bytes.
        required: u64,
        /// Available initialized table bytes.
        available: u64,
    },
}

impl fmt::Display for SectionImageError {
    /// Format one section image error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTableSegment { segment } => {
                write!(formatter, "missing table segment: {segment:?}")
            }
            Self::InvalidSegment { segment, reason } => {
                write!(formatter, "invalid segment {segment:?}: {reason}")
            }
            Self::InvalidSection { section, reason } => {
                write!(formatter, "invalid section {section:?}: {reason}")
            }
            Self::TruncatedTable {
                required,
                available,
            } => write!(
                formatter,
                "truncated table bytes: required {required}, available {available}"
            ),
        }
    }
}

impl std::error::Error for SectionImageError {}

/// Dense image segment id.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct SegmentId(pub u32);

impl SegmentId {
    /// Return this id as a dense table index.
    #[inline]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl From<u32> for SegmentId {
    /// Convert one raw segment id.
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<SegmentId> for u32 {
    /// Convert one segment id into its raw value.
    fn from(id: SegmentId) -> Self {
        id.0
    }
}

/// Dense image section id.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct SectionId(pub u32);

impl SectionId {
    /// Return this id as a dense table index.
    #[inline]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl From<u32> for SectionId {
    /// Convert one raw section id.
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<SectionId> for u32 {
    /// Convert one section id into its raw value.
    fn from(id: SectionId) -> Self {
        id.0
    }
}

/// Loadable byte segment inside one image.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Segment {
    /// Byte offset from the image base.
    pub offset: u64,
    /// Byte length stored in the image.
    pub byte_len: u64,
    /// Byte length reserved after loading.
    pub memory_len: u64,
    /// Required byte alignment.
    pub alignment: u32,
    /// Segment memory access policy.
    pub access: SegmentAccess,
}

impl Segment {
    /// Return the end offset of this segment in the image.
    #[inline]
    pub const fn end_offset(self) -> u64 {
        self.offset + self.byte_len
    }

    /// Return whether this segment carries no image bytes.
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.byte_len == 0
    }
}

/// Memory access policy for one segment.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SegmentAccess {
    /// Packed access flags.
    bits: u32,
}

impl SegmentAccess {
    /// Segment bytes may be read.
    pub const READ: u32 = 1 << 0;
    /// Segment bytes may be written.
    pub const WRITE: u32 = 1 << 1;
    /// Segment bytes may be executed.
    pub const EXECUTE: u32 = 1 << 2;
    /// All supported access flags.
    const ALL: u32 = Self::READ | Self::WRITE | Self::EXECUTE;

    /// Create one segment access policy.
    pub const fn new(is_readable: bool, is_writable: bool, is_executable: bool) -> Self {
        let bits = (Self::READ * is_readable as u32)
            | (Self::WRITE * is_writable as u32)
            | (Self::EXECUTE * is_executable as u32);

        Self { bits }
    }

    /// Return whether bytes may be read.
    pub const fn is_readable(self) -> bool {
        self.bits & Self::READ != 0
    }

    /// Return whether bytes may be written.
    pub const fn is_writable(self) -> bool {
        self.bits & Self::WRITE != 0
    }

    /// Return whether bytes may be executed.
    pub const fn is_executable(self) -> bool {
        self.bits & Self::EXECUTE != 0
    }

    /// Return whether only supported access bits are present.
    const fn is_valid(self) -> bool {
        self.bits & !Self::ALL == 0
    }
}

/// Loadable segment table inside one image.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Reflect)]
pub struct SegmentTable {
    /// Segments indexed by dense segment id.
    segments: Vec<Segment>,
}

impl SegmentTable {
    /// Create one empty segment table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert one segment and return its id.
    pub fn insert(&mut self, segment: Segment) -> SegmentId {
        let id = SegmentId(self.segments.len() as u32);
        self.segments.push(segment);

        id
    }

    /// Return one segment by id.
    pub fn get(&self, id: SegmentId) -> Option<Segment> {
        self.segments.get(id.index()).copied()
    }

    /// Mutate one segment by id.
    pub fn get_mut(&mut self, id: SegmentId) -> Option<&mut Segment> {
        self.segments.get_mut(id.index())
    }

    /// Return all segments in table order.
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }
}

impl PartialEq for SegmentTable {
    /// Compare segment tables by logical entries.
    fn eq(&self, other: &Self) -> bool {
        self.segments() == other.segments()
    }
}

impl Eq for SegmentTable {}

/// Logical byte section inside one image.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Section {
    /// Segment containing this section.
    pub segment: SegmentId,
    /// Byte offset from the containing segment base.
    pub byte_offset: u32,
    /// Section byte length inside the containing segment.
    pub byte_len: u32,
    /// Required byte alignment.
    pub alignment: u32,
}

impl Section {
    /// Return the end offset of this section inside its segment.
    #[inline]
    pub const fn end_offset(self) -> u32 {
        self.byte_offset + self.byte_len
    }

    /// Return whether this section carries no bytes.
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.byte_len == 0
    }
}

/// Logical section table inside one image.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Reflect)]
pub struct SectionTable {
    /// Sections indexed by dense section id.
    sections: Vec<Section>,
}

impl SectionTable {
    /// Create one empty section table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert one section and return its id.
    pub fn insert(&mut self, section: Section) -> SectionId {
        let id = SectionId(self.sections.len() as u32);
        self.sections.push(section);

        id
    }

    /// Return one section by id.
    pub fn get(&self, id: SectionId) -> Option<Section> {
        self.sections.get(id.index()).copied()
    }

    /// Return all sections in table order.
    pub fn sections(&self) -> &[Section] {
        &self.sections
    }
}

impl PartialEq for SectionTable {
    /// Compare section tables by logical entries.
    fn eq(&self, other: &Self) -> bool {
        self.sections() == other.sections()
    }
}

impl Eq for SectionTable {}

/// Typed entry slice stored in one image section.
#[repr(C)]
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SectionSlice<T> {
    /// Section containing the typed entries.
    pub section: SectionId,
    /// Byte offset from the containing section base.
    pub byte_offset: u32,
    /// Number of typed entries in the slice.
    pub len: u32,
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

impl<T> Default for SectionSlice<T> {
    /// Create an empty typed section slice.
    fn default() -> Self {
        Self::empty()
    }
}

impl<T> SectionSlice<T> {
    /// Create one section-relative typed slice.
    #[inline]
    pub const fn new(section: SectionId, byte_offset: u32, len: u32) -> Self {
        Self {
            section,
            byte_offset,
            len,
            marker: PhantomData,
        }
    }

    /// Create an empty typed section slice.
    #[inline]
    pub const fn empty() -> Self {
        Self::new(SectionId(0), 0, 0)
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
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
/// Implementors must have stable fixed-width layout, no process-local ownership, and no invalid
/// bit patterns in the section image format.
/// Their alignment must not exceed the section table alignment.
pub unsafe trait SectionEntry: Copy + 'static {}

// SAFETY: primitive integers and string ids are fixed-width entry scalars.
unsafe impl SectionEntry for u8 {}
unsafe impl SectionEntry for u16 {}
unsafe impl SectionEntry for u32 {}
unsafe impl SectionEntry for u64 {}
unsafe impl SectionEntry for i32 {}
unsafe impl SectionEntry for i64 {}
unsafe impl SectionEntry for StringId {}

// SAFETY: segments and sections are fixed-width image entry descriptors.
unsafe impl SectionEntry for Segment {}
unsafe impl SectionEntry for Section {}

// SAFETY: EntryRange<T> stores only section offsets and lengths.
unsafe impl<T: SectionEntry> SectionEntry for EntryRange<T> {}

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
    pub fn none() -> Self {
        let value = mem::MaybeUninit::zeroed();

        Self { is_some: 0, value }
    }

    /// Create a present optional entry value.
    pub fn some(value: T) -> Self {
        Self {
            is_some: 1,
            value: mem::MaybeUninit::new(value),
        }
    }

    /// Convert into a Rust option.
    pub fn get(self) -> Option<T> {
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

/// Section ids and byte ranges for one image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SectionDirectory {
    /// Loadable image segments.
    segments: SegmentTable,
    /// Logical image sections.
    sections: SectionTable,
    /// Segment holding immutable entry tables.
    table_segment: SegmentId,
    /// Number of initialized bytes in the table segment.
    table_byte_len: u64,
}

impl Default for SectionDirectory {
    /// Create an empty section directory.
    fn default() -> Self {
        Self::new()
    }
}

/// Immutable aligned section storage.
#[derive(Debug, Clone)]
pub enum SectionStorage {
    /// Owned aligned section chunks.
    Owned(Vec<u128>),
    /// Prelinked static section chunks.
    Static(&'static [u128]),
}

impl Serialize for SectionStorage {
    /// Serialize section storage as its logical aligned chunks.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.chunks().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SectionStorage {
    /// Deserialize section storage into owned aligned chunks.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let chunks = Vec::<u128>::deserialize(deserializer)?;

        Ok(Self::Owned(chunks))
    }
}

impl Reflect for SectionStorage {
    /// Reflect section storage as its logical aligned chunks.
    fn reflect(registry: &mut SchemaRegistry) -> SchemaRef {
        Vec::<u128>::reflect(registry)
    }
}

impl SectionStorage {
    /// Create prelinked static section storage.
    pub fn from_static(chunks: &'static [u128]) -> Self {
        Self::Static(chunks)
    }

    /// Create owned section storage from raw table bytes.
    pub fn from_table_bytes(
        table_bytes: &[u8],
        table_byte_len: u64,
    ) -> Result<Self, SectionImageError> {
        let Ok(table_byte_len) = usize::try_from(table_byte_len) else {
            return Err(SectionImageError::TruncatedTable {
                required: table_byte_len,
                available: table_bytes.len() as u64,
            });
        };

        // reject truncated table storage before copying
        if table_bytes.len() < table_byte_len {
            return Err(SectionImageError::TruncatedTable {
                required: table_byte_len as u64,
                available: table_bytes.len() as u64,
            });
        }

        // copy into aligned table storage
        let chunk_len = table_byte_len.div_ceil(SECTION_TABLE_CHUNK_BYTES);
        let mut chunks = vec![0; chunk_len];
        let destination = chunks.as_mut_ptr().cast::<u8>();

        // SAFETY: chunks has enough initialized byte storage for table_byte_len bytes.
        unsafe {
            std::ptr::copy_nonoverlapping(table_bytes.as_ptr(), destination, table_byte_len);
        }

        Ok(Self::Owned(chunks))
    }

    /// Return immutable aligned storage chunks.
    fn chunks(&self) -> &[u128] {
        match self {
            Self::Owned(chunks) => chunks,
            Self::Static(chunks) => chunks,
        }
    }
}

/// Mutable writer that packs typed sections into one section image.
#[derive(Debug, Clone)]
pub struct SectionPacker {
    /// Section image directory being packed.
    directory: SectionDirectory,
    /// Mutable table image storage being packed.
    storage: Vec<u128>,
}

/// Borrowed read-only section view.
#[derive(Clone, Copy, Debug)]
pub struct SectionImage<'a> {
    /// Section directory.
    directory: &'a SectionDirectory,
    /// Aligned section storage chunks.
    chunks: &'a [u128],
}

impl Default for SectionPacker {
    /// Create an empty section packer.
    fn default() -> Self {
        Self::new()
    }
}

impl SectionDirectory {
    /// Create an empty section directory.
    pub fn new() -> Self {
        let mut segments = SegmentTable::new();
        let table_segment = segments.insert(Segment {
            offset: 0,
            byte_len: 0,
            memory_len: 0,
            alignment: SECTION_TABLE_ALIGNMENT_BYTES as u32,
            access: SegmentAccess::new(true, false, false),
        });

        Self {
            segments,
            sections: SectionTable::new(),
            table_segment,
            table_byte_len: 0,
        }
    }

    /// Create one section directory from explicit parts.
    pub fn from_parts(
        segments: SegmentTable,
        sections: SectionTable,
        table_segment: SegmentId,
        table_byte_len: u64,
    ) -> Self {
        Self {
            segments,
            sections,
            table_segment,
            table_byte_len,
        }
    }

    /// Return the loadable image segments.
    pub fn segments(&self) -> &SegmentTable {
        &self.segments
    }

    /// Return the logical image sections.
    pub fn sections(&self) -> &SectionTable {
        &self.sections
    }

    /// Return the table segment id.
    pub fn table_segment(&self) -> SegmentId {
        self.table_segment
    }

    /// Return the number of table bytes.
    pub fn table_byte_len(&self) -> u64 {
        self.table_byte_len
    }

    /// Check section ids and byte ranges against aligned chunks.
    fn check(&self, chunks: &[u128]) -> Result<(), SectionImageError> {
        // resolve the table segment first
        let Some(table_segment) = self.segments.get(self.table_segment) else {
            return Err(SectionImageError::MissingTableSegment {
                segment: self.table_segment,
            });
        };

        // check segment bounds
        for (index, segment) in self.segments.segments().iter().copied().enumerate() {
            let segment_id = SegmentId(index as u32);

            // require usable segment alignment
            if segment.alignment == 0 || !segment.alignment.is_power_of_two() {
                return Err(SectionImageError::InvalidSegment {
                    segment: segment_id,
                    reason: "alignment must be a non-zero power of two",
                });
            }

            // require initialized bytes to fit in mapped memory
            if segment.byte_len > segment.memory_len {
                return Err(SectionImageError::InvalidSegment {
                    segment: segment_id,
                    reason: "byte length exceeds memory length",
                });
            }

            // reject unknown access flags
            if !segment.access.is_valid() {
                return Err(SectionImageError::InvalidSegment {
                    segment: segment_id,
                    reason: "access contains unknown flags",
                });
            }
        }

        // check table segment storage
        if table_segment.byte_len < self.table_byte_len {
            return Err(SectionImageError::InvalidSegment {
                segment: self.table_segment,
                reason: "table segment is shorter than table byte length",
            });
        }

        let available_table_bytes = (chunks.len() * SECTION_TABLE_CHUNK_BYTES) as u64;
        if available_table_bytes < self.table_byte_len {
            return Err(SectionImageError::TruncatedTable {
                required: self.table_byte_len,
                available: available_table_bytes,
            });
        }

        // check logical sections
        for (index, section) in self.sections.sections().iter().copied().enumerate() {
            let section_id = SectionId(index as u32);

            // resolve containing segment
            let Some(segment) = self.segments.get(section.segment) else {
                return Err(SectionImageError::InvalidSection {
                    section: section_id,
                    reason: "section references missing segment",
                });
            };

            // require usable section alignment
            if section.alignment == 0 || !section.alignment.is_power_of_two() {
                return Err(SectionImageError::InvalidSection {
                    section: section_id,
                    reason: "alignment must be a non-zero power of two",
                });
            }

            // require the section range to fit in its segment
            let byte_end = section.byte_offset as u64 + section.byte_len as u64;
            if byte_end > segment.memory_len {
                return Err(SectionImageError::InvalidSection {
                    section: section_id,
                    reason: "section exceeds segment memory length",
                });
            }
        }

        Ok(())
    }
}

impl SectionPacker {
    /// Create an empty section packer.
    pub fn new() -> Self {
        Self {
            directory: SectionDirectory::new(),
            storage: Vec::new(),
        }
    }

    /// Borrow this packer as a read-only section view.
    pub fn view(&self) -> SectionImage<'_> {
        // SAFETY: SectionPacker creates directory and storage together through insert.
        unsafe { SectionImage::from_chunks_unchecked(&self.directory, &self.storage) }
    }

    /// Return the section directory.
    pub fn directory(&self) -> &SectionDirectory {
        &self.directory
    }

    /// Return immutable aligned storage chunks.
    #[cfg(test)]
    fn chunks(&self) -> &[u128] {
        &self.storage
    }

    /// Finish this packer into its directory and storage.
    pub fn finish(self) -> (SectionDirectory, SectionStorage) {
        (self.directory, SectionStorage::Owned(self.storage))
    }

    /// Insert one typed entry section.
    pub fn insert<T: SectionEntry>(&mut self, entries: impl AsRef<[T]>) -> SectionSlice<T> {
        debug_assert_ne!(mem::size_of::<T>(), 0);
        debug_assert!(mem::align_of::<T>() <= SECTION_TABLE_ALIGNMENT_BYTES);

        let entries = entries.as_ref();
        let byte_len = mem::size_of_val(entries);
        let alignment = mem::align_of::<T>().max(1);
        let byte_offset = align_usize(self.directory.table_byte_len as usize, alignment);

        // copy typed entries into the aligned table image
        let section_end = byte_offset + byte_len;
        let chunk_len = section_end.div_ceil(SECTION_TABLE_CHUNK_BYTES);
        self.storage.resize(chunk_len, 0);

        let source = entries.as_ptr().cast::<u8>();

        // SAFETY: storage was resized to contain section_end bytes above.
        let destination = unsafe { self.storage.as_mut_ptr().cast::<u8>().add(byte_offset) };

        // SAFETY: source points to byte_len initialized entry bytes and destination
        // points to distinct table storage with enough initialized capacity.
        unsafe {
            std::ptr::copy_nonoverlapping(source, destination, byte_len);
        }

        // record the logical section inside the table segment
        let section = self.directory.sections.insert(Section {
            segment: self.directory.table_segment,
            byte_offset: byte_offset as u32,
            byte_len: byte_len as u32,
            alignment: alignment as u32,
        });
        self.directory.table_byte_len = section_end as u64;

        // update the physical table segment extent
        if let Some(segment) = self
            .directory
            .segments
            .get_mut(self.directory.table_segment)
        {
            segment.byte_len = self.directory.table_byte_len;
            segment.memory_len = self.directory.table_byte_len;
        }

        SectionSlice::new(section, 0, entries.len() as u32)
    }
}

impl<'a> SectionImage<'a> {
    /// Load one checked read-only section view.
    pub fn load(
        directory: &'a SectionDirectory,
        storage: &'a SectionStorage,
    ) -> Result<Self, SectionImageError> {
        directory.check(storage.chunks())?;

        // SAFETY: check above verified directory and storage coherence.
        Ok(unsafe { Self::new_unchecked(directory, storage) })
    }

    /// Create one read-only section view without checking directory and storage coherence.
    ///
    /// # Safety
    ///
    /// The directory must describe initialized storage bytes, all sections must fit in their
    /// segments, and all typed section slices must only be read as their original SectionEntry
    /// types.
    pub unsafe fn new_unchecked(
        directory: &'a SectionDirectory,
        storage: &'a SectionStorage,
    ) -> Self {
        Self {
            directory,
            chunks: storage.chunks(),
        }
    }

    /// Create one read-only section view from aligned chunks without checking coherence.
    ///
    /// # Safety
    ///
    /// The directory must describe initialized storage chunks, all sections must fit in their
    /// segments, and all typed section slices must only be read as their original SectionEntry
    /// types.
    unsafe fn from_chunks_unchecked(directory: &'a SectionDirectory, chunks: &'a [u128]) -> Self {
        Self { directory, chunks }
    }

    /// Return the section directory.
    pub fn directory(&self) -> &'a SectionDirectory {
        self.directory
    }

    /// Borrow one typed entry slice from the image.
    pub fn entries<T: SectionEntry>(&self, slice: SectionSlice<T>) -> &'a [T] {
        debug_assert_ne!(mem::size_of::<T>(), 0);

        if slice.is_empty() {
            return &[];
        }

        let Some(section) = self.directory.sections.get(slice.section) else {
            unreachable!("section slice references missing section");
        };
        debug_assert_eq!(section.segment, self.directory.table_segment);

        let byte_offset = section.byte_offset as usize + slice.byte_offset as usize;
        let byte_len = slice.len as usize * mem::size_of::<T>();
        let byte_end = byte_offset + byte_len;
        debug_assert!(byte_end <= self.directory.table_byte_len as usize);
        debug_assert_eq!(byte_offset % mem::align_of::<T>(), 0);

        let bytes = self.chunk_slice().as_ptr().cast::<u8>();
        let entries = unsafe { bytes.add(byte_offset).cast::<T>() };

        // SAFETY: entries enter the table image only through insert<T>.
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

    /// Return immutable table bytes.
    pub fn table_bytes(&self) -> &'a [u8] {
        let byte_len = self.directory.table_byte_len as usize;
        let bytes = self.chunk_slice().as_ptr().cast::<u8>();

        // SAFETY: chunk_slice returns at least table_byte_len initialized bytes.
        unsafe { slice::from_raw_parts(bytes, byte_len) }
    }

    /// Return the number of table bytes.
    pub fn table_byte_len(&self) -> u64 {
        self.directory.table_byte_len
    }

    /// Return immutable aligned storage chunks.
    fn chunk_slice(&self) -> &'a [u128] {
        self.chunks
    }
}

impl PartialEq for SectionImage<'_> {
    /// Compare section images by logical image content.
    fn eq(&self, other: &Self) -> bool {
        self.directory == other.directory && self.table_bytes() == other.table_bytes()
    }
}

impl Eq for SectionImage<'_> {}

fn align_usize(value: usize, alignment: usize) -> usize {
    let mask = alignment - 1;

    (value + mask) & !mask
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_borrow_static_section_image() {
        let mut packer = SectionPacker::new();
        let values = packer.insert(vec![1u32, 2, 3]);
        let owned = packer.view();

        let directory = packer.directory().clone();
        let chunks = packer.chunks().to_vec().into_boxed_slice();

        let chunks = Box::leak(chunks);
        let storage = SectionStorage::from_static(chunks);
        let prelinked = SectionImage::load(&directory, &storage).unwrap();

        assert_eq!(prelinked.entries(values), &[1, 2, 3]);
        assert_eq!(prelinked.entry(values, 1), Some(&2));
        assert_eq!(prelinked.iter(values).copied().sum::<u32>(), 6);
        assert_eq!(prelinked, owned);

        let bytes = destack_serde::to_vec(&directory).unwrap();
        let decoded = destack_serde::from_slice::<SectionDirectory>(&bytes).unwrap();
        let decoded = SectionImage::load(&decoded, &storage).unwrap();

        assert_eq!(decoded, owned);
    }
}
