use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::{fmt, mem, slice};

use serde::{Deserialize, Serialize};
pub use tspp_serde::SectionEntry;
use tspp_serde::{Reflect, Schema, Type};

use crate::{Blob, BlobMemory, StringId};

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
    /// One typed section lies outside the image or violates entry alignment.
    InvalidSection,
    /// One typed section contains an invalid entry representation.
    InvalidEntry,
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
            Self::InvalidSection => formatter.write_str("invalid image section"),
            Self::InvalidEntry => formatter.write_str("invalid image entry"),
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

    /// Return whether this range fits one sibling entry slice.
    #[inline]
    pub fn fits(self, entries: usize) -> bool {
        self.start
            .checked_add(self.len)
            .is_some_and(|end| end as usize <= entries)
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
pub unsafe trait SectionEntry: Copy + 'static {
    /// Whether this representation or one referenced section requires validation.
    const NEEDS_VALIDATION: bool;

    /// Validate one entry and every absolute section it references.
    fn validate(bytes: &[u8], loader: SectionLoader<'_>) -> Result<(), SectionImageError>;
}

macro_rules! scalar_entries {
    ($($ty:ty),* $(,)?) => {
        $(
            // SAFETY: every bit pattern is a valid scalar value.
            unsafe impl SectionEntry for $ty {
                const NEEDS_VALIDATION: bool = false;

                fn validate(
                    _bytes: &[u8],
                    _loader: SectionLoader<'_>,
                ) -> Result<(), SectionImageError> {
                    Ok(())
                }
            }
        )*
    };
}

scalar_entries!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

// SAFETY: every bit pattern is a valid stable string identity.
unsafe impl SectionEntry for StringId {
    const NEEDS_VALIDATION: bool = false;

    fn validate(_bytes: &[u8], _loader: SectionLoader<'_>) -> Result<(), SectionImageError> {
        Ok(())
    }
}

// SAFETY: validation rejects the scalar's invalid zero representation.
unsafe impl SectionEntry for NonZeroU32 {
    const NEEDS_VALIDATION: bool = true;

    fn validate(bytes: &[u8], _loader: SectionLoader<'_>) -> Result<(), SectionImageError> {
        if bytes.len() != mem::size_of::<Self>() {
            return Err(SectionImageError::InvalidEntry);
        }

        let mut value = [0; mem::size_of::<u32>()];
        value.copy_from_slice(bytes);

        if u32::from_ne_bytes(value) == 0 {
            Err(SectionImageError::InvalidEntry)
        } else {
            Ok(())
        }
    }
}

// SAFETY: fixed arrays preserve their entry layout and validate every element.
unsafe impl<T: SectionEntry, const N: usize> SectionEntry for [T; N] {
    const NEEDS_VALIDATION: bool = T::NEEDS_VALIDATION;

    fn validate(bytes: &[u8], loader: SectionLoader<'_>) -> Result<(), SectionImageError> {
        validate_entries::<T>(bytes, N, loader)
    }
}

// SAFETY: EntryRange<T> stores only integer offsets and lengths.
unsafe impl<T: SectionEntry> SectionEntry for EntryRange<T> {
    const NEEDS_VALIDATION: bool = false;

    fn validate(_bytes: &[u8], _loader: SectionLoader<'_>) -> Result<(), SectionImageError> {
        Ok(())
    }
}

// SAFETY: SectionSlice<T> stores only an image offset and integer entry count.
unsafe impl<T: SectionEntry> SectionEntry for SectionSlice<T> {
    const NEEDS_VALIDATION: bool = true;

    fn validate(bytes: &[u8], loader: SectionLoader<'_>) -> Result<(), SectionImageError> {
        if bytes.len() != mem::size_of::<Self>() {
            return Err(SectionImageError::InvalidEntry);
        }

        // SAFETY: SectionSlice contains only integer fields and a zero-sized marker.
        let section = unsafe { bytes.as_ptr().cast::<Self>().read_unaligned() };
        loader.entries(section)?;

        Ok(())
    }
}

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
    fn reflect(schema: &mut Schema) -> Type {
        Option::<T>::reflect(schema)
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

// SAFETY: Optional<T> validates its tag and every initialized value.
unsafe impl<T: SectionEntry> SectionEntry for Optional<T> {
    const NEEDS_VALIDATION: bool = true;

    fn validate(bytes: &[u8], loader: SectionLoader<'_>) -> Result<(), SectionImageError> {
        if bytes.len() != mem::size_of::<Self>() {
            return Err(SectionImageError::InvalidEntry);
        }

        // read the stable presence tag without constructing Optional<T>
        let mut presence = [0; mem::size_of::<u32>()];
        presence.copy_from_slice(&bytes[..mem::size_of::<u32>()]);
        let presence = u32::from_ne_bytes(presence);
        if presence == 0 {
            return Ok(());
        }
        if presence != 1 {
            return Err(SectionImageError::InvalidEntry);
        }

        // validate the initialized value after its representation padding
        let value_offset = mem::offset_of!(Self, value);
        let value_end = value_offset + mem::size_of::<T>();

        if T::NEEDS_VALIDATION {
            T::validate(&bytes[value_offset..value_end], loader)?;
        }

        Ok(())
    }
}

/// Immutable aligned section storage.
#[derive(Debug, Clone)]
pub struct SectionStorage {
    /// Owned, static, or shared image bytes.
    bytes: SectionBytes,
}

/// Physical storage for one section image.
#[derive(Debug, Clone)]
enum SectionBytes {
    /// Owned aligned image bytes.
    Owned(Buffer),
    /// Prelinked static image chunks and initialized byte length.
    Static {
        /// Aligned image chunks.
        chunks: &'static [u128],
        /// Number of initialized bytes.
        byte_len: u64,
    },
    /// Shared immutable image memory.
    Shared(Arc<BlobMemory>),
}

impl Serialize for SectionStorage {
    /// Serialize section storage as its logical image bytes.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.bytes().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SectionStorage {
    /// Deserialize section storage into owned aligned bytes.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;

        Ok(Self::from_bytes(&bytes))
    }
}

impl Reflect for SectionStorage {
    /// Reflect section storage as its logical image bytes.
    fn reflect(schema: &mut Schema) -> Type {
        Vec::<u8>::reflect(schema)
    }
}

impl SectionStorage {
    /// Create prelinked static section storage.
    pub fn from_static(chunks: &'static [u128], byte_len: u64) -> Result<Self, SectionImageError> {
        Self::check_len(chunks, byte_len)?;

        Ok(Self {
            bytes: SectionBytes::Static { chunks, byte_len },
        })
    }

    /// Create section storage from shared immutable memory.
    pub fn from_memory(memory: Arc<BlobMemory>) -> Self {
        let bytes = memory.bytes();
        if (bytes.as_ptr() as usize).is_multiple_of(SECTION_ALIGNMENT_BYTES) {
            Self {
                bytes: SectionBytes::Shared(memory),
            }
        } else {
            Self::from_bytes(bytes)
        }
    }

    /// Copy raw bytes into owned aligned section storage.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            bytes: SectionBytes::Owned(Buffer::from_bytes(bytes, SECTION_ALIGNMENT_BYTES)),
        }
    }

    /// Return the exact Blob identity of this section image.
    pub fn blob(&self) -> Blob {
        match &self.bytes {
            SectionBytes::Shared(memory) => memory.blob(),
            SectionBytes::Owned(_) | SectionBytes::Static { .. } => Blob::for_bytes(self.bytes()),
        }
    }

    /// Align owned bytes or require retained memory to satisfy the requested alignment.
    pub fn align(&mut self, alignment: usize) -> Result<(), SectionImageError> {
        if !alignment.is_power_of_two() {
            return Err(SectionImageError::Misaligned);
        }
        let is_aligned = (self.bytes().as_ptr() as usize).is_multiple_of(alignment);

        // retain alignment across future owned storage clones
        match &mut self.bytes {
            SectionBytes::Owned(bytes) => {
                bytes.align(alignment);

                Ok(())
            }
            // retain compatible shared and static storage in place
            SectionBytes::Static { .. } | SectionBytes::Shared(_) if is_aligned => Ok(()),

            // immutable static images must already satisfy their declared alignment
            SectionBytes::Static { .. } => Err(SectionImageError::Misaligned),

            // copy an unusually aligned mapped image only when required
            SectionBytes::Shared(_) => {
                let bytes = Buffer::from_bytes(self.bytes(), alignment);
                self.bytes = SectionBytes::Owned(bytes);

                Ok(())
            }
        }
    }

    /// Return initialized image bytes.
    pub fn bytes(&self) -> &[u8] {
        match &self.bytes {
            SectionBytes::Owned(bytes) => bytes.as_ref(),
            SectionBytes::Static { chunks, byte_len } => {
                let bytes = chunks.as_ptr().cast::<u8>();

                // SAFETY: constructors require byte_len to fit in the backing chunks.
                unsafe { slice::from_raw_parts(bytes, *byte_len as usize) }
            }
            SectionBytes::Shared(memory) => memory.bytes(),
        }
    }

    /// Return the number of initialized bytes.
    pub fn byte_len(&self) -> u64 {
        match &self.bytes {
            SectionBytes::Owned(bytes) => bytes.len() as u64,
            SectionBytes::Static { byte_len, .. } => *byte_len,
            SectionBytes::Shared(memory) => memory.bytes().len() as u64,
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

/// Owned section buffer with one stable power-of-two alignment.
#[derive(Debug)]
struct Buffer {
    /// Overallocated backing bytes.
    allocation: Vec<u8>,
    /// Aligned logical image start.
    offset: usize,
    /// Number of initialized image bytes.
    byte_len: usize,
    /// Guaranteed image alignment.
    alignment: usize,
}

/// Mutable writer that packs typed sections into one section image.
#[derive(Debug, Clone)]
pub struct SectionBuilder {
    /// Mutable aligned image storage being packed.
    storage: Buffer,
}

/// Borrowed read-only section view.
#[derive(Clone, Copy, Debug)]
pub struct SectionImage<'a> {
    /// Exact immutable image bytes.
    bytes: &'a [u8],
}

/// Checked loader for typed entries inside untrusted section storage.
#[derive(Clone, Copy, Debug)]
pub struct SectionLoader<'a> {
    /// Complete aligned image bytes.
    bytes: &'a [u8],
}

