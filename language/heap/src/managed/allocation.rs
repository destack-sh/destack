use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::page::{PageSlot, RetainedBytes};
use crate::value::Value;

/// The target inline payload size for one managed allocation.
///
/// 64 bytes keeps the common small aggregate case inline while preserving a
/// compact allocation shape before managed span storage takes over.
const INLINE_ALLOCATION_TARGET_BYTES: usize = 64;

/// The number of values kept inline in one managed allocation.
pub(crate) const INLINE_ALLOCATION_VALUES: usize =
    INLINE_ALLOCATION_TARGET_BYTES / std::mem::size_of::<Value>();

/// One stable large managed span identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ManagedLargeSpanId(u64);

impl ManagedLargeSpanId {
    /// Create one large managed span identifier.
    pub(crate) const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Return the large managed span identifier value.
    pub(crate) const fn id(self) -> u64 {
        self.0
    }
}

/// The storage class for one managed span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ManagedSpanClass {
    /// One page-backed managed span.
    Paged,
    /// One dedicated large managed span.
    Large,
}

/// The storage class for one managed span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ManagedSpanStorage {
    /// One page-backed managed span.
    Paged {
        /// The first page slot in this span.
        start: PageSlot,
    },
    /// One dedicated large managed span.
    Large {
        /// The stable large-span identifier.
        id: ManagedLargeSpanId,
    },
}

/// One stable managed span of external values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedSpan {
    /// The storage backing this span.
    storage: ManagedSpanStorage,
    /// The number of values in this span.
    len: u32,
}

impl ManagedSpan {
    /// Create one page-backed managed span.
    pub fn new(start: PageSlot, len: usize) -> Self {
        Self {
            storage: ManagedSpanStorage::Paged { start },
            len: len as u32,
        }
    }

    /// Create one dedicated large managed span.
    pub(crate) fn large(id: ManagedLargeSpanId, len: usize) -> Self {
        Self {
            storage: ManagedSpanStorage::Large { id },
            len: len as u32,
        }
    }

    /// Return the first managed span location when this span is page-backed.
    #[inline]
    pub fn start(&self) -> Option<PageSlot> {
        match self.storage {
            ManagedSpanStorage::Paged { start } => Some(start),
            ManagedSpanStorage::Large { .. } => None,
        }
    }

    /// Return the large managed span identifier when this span is dedicated.
    #[inline]
    pub(crate) fn large_span_id(&self) -> Option<ManagedLargeSpanId> {
        match self.storage {
            ManagedSpanStorage::Paged { .. } => None,
            ManagedSpanStorage::Large { id } => Some(id),
        }
    }

    /// Report whether this span is large.
    #[inline]
    pub fn is_large(&self) -> bool {
        matches!(self.storage, ManagedSpanStorage::Large { .. })
    }

    /// Return the number of values in this span.
    #[inline]
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Report whether this span is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// One dedicated large managed span.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ManagedLargeSpan {
    /// The contiguous value storage for this span.
    storage: ManagedLargeSpanStorage,
}

/// The backing storage for one large managed span.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
enum ManagedLargeSpanStorage {
    /// Owned mutable values local to one live heap.
    Owned(Box<[Value]>),
    /// Immutable values shared through forked images.
    Shared(Arc<[Value]>),
}

impl Clone for ManagedLargeSpan {
    fn clone(&self) -> Self {
        match &self.storage {
            ManagedLargeSpanStorage::Owned(values) => Self {
                storage: ManagedLargeSpanStorage::Owned(values.clone()),
            },
            ManagedLargeSpanStorage::Shared(values) => Self {
                storage: ManagedLargeSpanStorage::Shared(values.clone()),
            },
        }
    }
}

impl ManagedLargeSpan {
    /// Create one large managed span from the given values.
    pub(crate) fn new(values: &[Value]) -> Self {
        Self {
            storage: ManagedLargeSpanStorage::Owned(values.to_vec().into_boxed_slice()),
        }
    }

    /// Return the span values as one slice.
    pub(crate) fn as_slice(&self) -> &[Value] {
        match &self.storage {
            ManagedLargeSpanStorage::Owned(values) => values.as_ref(),
            ManagedLargeSpanStorage::Shared(values) => values.as_ref(),
        }
    }

