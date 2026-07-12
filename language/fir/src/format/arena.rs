use std::alloc::{Layout, alloc, dealloc, handle_alloc_error};
use std::cell::{Cell, UnsafeCell};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::mem::{align_of, size_of};
use std::ops::{Deref, DerefMut};
use std::ptr::{NonNull, copy_nonoverlapping};

/// The target first chunk size.
const CHUNK_BYTES: usize = 16 * 1024;
/// The minimum chunk alignment.
const CHUNK_ALIGNMENT: usize = 16;
/// The chunk size rounding unit.
const PAGE_BYTES: usize = 4 * 1024;

/// Storage for one formatting pass.
///
/// Allocations remain stable until the allocator releases every chunk in bulk.
/// Each formatting pass owns one allocator and uses it from one thread.
pub struct Allocator {
    /// The unallocated range in the current chunk.
    current: Cell<Option<Cursor>>,
    /// The owned allocation chunks.
    chunks: UnsafeCell<std::vec::Vec<Chunk>>,
}

impl Debug for Allocator {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // read single-threaded allocator state
        // safety: Allocator is not Sync, so formatting cannot race allocation
        let chunks = unsafe { &*self.chunks.get() };

        // report retained chunk count
        formatter
            .debug_struct("Allocator")
            .field("chunks", &chunks.len())
            .finish()
    }
}

impl Allocator {
    /// Allocate one copyable value.
    pub(crate) fn alloc<T: Copy>(&self, value: T) -> &T {
        // allocate stable value storage
        let pointer: NonNull<T> = self.allocate_array(1);

        // initialize the allocated value
        // safety: pointer addresses one aligned writable T slot
        unsafe {
            pointer.as_ptr().write(value);
            &*pointer.as_ptr()
        }
    }