impl Default for SectionBuilder {
    /// Create an empty section builder.
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Buffer {
    /// Clone logical bytes into a separately aligned allocation.
    fn clone(&self) -> Self {
        Self::from_bytes(self.as_ref(), self.alignment)
    }
}

impl AsRef<[u8]> for Buffer {
    /// Borrow initialized image bytes.
    fn as_ref(&self) -> &[u8] {
        &self.allocation[self.offset..self.offset + self.byte_len]
    }
}

impl Buffer {
    /// Create empty storage with the requested alignment and capacity.
    fn new(alignment: usize, capacity: usize) -> Self {
        let allocation_len = capacity.max(1) + alignment - 1;
        let allocation = vec![0; allocation_len];
        let address = allocation.as_ptr() as usize;
        let offset = align_usize(address, alignment) - address;

        Self {
            allocation,
            offset,
            byte_len: 0,
            alignment,
        }
    }

    /// Copy bytes into one aligned allocation.
    fn from_bytes(bytes: &[u8], alignment: usize) -> Self {
        let mut buffer = Self::new(alignment, bytes.len());
        buffer.allocation_mut()[..bytes.len()].copy_from_slice(bytes);
        buffer.byte_len = bytes.len();

        buffer
    }

    /// Return the initialized byte count.
    fn len(&self) -> usize {
        self.byte_len
    }

