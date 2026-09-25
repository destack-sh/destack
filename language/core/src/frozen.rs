use std::alloc::{Layout, alloc, dealloc, handle_alloc_error};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::mem::size_of;
use std::ptr::{drop_in_place, null_mut};
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

use parking_lot::Mutex;
use serde::de::{Deserializer, SeqAccess, Visitor};
use serde::ser::{SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};
use tspp_serde::{Reflect, Schema, Type};

/// The element count of the first chunk; each later chunk doubles it.
const FIRST_CHUNK_LEN: usize = 64;
/// The number of chunks, enough for `FIRST_CHUNK_LEN << (CHUNK_COUNT - 1)` elements.
const CHUNK_COUNT: usize = 26;

/// Append-only storage whose elements never move, read concurrently while it grows.
pub struct FrozenArena<T> {
    /// The chunk pointers, published once allocated.
    chunks: [AtomicPtr<T>; CHUNK_COUNT],
    /// The number of initialized elements.
    len: AtomicUsize,
    /// The append lock.
    append: Mutex<()>,
}

// SAFETY: elements are written once under the append lock before their index is published,
// and only read afterwards
unsafe impl<T: Send> Send for FrozenArena<T> {}
unsafe impl<T: Send + Sync> Sync for FrozenArena<T> {}

impl<T> FrozenArena<T> {
    /// Create an empty arena.
    pub fn new() -> Self {
        Self {
            chunks: std::array::from_fn(|_| AtomicPtr::new(null_mut())),
            len: AtomicUsize::new(0),
            append: Mutex::new(()),
        }
    }

    /// Append one element and return its index.
    pub fn push(&self, element: T) -> u32 {
        // reserve the next index under the append lock
        let _append = self.append.lock();
        let index = self.len.load(Ordering::Relaxed);
        let (chunk, offset) = Self::locate(index);

        // allocate the chunk on its first element
        let mut pointer = self.chunks[chunk].load(Ordering::Acquire);
        if pointer.is_null() {
            let layout = Self::chunk_layout(chunk);
            // SAFETY: the layout has the nonzero size of a chunk of T
            pointer = unsafe { alloc(layout) }.cast::<T>();
            if pointer.is_null() {
                handle_alloc_error(layout);
            }
            self.chunks[chunk].store(pointer, Ordering::Release);
        }

        // write the element before publishing its index
        // SAFETY: the offset lies inside the allocated chunk and its element is uninitialized
        unsafe { pointer.add(offset).write(element) };
        self.len.store(index + 1, Ordering::Release);

        index as u32
    }

    /// Return one element when its index is published.
    #[inline]
    pub fn get(&self, index: u32) -> Option<&T> {
        let index = index as usize;
        if index >= self.len.load(Ordering::Acquire) {
            return None;
        }
        let (chunk, offset) = Self::locate(index);
        let pointer = self.chunks[chunk].load(Ordering::Acquire);

        // SAFETY: a published index names an initialized element that never moves
        Some(unsafe { &*pointer.add(offset) })
    }

    /// Return the number of elements.
    #[inline]
    pub fn len(&self) -> usize {
        self.len.load(Ordering::Acquire)
    }

    /// Return whether the arena is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Iterate the elements in index order.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        (0..self.len() as u32).map(|index| {
            self.get(index)
                .unwrap_or_else(|| unreachable!("published arena index is initialized"))
        })
    }

    /// Return the chunk and offset holding one index.
    #[inline]
    fn locate(index: usize) -> (usize, usize) {
        let position = index / FIRST_CHUNK_LEN + 1;
        let chunk = (usize::BITS - 1 - position.leading_zeros()) as usize;
        let base = FIRST_CHUNK_LEN * ((1 << chunk) - 1);

        (chunk, index - base)
    }

    /// Return the allocation layout of one chunk.
    fn chunk_layout(chunk: usize) -> Layout {
        Layout::array::<T>(FIRST_CHUNK_LEN << chunk)
            .unwrap_or_else(|_| unreachable!("arena chunk layout overflows"))
    }
}

impl<T> Drop for FrozenArena<T> {
    fn drop(&mut self) {
        // drop every initialized element
        let len = self.len.load(Ordering::Acquire);
        for index in 0..len {
            let (chunk, offset) = Self::locate(index);
            let pointer = self.chunks[chunk].load(Ordering::Acquire);
            // SAFETY: every index below len is initialized and dropped once here
            unsafe { drop_in_place(pointer.add(offset)) };
        }

        // free every allocated chunk
        for (chunk, stored) in self.chunks.iter().enumerate() {
            let pointer = stored.load(Ordering::Acquire);
            if pointer.is_null() {
                continue;
            }
            // SAFETY: the chunk was allocated with this layout and its elements are dropped
            unsafe { dealloc(pointer.cast(), Self::chunk_layout(chunk)) };
        }
    }
}

impl<T> Default for FrozenArena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Clone for FrozenArena<T> {
    fn clone(&self) -> Self {
        let arena = Self::new();
        for element in self.iter() {
            arena.push(element.clone());
        }

        arena
    }
}

impl<T: Debug> Debug for FrozenArena<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T: Serialize> Serialize for FrozenArena<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.len()))?;
        for element in self.iter() {
            sequence.serialize_element(element)?;
        }

        sequence.end()
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for FrozenArena<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_seq(ArenaVisitor(PhantomData))
    }
}

/// Sequence visitor filling one arena.
struct ArenaVisitor<T>(PhantomData<fn() -> T>);

impl<'de, T: Deserialize<'de>> Visitor<'de> for ArenaVisitor<T> {
    type Value = FrozenArena<T>;

    fn expecting(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("a sequence of arena elements")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let arena = FrozenArena::new();
        while let Some(element) = sequence.next_element()? {
            arena.push(element);
        }

        Ok(arena)
    }
}

impl<T: Reflect> Reflect for FrozenArena<T> {
    fn reflect(schema: &mut Schema) -> Type {
        Type::Sequence(Box::new(T::reflect(schema)))
    }
}

const _: () = assert!(size_of::<AtomicPtr<u8>>() == size_of::<usize>());