    /// Return one value by index.
    pub(crate) fn get(&self, index: usize) -> Option<&Value> {
        self.as_slice().get(index)
    }

    /// Return one mutable value by index.
    pub(crate) fn get_mut(&mut self, index: usize) -> Option<&mut Value> {
        self.values_mut().get_mut(index)
    }

    // convert one owned span payload into shared immutable backing
    fn share(&mut self) {
        if let ManagedLargeSpanStorage::Owned(values) = &mut self.storage {
            let values = std::mem::take(values);
            self.storage = ManagedLargeSpanStorage::Shared(Arc::from(values));
        }
    }

    /// Capture one immutable image for this large managed span.
    pub(crate) fn image(&mut self) -> Arc<[Value]> {
        self.share();

        match &self.storage {
            ManagedLargeSpanStorage::Owned(_) => unreachable!(),
            ManagedLargeSpanStorage::Shared(values) => values.clone(),
        }
    }

    /// Restore one large managed span from one immutable image.
    pub(crate) fn from_image(values: Arc<[Value]>) -> Self {
        Self {
            storage: ManagedLargeSpanStorage::Shared(values),
        }
    }

    /// Replace the entire value payload.
    pub(crate) fn replace(&mut self, values: &[Value]) {
        self.storage = ManagedLargeSpanStorage::Owned(values.to_vec().into_boxed_slice());
    }

    /// Resize the payload and fill new values when it grows.
    pub(crate) fn resize(&mut self, len: usize, value: Value) {
        let mut values = self.as_slice().to_vec();
        values.resize(len, value);
        self.storage = ManagedLargeSpanStorage::Owned(values.into_boxed_slice());
    }

    // return mutable value storage, detaching shared backing on first write
    pub(crate) fn values_mut(&mut self) -> &mut [Value] {
        if let ManagedLargeSpanStorage::Shared(values) = &self.storage {
            self.storage =
                ManagedLargeSpanStorage::Owned(values.as_ref().to_vec().into_boxed_slice());
        }

        match &mut self.storage {
            ManagedLargeSpanStorage::Owned(values) => values.as_mut(),
            ManagedLargeSpanStorage::Shared(_) => unreachable!(),
        }
    }

    /// Return the retained heap bytes owned by this large span outside its inline form.
    pub(crate) fn retained_bytes(&self) -> usize {
        match &self.storage {
            ManagedLargeSpanStorage::Owned(values) => std::mem::size_of_val(values.as_ref()),
            ManagedLargeSpanStorage::Shared(values) => std::mem::size_of_val(values.as_ref()),
        }
    }
}

/// One stable managed allocation record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManagedAllocation {
    /// One inline allocation.
    Inline {
        /// The number of populated inline values.
        len: u8,
        /// The inline value payload.
        values: [Value; INLINE_ALLOCATION_VALUES],
    },
    /// One allocation with external span storage.
    External {
        /// The number of values stored in the external span.
        len: u32,
        /// The external managed span for this allocation.
        span: ManagedSpan,
    },
}

impl Default for ManagedAllocation {
    fn default() -> Self {
        Self::new()
    }
}

impl ManagedAllocation {
    /// Report whether one value count fits inline.
    #[inline]
    pub fn can_inline(len: usize) -> bool {
        len <= INLINE_ALLOCATION_VALUES
    }

    /// Return the logical live bytes for this allocation.
    pub fn logical_bytes(&self) -> u64 {
        let allocation_bytes = std::mem::size_of::<Self>() as u64;

        match self {
            Self::Inline { .. } => allocation_bytes,
            Self::External { len, .. } => {
                allocation_bytes.saturating_add((*len as u64) * std::mem::size_of::<Value>() as u64)
            }
        }
    }

    /// Create a new empty allocation.
    pub fn new() -> Self {
        Self::Inline {
            len: 0,
            values: [Value::VOID; INLINE_ALLOCATION_VALUES],
        }
    }

    /// Create one inline allocation with the given number of values.
    pub fn with_values_len(count: usize) -> Self {
        debug_assert!(
            count <= INLINE_ALLOCATION_VALUES,
            "inline allocation too large"
        );

        Self::Inline {
            len: count as u8,
            values: [Value::VOID; INLINE_ALLOCATION_VALUES],
        }
    }