    /// Return the available logical byte capacity.
    fn capacity(&self) -> usize {
        self.allocation.len() - self.offset
    }

    /// Strengthen the logical image alignment.
    fn align(&mut self, alignment: usize) {
        let byte_len = self.byte_len;
        self.grow(byte_len, alignment);
    }

    /// Grow initialized storage and strengthen its alignment when required.
    fn grow(&mut self, byte_len: usize, alignment: usize) {
        assert!(byte_len >= self.byte_len, "aligned storage cannot shrink");

        let alignment = self.alignment.max(alignment);
        let must_reallocate = alignment != self.alignment || byte_len > self.capacity();

        // preserve logical bytes while changing allocation shape
        if must_reallocate {
            let capacity = byte_len
                .max(self.capacity().saturating_mul(2))
                .max(SECTION_CHUNK_BYTES);
            let mut buffer = Self::new(alignment, capacity);
            buffer.allocation_mut()[..self.byte_len].copy_from_slice(self.as_ref());
            buffer.byte_len = byte_len;
            *self = buffer;
        }
        // initialize newly exposed bytes in place
        else {
            let initialized = self.byte_len;
            self.allocation_mut()[initialized..byte_len].fill(0);
            self.byte_len = byte_len;
        }
    }

    /// Borrow the complete logical allocation mutably.
    fn allocation_mut(&mut self) -> &mut [u8] {
        &mut self.allocation[self.offset..]
    }
}

impl SectionBuilder {
    /// Create an empty section builder.
    pub fn new() -> Self {
        Self {
            storage: Buffer::new(SECTION_ALIGNMENT_BYTES, 0),
        }
    }

