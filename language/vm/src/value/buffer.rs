use serde::{Deserialize, Serialize};

use super::Value;

/// The target inline payload size for one value buffer.
///
/// 32 bytes keeps small local and raw value buffers inline without making the
/// buffer disproportionately large in interpreter hot paths.
/// This is a representation budget, not a runtime tuning option.
const INLINE_VALUE_BUFFER_TARGET_BYTES: usize = 32;

/// The number of values kept inline in one value buffer.
const INLINE_VALUE_BUFFER_VALUES: usize =
    INLINE_VALUE_BUFFER_TARGET_BYTES / std::mem::size_of::<Value>();

/// One inline-optimized local value buffer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueBuffer {
    /// Inline storage for small value counts.
    Inline {
        /// Number of values currently in use.
        len: u8,
        /// Inline value buffer.
        values: [Value; INLINE_VALUE_BUFFER_VALUES],
    },
    /// Heap-allocated value storage.
    Heap(Box<[Value]>),
}

impl Default for ValueBuffer {
    fn default() -> Self {
        Self::Inline {
            len: 0,
            values: [Value::VOID; INLINE_VALUE_BUFFER_VALUES],
        }
    }
}

impl ValueBuffer {
    /// Create a new empty value buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create one value buffer with the given number of values.
    pub fn with_values_len(count: usize) -> Self {
        if count <= INLINE_VALUE_BUFFER_VALUES {
            Self::Inline {
                len: count as u8,
                values: [Value::VOID; INLINE_VALUE_BUFFER_VALUES],
            }
        } else {
            Self::Heap(vec![Value::VOID; count].into_boxed_slice())
        }
    }

    /// Create one value buffer from one list of values.
    pub fn with_values(values: Vec<Value>) -> Self {
        let len = values.len();

        // inline values
        if len <= INLINE_VALUE_BUFFER_VALUES {
            let mut inline_values = [Value::VOID; INLINE_VALUE_BUFFER_VALUES];
            for (index, value) in values.into_iter().enumerate() {
                inline_values[index] = value;
            }
            Self::Inline {
                len: len as u8,
                values: inline_values,
            }
        }
        // heap values
        else {
            Self::Heap(values.into_boxed_slice())
        }
    }

    /// Create one value buffer with exactly 2 values.
    #[inline]
    pub fn with_pair(first: Value, second: Value) -> Self {
        Self::Inline {
            len: 2,
            values: [first, second],
        }
    }

    /// Create one value buffer with exactly 1 value.
    #[inline]
    pub fn with_single(value: Value) -> Self {
        Self::Inline {
            len: 1,
            values: [value, Value::VOID],
        }
    }

    /// Return the retained heap bytes owned by this value buffer outside its inline form.
    pub fn retained_bytes(&self) -> usize {
        match self {
            Self::Inline { .. } => 0,
            Self::Heap(values) => std::mem::size_of_val(values.as_ref()),
        }
    }

    /// Return the number of values in this value buffer.
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            Self::Inline { len, .. } => *len as usize,
            Self::Heap(values) => values.len(),
        }
    }

    /// Report whether this value buffer has no values.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return the value slice for this value buffer.
    #[inline]
    pub fn as_slice(&self) -> &[Value] {
        match self {
            Self::Inline { len, values } => &values[..*len as usize],
            Self::Heap(values) => values.as_ref(),
        }
    }

    /// Return one value by index.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.as_slice().get(index)
    }

    /// Return one mutable value by index.
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Value> {
        match self {
            // inline values
            Self::Inline { len, values } => {
                if index < *len as usize {
                    Some(&mut values[index])
                } else {
                    None
                }
            }

            // heap values
            Self::Heap(values) => values.get_mut(index),
        }
    }

    /// Return one value without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the index is within the current bounds.
    #[inline(always)]
    pub unsafe fn get_unchecked(&self, index: usize) -> &Value {
        match self {
            Self::Inline { len, values } => {
                debug_assert!(index < *len as usize, "inline value out of bounds");
                unsafe { values.get_unchecked(index) }
            }
            Self::Heap(values) => unsafe { values.get_unchecked(index) },
        }
    }

    /// Return one mutable value without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the index is within the current bounds.
    #[inline(always)]
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut Value {
        match self {
            Self::Inline { len, values } => {
                debug_assert!(index < *len as usize, "inline value out of bounds");
                unsafe { values.get_unchecked_mut(index) }
            }
            Self::Heap(values) => unsafe { values.get_unchecked_mut(index) },
        }
    }

    /// Return the inline values when this value buffer is stored inline.
    #[inline]
    pub fn inline_values(&self) -> Option<&[Value]> {
        match self {
            Self::Inline { len, values } => Some(&values[..*len as usize]),
            Self::Heap(_) => None,
        }
    }

    /// Return the mutable inline values when this value buffer is stored inline.
    #[inline]
    pub fn inline_values_mut(&mut self) -> Option<&mut [Value]> {
        match self {
            Self::Inline { len, values } => Some(&mut values[..*len as usize]),
            Self::Heap(_) => None,
        }
    }

    /// Resize the local value storage for this value buffer.
    pub fn resize(&mut self, new_len: usize, value: Value) {
        match self {
            // inline values
            Self::Inline { len, values } => {
                let old_len = *len as usize;

                // stay inline
                if new_len <= INLINE_VALUE_BUFFER_VALUES {
                    if new_len > old_len {
                        values[old_len..new_len].fill(value);
                    }
                    *len = new_len as u8;
                    return;
                }

                // spill to heap
                let mut heap_values = Vec::with_capacity(new_len);
                heap_values.extend_from_slice(&values[..old_len]);
                heap_values.resize(new_len, value);
                *self = Self::Heap(heap_values.into_boxed_slice());
            }

            // heap values
            Self::Heap(values) => {
                let mut heap_values = values.to_vec();
                heap_values.resize(new_len, value);
                *values = heap_values.into_boxed_slice();
            }
        }
    }

    /// Append one local value to this value buffer.
    pub fn push(&mut self, value: Value) {
        match self {
            // inline values
            Self::Inline { len, values } => {
                let index = *len as usize;

                // keep the payload inline
                if index < INLINE_VALUE_BUFFER_VALUES {
                    values[index] = value;
                    *len += 1;
                    return;
                }

                // spill to heap
                let mut heap_values = Vec::with_capacity(index + 1);
                heap_values.extend_from_slice(&values[..index]);
                heap_values.push(value);
                *self = Self::Heap(heap_values.into_boxed_slice());
            }

            // heap values
            Self::Heap(values) => {
                let mut heap_values = values.to_vec();
                heap_values.push(value);
                *values = heap_values.into_boxed_slice();
            }
        }
    }

    /// Clone this value buffer for a forked continuation.
    pub fn clone_for_fork(&self) -> Self {
        self.clone()
    }
}

impl<'a> IntoIterator for &'a ValueBuffer {
    type Item = &'a Value;
    type IntoIter = std::slice::Iter<'a, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}