    /// Copy one string into this allocator.
    pub fn alloc_str<'a>(&'a self, text: &str) -> &'a str {
        // allocate stable string storage
        let target = self.allocate(text.len(), align_of::<u8>());

        // copy the source into stable arena storage
        // safety: target spans text.len() writable bytes and text is valid UTF-8
        unsafe {
            copy_nonoverlapping(text.as_ptr(), target.as_ptr(), text.len());
            let bytes = std::slice::from_raw_parts(target.as_ptr(), text.len());
            std::str::from_utf8_unchecked(bytes)
        }
    }

    /// Allocate one uninitialized array.
    fn allocate_array<T>(&self, capacity: usize) -> NonNull<T> {
        // represent empty allocations without touching a chunk
        if size_of::<T>() == 0 || capacity == 0 {
            return NonNull::dangling();
        }

        // allocate storage with the element layout
        let bytes = size_of::<T>() * capacity;

        self.allocate(bytes, align_of::<T>()).cast()
    }

    /// Grow one array, extending the most recent allocation in place when possible.
    fn grow_array<T>(
        &self,
        pointer: NonNull<T>,
        length: usize,
        old_capacity: usize,
        new_capacity: usize,
    ) -> NonNull<T> {
        // preserve the canonical pointer for zero-sized elements
        let element_bytes = size_of::<T>();
        if element_bytes == 0 {
            return pointer;
        }

        // calculate the old and requested allocation widths
        let old_bytes = element_bytes * old_capacity;
        let new_bytes = element_bytes * new_capacity;

        // extend the current tail allocation without copying
        if self.grow_current(pointer.cast(), old_bytes, new_bytes) {
            return pointer;
        }

        // move older allocations into fresh arena storage
        let target = self.allocate_array(new_capacity);

        // safety: both arrays hold at least length initialized elements and do not overlap
        unsafe { copy_nonoverlapping(pointer.as_ptr(), target.as_ptr(), length) };

        target
    }

    /// Allocate one aligned byte range.
    #[inline(always)]
    fn allocate(&self, bytes: usize, alignment: usize) -> NonNull<u8> {
        // represent empty allocations without touching a chunk
        if bytes == 0 {
            return NonNull::dangling();
        }

        // serve the predictable fast path from the current chunk
        if let Some(pointer) = self.allocate_current(bytes, alignment) {
            return pointer;
        }

        // enter the cold chunk allocation path
        self.allocate_chunk(bytes, alignment)
    }

    /// Allocate from the current chunk when capacity remains.
    #[inline(always)]
    fn allocate_current(&self, bytes: usize, alignment: usize) -> Option<NonNull<u8>> {
        // load the current bump cursor
        let mut cursor = self.current.get()?;

        // align the next allocation start
        let address = cursor.pointer.as_ptr() as usize;
        let padding = alignment.wrapping_sub(address & (alignment - 1)) & (alignment - 1);
        let required = padding + bytes;
        let remaining = cursor.end.as_ptr() as usize - address;

        // leave chunk growth to the cold path
        if required > remaining {
            return None;
        }

        // advance the cursor past the allocation
        // safety: the capacity check proves both pointers remain in the current chunk
        let pointer = unsafe { NonNull::new_unchecked(cursor.pointer.as_ptr().add(padding)) };
        cursor.pointer = unsafe { NonNull::new_unchecked(pointer.as_ptr().add(bytes)) };
        self.current.set(Some(cursor));

        Some(pointer)
    }

    /// Extend the most recent allocation when capacity remains.
    #[inline(always)]
    fn grow_current(&self, pointer: NonNull<u8>, old_bytes: usize, new_bytes: usize) -> bool {
        // reject the canonical dangling pointer for empty vectors
        if old_bytes == 0 {
            return false;
        }

        // require one active chunk
        let Some(mut cursor) = self.current.get() else {
            return false;
        };

        // require the vector to be the most recent allocation
        // safety: pointer addresses old_bytes allocated bytes
        let pointer_end = unsafe { pointer.as_ptr().add(old_bytes) };
        if pointer_end != cursor.pointer.as_ptr() {
            return false;
        }

        // require sufficient remaining chunk capacity
        let remaining = cursor.end.as_ptr() as usize - pointer.as_ptr() as usize;
        if new_bytes > remaining {
            return false;
        }

        // extend the allocation and publish the advanced cursor
        // safety: the capacity check keeps the cursor inside the current chunk
        cursor.pointer = unsafe { NonNull::new_unchecked(pointer.as_ptr().add(new_bytes)) };
        self.current.set(Some(cursor));

        true
    }

    /// Allocate and enter one geometrically larger chunk.
    #[cold]
    #[inline(never)]
    fn allocate_chunk(&self, bytes: usize, alignment: usize) -> NonNull<u8> {
        // read the previous chunk size
        // safety: formatting is single threaded, so chunk ownership cannot race
        let chunks = unsafe { &mut *self.chunks.get() };
        let previous_capacity = chunks.last().map_or(0, |chunk| chunk.capacity);

        // choose geometric, page-aligned chunk capacity
        let requested_capacity = bytes + alignment;
        let capacity = CHUNK_BYTES
            .max(previous_capacity * 2)
            .max(requested_capacity);
        let capacity = (capacity + PAGE_BYTES - 1) & !(PAGE_BYTES - 1);

        // allocate stable storage for the new chunk
        let chunk_alignment = CHUNK_ALIGNMENT.max(alignment);
        let chunk = Chunk::new(capacity, chunk_alignment);
        let start = chunk.pointer;

        // initialize the cursor after the first allocation
        // safety: capacity includes the requested allocation and end address
        let end = unsafe { NonNull::new_unchecked(start.as_ptr().add(capacity)) };
        let pointer = unsafe { NonNull::new_unchecked(start.as_ptr().add(bytes)) };
        chunks.push(chunk);
        self.current.set(Some(Cursor { pointer, end }));

        start
    }
}

impl Default for Allocator {
    fn default() -> Self {
        Self {
            current: Cell::new(None),
            chunks: UnsafeCell::new(std::vec::Vec::new()),
        }
    }
}