    /// Borrow this builder as a read-only section image.
    pub fn view(&self) -> SectionImage<'_> {
        // SAFETY: SectionBuilder creates offsets and storage together through insert.
        unsafe { SectionImage::from_bytes_unchecked(self.storage.as_ref()) }
    }

    /// Return the physical alignment required by this image.
    pub const fn alignment(&self) -> usize {
        self.storage.alignment
    }

    /// Build immutable aligned section storage.
    pub fn build(self) -> SectionStorage {
        SectionStorage {
            bytes: SectionBytes::Owned(self.storage),
        }
    }

    /// Insert one typed entry section.
    pub fn insert<T: SectionEntry>(&mut self, entries: impl AsRef<[T]>) -> SectionSlice<T> {
        assert_ne!(
            mem::size_of::<T>(),
            0,
            "section entries must not be zero-sized"
        );

        let entries = entries.as_ref();
        let byte_len = mem::size_of_val(entries);
        let alignment = mem::align_of::<T>().max(1);
        let byte_offset = align_usize(self.storage.len(), alignment);

        // copy typed entries into the aligned section image
        let section_end = byte_offset + byte_len;
        self.storage.grow(section_end, alignment);

        let source = entries.as_ptr().cast::<u8>();

        // SAFETY: storage was resized to contain section_end bytes above.
        let destination = unsafe { self.storage.allocation_mut().as_mut_ptr().add(byte_offset) };

        // SAFETY: source points to byte_len initialized entry bytes and destination
        // points to distinct table storage with enough initialized capacity.
        unsafe {
            std::ptr::copy_nonoverlapping(source, destination, byte_len);
        }

        SectionSlice::new(byte_offset as u64, entries.len() as u32)
    }

    /// Insert raw bytes at one explicit power-of-two alignment.
    pub fn insert_bytes(&mut self, bytes: impl AsRef<[u8]>, alignment: usize) -> SectionSlice<u8> {
        assert!(
            alignment.is_power_of_two(),
            "section alignment must be a power of two"
        );
        let bytes = bytes.as_ref();
        let byte_offset = align_usize(self.storage.len(), alignment);
        let byte_end = byte_offset + bytes.len();
        self.storage.grow(byte_end, alignment);

        // copy bytes into the explicitly aligned image range
        self.storage.allocation_mut()[byte_offset..byte_end].copy_from_slice(bytes);

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
            byte_end <= self.storage.len(),
            "replacement section exceeds image bytes"
        );

        // overwrite the complete existing section
        let source = entries.as_ptr().cast::<u8>();
        let destination = unsafe { self.storage.allocation_mut().as_mut_ptr().add(byte_offset) };

        // SAFETY: source and destination name distinct initialized ranges of equal length.
        unsafe {
            std::ptr::copy_nonoverlapping(source, destination, byte_len);
        }
    }
}