    /// Create one inline allocation from one list of values.
    pub fn with_values(values: &[Value]) -> Self {
        debug_assert!(
            values.len() <= INLINE_ALLOCATION_VALUES,
            "inline allocation too large"
        );

        let mut inline_values = [Value::VOID; INLINE_ALLOCATION_VALUES];

        // copy inline values
        for (index, value) in values.iter().copied().enumerate() {
            inline_values[index] = value;
        }

        Self::Inline {
            len: values.len() as u8,
            values: inline_values,
        }
    }

    /// Create an allocation with exactly 2 values.
    #[inline]
    pub fn with_pair(first: Value, second: Value) -> Self {
        Self::with_values(&[first, second])
    }

    /// Create an allocation with exactly 1 value.
    #[inline]
    pub fn with_single(value: Value) -> Self {
        Self::with_values(&[value])
    }

    /// Create one externally backed allocation.
    #[inline]
    pub fn with_span(span: ManagedSpan) -> Self {
        Self::External {
            len: span.len() as u32,
            span,
        }
    }

    /// Return the number of values in this allocation.
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            Self::Inline { len, .. } => *len as usize,
            Self::External { len, .. } => *len as usize,
        }
    }

    /// Report whether this allocation has no values.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return the inline values when this allocation is stored inline.
    #[inline]
    pub fn inline_values(&self) -> Option<&[Value]> {
        match self {
            Self::Inline { len, values } => Some(&values[..*len as usize]),
            Self::External { .. } => None,
        }
    }

    /// Return the mutable inline values when this allocation is stored inline.
    #[inline]
    pub fn inline_values_mut(&mut self) -> Option<&mut [Value]> {
        match self {
            Self::Inline { len, values } => Some(&mut values[..*len as usize]),
            Self::External { .. } => None,
        }
    }

    /// Return the external managed span when this allocation uses external storage.
    #[inline]
    pub fn span(&self) -> Option<ManagedSpan> {
        match self {
            Self::Inline { .. } => None,
            Self::External { span, .. } => Some(*span),
        }
    }

    /// Return one inline value by index when this allocation is stored inline.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&Value> {
        let values = self.inline_values()?;
        values.get(index)
    }

    /// Return one mutable inline value by index when this allocation is stored inline.
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Value> {
        let values = self.inline_values_mut()?;
        values.get_mut(index)
    }

    /// Resize one inline allocation.
    pub fn resize_inline(&mut self, new_len: usize, value: Value) {
        debug_assert!(
            Self::can_inline(new_len),
            "inline resize requires one inline sized allocation"
        );

        let old_values = self
            .inline_values()
            .expect("inline resize requires one inline allocation");
        let mut new_values = [Value::VOID; INLINE_ALLOCATION_VALUES];

        // preserve existing inline values
        for (index, slot) in old_values.iter().copied().enumerate() {
            new_values[index] = slot;
        }

        // initialize newly visible inline values
        if new_len > old_values.len() {
            new_values[old_values.len()..new_len].fill(value);
        }

        // install the resized inline shape
        *self = Self::Inline {
            len: new_len as u8,
            values: new_values,
        };
    }

    /// Append one value to one inline allocation.
    pub fn push_inline(&mut self, value: Value) {
        let len = self.len();
        self.resize_inline(len + 1, Value::VOID);

        let slot = self
            .get_mut(len)
            .expect("pushed inline value should be addressable");
        *slot = value;
    }

    /// Return one inline value without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the index is in bounds and this allocation is inline.
    #[inline(always)]
    pub unsafe fn get_unchecked(&self, index: usize) -> &Value {
        let values = self
            .inline_values()
            .expect("inline unchecked access requires one inline allocation");

        unsafe { values.get_unchecked(index) }
    }

    /// Return one mutable inline value without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the index is in bounds and this allocation is inline.
    #[inline(always)]
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut Value {
        let values = self
            .inline_values_mut()
            .expect("inline unchecked access requires one inline allocation");

        unsafe { values.get_unchecked_mut(index) }
    }
}

impl RetainedBytes for ManagedAllocation {}

impl<'a> IntoIterator for &'a ManagedAllocation {
    type Item = &'a Value;
    type IntoIter = std::slice::Iter<'a, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.inline_values()
            .expect("managed allocation iteration requires one inline allocation")
            .iter()
    }
}
