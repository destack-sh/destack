use crate::value::Value;

const INLINE_SLOT_CAP: usize = 2;

/// Slot storage for heap cells, optimized for small fixed layouts.
#[derive(Debug, Clone)]
pub enum SlotStorage {
    /// Inline storage for small slot counts.
    Inline {
        /// Number of slots currently in use.
        len: u8,
        /// Inline slot buffer.
        slots: [Value; INLINE_SLOT_CAP],
    },
    /// Heap-allocated slot storage.
    Heap(Vec<Value>),
}

impl Default for SlotStorage {
    fn default() -> Self {
        Self::Inline {
            len: 0,
            slots: [Value::VOID; INLINE_SLOT_CAP],
        }
    }
}

impl SlotStorage {
    /// Create inline storage with the given slot count.
    pub fn with_slots(count: usize) -> Self {
        if count <= INLINE_SLOT_CAP {
            Self::Inline {
                len: count as u8,
                slots: [Value::VOID; INLINE_SLOT_CAP],
            }
        } else {
            Self::Heap(vec![Value::VOID; count])
        }
    }

    /// Create slot storage from a list of values.
    pub fn from_values(values: Vec<Value>) -> Self {
        let len = values.len();
        if len <= INLINE_SLOT_CAP {
            let mut slots = [Value::VOID; INLINE_SLOT_CAP];
            for (index, value) in values.into_iter().enumerate() {
                slots[index] = value;
            }
            Self::Inline {
                len: len as u8,
                slots,
            }
        } else {
            Self::Heap(values)
        }
    }

    /// Create slot storage for exactly 2 values (avoids Vec allocation).
    #[inline]
    pub fn from_pair(first: Value, second: Value) -> Self {
        Self::Inline {
            len: 2,
            slots: [first, second],
        }
    }

    /// Create slot storage for exactly 1 value (avoids Vec allocation).
    #[inline]
    pub fn from_single(value: Value) -> Self {
        Self::Inline {
            len: 1,
            slots: [value, Value::VOID],
        }
    }

    /// Return the number of slots.
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            Self::Inline { len, .. } => *len as usize,
            Self::Heap(values) => values.len(),
        }
    }

    /// Report whether this storage is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return the slot slice.
    #[inline]
    pub fn as_slice(&self) -> &[Value] {
        match self {
            Self::Inline { len, slots } => &slots[..*len as usize],
            Self::Heap(values) => values.as_slice(),
        }
    }

    /// Return a slot by index.
    #[inline(always)]
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.as_slice().get(index)
    }

    /// Return a mutable slot by index.
    #[inline(always)]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Value> {
        match self {
            Self::Inline { len, slots } => {
                if index < *len as usize {
                    Some(&mut slots[index])
                } else {
                    None
                }
            }
            Self::Heap(values) => values.get_mut(index),
        }
    }

    /// Return a slot without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the index is within the current slot bounds.
    #[inline(always)]
    pub unsafe fn get_unchecked(&self, index: usize) -> &Value {
        match self {
            Self::Inline { len, slots } => {
                debug_assert!(index < *len as usize, "inline slot out of bounds");
                unsafe { slots.get_unchecked(index) }
            }
            Self::Heap(values) => unsafe { values.get_unchecked(index) },
        }
    }

    /// Return a mutable slot without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the index is within the current slot bounds.
    #[inline(always)]
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut Value {
        match self {
            Self::Inline { len, slots } => {
                debug_assert!(index < *len as usize, "inline slot out of bounds");
                unsafe { slots.get_unchecked_mut(index) }
            }
            Self::Heap(values) => unsafe { values.get_unchecked_mut(index) },
        }
    }

    /// Resize the slot storage.
    pub fn resize(&mut self, new_len: usize, value: Value) {
        match self {
            Self::Inline { len, slots } => {
                let old_len = *len as usize;
                if new_len <= INLINE_SLOT_CAP {
                    if new_len > old_len {
                        slots[old_len..new_len].fill(value);
                    }
                    *len = new_len as u8;
                    return;
                }

                let mut values = Vec::with_capacity(new_len);
                values.extend_from_slice(&slots[..old_len]);
                values.resize(new_len, value);
                *self = Self::Heap(values);
            }
            Self::Heap(values) => {
                values.resize(new_len, value);
            }
        }
    }

    /// Append a slot to the storage.
    pub fn push(&mut self, value: Value) {
        match self {
            Self::Inline { len, slots } => {
                let index = *len as usize;
                if index < INLINE_SLOT_CAP {
                    slots[index] = value;
                    *len += 1;
                    return;
                }

                let mut values = Vec::with_capacity(index + 1);
                values.extend_from_slice(&slots[..index]);
                values.push(value);
                *self = Self::Heap(values);
            }
            Self::Heap(values) => {
                values.push(value);
            }
        }
    }
}

impl<'a> IntoIterator for &'a SlotStorage {
    type Item = &'a Value;
    type IntoIter = std::slice::Iter<'a, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

/// A cell on the heap (unit of allocation).
#[derive(Debug, Default)]
pub struct HeapCell {
    /// The cell's slots (for structs/tuples) or elements (for arrays).
    pub slots: SlotStorage,
    /// Whether this cell has been marked (for GC).
    pub marked: bool,
}

impl HeapCell {
    /// Create a new empty cell.
    pub fn new() -> Self {
        Self {
            slots: SlotStorage::default(),
            marked: false,
        }
    }

    /// Create a cell with the given number of slots (initialized to Void).
    pub fn with_slots(count: usize) -> Self {
        Self {
            slots: SlotStorage::with_slots(count),
            marked: false,
        }
    }

    /// Clone this cell for a forked continuation.
    pub fn clone_for_fork(&self) -> Self {
        Self {
            slots: self.slots.clone(),
            marked: false,
        }
    }
}