impl<'a> SectionImage<'a> {
    /// Borrow one trusted read-only section image.
    ///
    /// # Safety
    ///
    /// Every accessed slice must contain valid entries of its original SectionEntry type.
    pub unsafe fn new(storage: &'a SectionStorage) -> Self {
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
        let bytes = self.bytes.as_ptr();
        let entries = unsafe { bytes.add(byte_offset).cast::<T>() };
        assert_eq!(
            entries as usize % mem::align_of::<T>(),
            0,
            "section slice is misaligned"
        );

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

impl<'a> SectionLoader<'a> {
    /// Create one checked loader over aligned immutable storage.
    pub fn new(storage: &'a SectionStorage) -> Result<Self, SectionImageError> {
        let bytes = storage.bytes();
        if !(bytes.as_ptr() as usize).is_multiple_of(SECTION_ALIGNMENT_BYTES) {
            return Err(SectionImageError::Misaligned);
        }

        Ok(Self { bytes })
    }

    /// Load one fixed header from byte zero.
    pub fn header<T: SectionEntry>(&self) -> Result<&'a T, SectionImageError> {
        let byte_len = mem::size_of::<T>();
        if self.bytes.len() < byte_len {
            return Err(SectionImageError::Truncated {
                required: byte_len as u64,
                available: self.bytes.len() as u64,
            });
        }
        if !(self.bytes.as_ptr() as usize).is_multiple_of(mem::align_of::<T>()) {
            return Err(SectionImageError::Misaligned);
        }
        if T::NEEDS_VALIDATION {
            T::validate(&self.bytes[..byte_len], *self)?;
        }

        // SAFETY: the loader checks image alignment, bounds, and entry representation above.
        Ok(unsafe { &*self.bytes.as_ptr().cast::<T>() })
    }

    /// Load one typed section after checking its complete physical representation.
    pub fn entries<T: SectionEntry>(
        &self,
        section: SectionSlice<T>,
    ) -> Result<&'a [T], SectionImageError> {
        let byte_offset = section.byte_offset as usize;
        let Some(byte_len) = section.len().checked_mul(mem::size_of::<T>()) else {
            return Err(SectionImageError::InvalidSection);
        };
        let Some(byte_end) = byte_offset.checked_add(byte_len) else {
            return Err(SectionImageError::InvalidSection);
        };
        let address = self.bytes.as_ptr() as usize + byte_offset;
        if byte_end > self.bytes.len() || !address.is_multiple_of(mem::align_of::<T>()) {
            return Err(SectionImageError::InvalidSection);
        }

        if T::NEEDS_VALIDATION {
            let bytes = &self.bytes[byte_offset..byte_end];
            validate_entries::<T>(bytes, section.len(), *self)?;
        }

        // SAFETY: bounds, alignment, and every entry representation were checked above.
        let entries = unsafe { self.bytes.as_ptr().add(byte_offset).cast::<T>() };

        // SAFETY: the checked entries remain immutable for the loader lifetime.
        Ok(unsafe { slice::from_raw_parts(entries, section.len()) })
    }

    /// Return complete image bytes.
    pub const fn bytes(self) -> &'a [u8] {
        self.bytes
    }
}

impl PartialEq for SectionImage<'_> {
    /// Compare section images by logical image content.
    fn eq(&self, other: &Self) -> bool {
        self.bytes() == other.bytes()
    }
}

impl Eq for SectionImage<'_> {}

/// Validate one contiguous sequence of section entries.
fn validate_entries<T: SectionEntry>(
    bytes: &[u8],
    entry_len: usize,
    loader: SectionLoader<'_>,
) -> Result<(), SectionImageError> {
    let Some(byte_len) = entry_len.checked_mul(mem::size_of::<T>()) else {
        return Err(SectionImageError::InvalidSection);
    };
    if bytes.len() != byte_len {
        return Err(SectionImageError::InvalidSection);
    }

    for entry in bytes.chunks_exact(mem::size_of::<T>()) {
        T::validate(entry, loader)?;
    }

    Ok(())
}

fn align_usize(value: usize, alignment: usize) -> usize {
    let mask = alignment - 1;

    (value + mask) & !mask
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Preserve existing sections when a later section strengthens image alignment.
    #[test]
    fn test_preserve_overaligned_sections() {
        let mut builder = SectionBuilder::new();
        let numbers = builder.insert([11_u32, 22]);
        let bytes = builder.insert_bytes([1_u8, 2, 3, 4], 64);
        let storage = builder.build();

        // require independent owned images to retain content and physical alignment
        for storage in [storage.clone(), storage] {
            let image = unsafe { SectionImage::new(&storage) };
            let address = image.bytes().as_ptr() as usize + bytes.byte_offset as usize;

            assert_eq!(image.entries(numbers), &[11, 22]);
            assert_eq!(image.entries(bytes), &[1, 2, 3, 4]);
            assert!(address.is_multiple_of(64));
        }
    }

    /// Realign copied storage while preserving its complete logical image.
    #[test]
    fn test_realign_owned_section_storage() {
        let mut storage = SectionStorage::from_bytes(&[1, 2, 3, 4]);
        storage.align(128).expect("owned storage should realign");

        // retain strengthened alignment when owned storage is cloned
        for storage in [storage.clone(), storage] {
            assert_eq!(storage.bytes(), &[1, 2, 3, 4]);
            assert!((storage.bytes().as_ptr() as usize).is_multiple_of(128));
        }
    }
}