/// The unallocated range in the current chunk.
#[derive(Clone, Copy)]
struct Cursor {
    /// The first unallocated byte.
    pointer: NonNull<u8>,
    /// The byte after the current chunk.
    end: NonNull<u8>,
}

/// One stable allocation chunk.
struct Chunk {
    /// The first byte in this chunk.
    pointer: NonNull<u8>,
    /// The allocated byte count.
    capacity: usize,
    /// The allocation alignment.
    alignment: usize,
}

impl Chunk {
    /// Allocate one chunk.
    fn new(capacity: usize, alignment: usize) -> Self {
        // construct the proven chunk layout
        // safety: capacity is nonzero and alignment is a power of two
        let layout = unsafe { Layout::from_size_align_unchecked(capacity, alignment) };

        // allocate or abort through the standard allocation failure path
        // safety: layout is valid for the global allocator
        let pointer = unsafe { alloc(layout) };
        let Some(pointer) = NonNull::new(pointer) else {
            handle_alloc_error(layout);
        };

        // retain the exact layout for bulk destruction
        Self {
            pointer,
            capacity,
            alignment,
        }
    }
}

impl Drop for Chunk {
    fn drop(&mut self) {
        // reconstruct the original allocation layout
        // safety: capacity and alignment are unchanged from Chunk::new
        let layout = unsafe { Layout::from_size_align_unchecked(self.capacity, self.alignment) };

        // release the complete chunk
        // safety: pointer and layout identify the live allocation from Chunk::new
        unsafe { dealloc(self.pointer.as_ptr(), layout) };
    }
}

/// A growable vector allocated for one formatting pass.
///
/// Element types are statically required to need no individual destruction.
pub(crate) struct ArenaVec<'a, T> {
    /// The first element allocation.
    pointer: NonNull<T>,
    /// The initialized element count.
    length: usize,
    /// The allocated element count.
    capacity: usize,
    /// The allocator that owns this vector's storage.
    allocator: &'a Allocator,
    /// The owned element type.
    elements: PhantomData<T>,
}

impl<'a, T> ArenaVec<'a, T> {
    /// Compile-time rejection for values requiring destruction.
    const ASSERT_T_IS_NOT_DROP: () = assert!(!std::mem::needs_drop::<T>());
    /// The first allocation capacity for this element width.
    const INITIAL_CAPACITY: usize = if size_of::<T>() == 1 {
        8
    } else if size_of::<T>() <= 1024 {
        4
    } else {
        1
    };

    /// Create an empty vector.
    pub(crate) fn new_in(allocator: &'a Allocator) -> Self {
        // reject values that require individual destruction
        const { Self::ASSERT_T_IS_NOT_DROP };

        // construct the canonical unallocated vector
        Self {
            pointer: NonNull::dangling(),
            length: 0,
            capacity: 0,
            allocator,
            elements: PhantomData,
        }
    }

    /// Create an empty vector with capacity for `capacity` elements.
    pub(crate) fn with_capacity_in(capacity: usize, allocator: &'a Allocator) -> Self {
        // reject values that require individual destruction
        const { Self::ASSERT_T_IS_NOT_DROP };

        // allocate the requested element storage
        let pointer = allocator.allocate_array(capacity);

        // construct an empty initialized range
        Self {
            pointer,
            length: 0,
            capacity,
            allocator,
            elements: PhantomData,
        }
    }

    /// Append one element.
    pub(crate) fn push(&mut self, value: T) {
        // grow a full element allocation
        if self.length == self.capacity {
            self.grow();
        }

        // initialize the next element
        // safety: capacity exceeds length after optional growth
        unsafe { self.pointer.as_ptr().add(self.length).write(value) };
        self.length += 1;
    }

    /// Remove and return the final value.
    pub(crate) fn pop(&mut self) -> Option<T> {
        if self.length == 0 {
            return None;
        }

        // move the initialized final value out of the vector
        self.length -= 1;

        // safety: the decremented length selects the previous final value
        unsafe { Some(self.pointer.as_ptr().add(self.length).read()) }
    }

    /// Convert this vector into its stable arena slice.
    pub(crate) fn into_slice(self) -> &'a [T] {
        // safety: pointer and length describe the initialized arena allocation
        unsafe { std::slice::from_raw_parts(self.pointer.as_ptr(), self.length) }
    }

    /// Return the allocator that owns this vector.
    pub(crate) const fn allocator(&self) -> &'a Allocator {
        self.allocator
    }

    /// Double this vector's capacity.
    fn grow(&mut self) {
        // establish a useful first allocation, then grow geometrically
        let capacity = if self.capacity == 0 {
            Self::INITIAL_CAPACITY
        } else {
            self.capacity * 2
        };

        self.reallocate(capacity);
    }

    /// Move elements into one larger arena allocation.
    fn reallocate(&mut self, capacity: usize) {
        // extend the tail allocation or copy into fresh arena storage
        self.pointer =
            self.allocator
                .grow_array(self.pointer, self.length, self.capacity, capacity);

        // publish the new capacity after successful allocation
        self.capacity = capacity;
    }
}

impl<T> Deref for ArenaVec<'_, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        // safety: pointer and length describe the initialized element range
        unsafe { std::slice::from_raw_parts(self.pointer.as_ptr(), self.length) }
    }
}

impl<T> DerefMut for ArenaVec<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // safety: mutable vector access uniquely borrows its initialized range
        unsafe { std::slice::from_raw_parts_mut(self.pointer.as_ptr(), self.length) }
    }
}

impl<T> Debug for ArenaVec<'_, T>
where
    T: Debug,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(formatter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One value requiring more than the default chunk alignment.
    #[repr(align(128))]
    struct Aligned(
        /// The test payload.
        u8,
    );

    /// Allocate aligned values across multiple chunks.
    #[test]
    fn test_allocates_aligned_values() {
        let allocator = Allocator::default();
        let mut values = ArenaVec::with_capacity_in(1, &allocator);
        values.push(0_u64);
        let first_pointer = values.as_ptr();

        // grow the most recent allocation without moving it inside one chunk
        for value in 0..100_u64 {
            values.push(value);
        }

        assert_eq!(values.as_ptr(), first_pointer);

        // retain values while growing across later chunks
        for value in 100..20_000_u64 {
            values.push(value);
        }

        assert_eq!(values.len(), 20_001);
        assert_eq!(values[20_000], 19_999);
        assert_eq!(values.as_ptr().align_offset(align_of::<u64>()), 0);

        // honor alignment above the default chunk alignment
        let mut aligned = ArenaVec::new_in(&allocator);
        aligned.push(Aligned(1));
        assert_eq!(aligned.as_ptr().align_offset(align_of::<Aligned>()), 0);
        assert_eq!(aligned[0].0, 1);
    }

    /// Keep copied strings stable across later allocations.
    #[test]
    fn test_allocates_stable_strings() {
        let allocator = Allocator::default();
        let first = allocator.alloc_str("first");
        let _large = allocator.alloc_str(&"x".repeat(CHUNK_BYTES * 2));

        assert_eq!(first, "first");
    }

    /// Move an older vector allocation without losing initialized values.
    #[test]
    fn test_grows_interleaved_vector() {
        let allocator = Allocator::default();
        let mut values = ArenaVec::with_capacity_in(1, &allocator);
        values.push(1_u64);
        let first_pointer = values.as_ptr();

        // place another allocation after the vector
        let separator = allocator.alloc_str("separator");

        // force the vector to move to the current chunk tail
        values.push(2);

        assert_ne!(values.as_ptr(), first_pointer);
        assert_eq!(&*values, &[1, 2]);
        assert_eq!(separator, "separator");
    }
}
