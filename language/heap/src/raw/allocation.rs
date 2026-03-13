use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::ValueCell;
use crate::page::RetainedBytes;
use crate::value::Value;

/// One stable raw span identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RawSpanId(u64);

impl RawSpanId {
    /// Create one raw span identifier.
    pub(crate) const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Return the raw span identifier value.
    pub(crate) const fn id(self) -> u64 {
        self.0
    }
}

/// One stable large raw span identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RawLargeSpanId(u64);

impl RawLargeSpanId {
    /// Create one large raw span identifier.
    pub(crate) const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Return the large raw span identifier value.
    pub(crate) const fn id(self) -> u64 {
        self.0
    }
}

/// The storage class for one raw span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum RawSpanClass {
    /// One ordinary raw span.
    Regular,
    /// One dedicated large raw span.
    Large,
}

/// One raw span reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum RawSpanReference {
    /// One ordinary raw span identifier.
    Regular(RawSpanId),
    /// One large raw span identifier.
    Large(RawLargeSpanId),
}

impl RawSpanReference {
    /// Return the storage class for this raw span reference.
    pub(crate) const fn class(self) -> RawSpanClass {
        match self {
            Self::Regular(_) => RawSpanClass::Regular,
            Self::Large(_) => RawSpanClass::Large,
        }
    }
}

/// One contiguous raw span.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RawSpan {
    /// The contiguous byte storage for this span.
    storage: RawSpanStorage,
}

/// The backing storage for one raw span.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
enum RawSpanStorage {
    /// Owned mutable bytes local to one live heap.
    Owned(Box<[u8]>),
    /// Immutable bytes shared through forked images.
    Shared(Arc<[u8]>),
}

impl Clone for RawSpan {
    /// Clone this raw span.
    fn clone(&self) -> Self {
        match &self.storage {
            RawSpanStorage::Owned(bytes) => Self {
                storage: RawSpanStorage::Owned(bytes.clone()),
            },
            RawSpanStorage::Shared(bytes) => Self {
                storage: RawSpanStorage::Shared(bytes.clone()),
            },
        }
    }
}

impl RawSpan {
    /// Create one raw span from the given bytes.
    pub(crate) fn new(bytes: &[u8]) -> Self {
        Self {
            storage: RawSpanStorage::Owned(bytes.to_vec().into_boxed_slice()),
        }
    }

    /// Return the span length in bytes.
    pub(crate) fn len(&self) -> usize {
        match &self.storage {
            RawSpanStorage::Owned(bytes) => bytes.len(),
            RawSpanStorage::Shared(bytes) => bytes.len(),
        }
    }

    /// Return the span bytes as a slice.
    pub(crate) fn as_slice(&self) -> &[u8] {
        match &self.storage {
            RawSpanStorage::Owned(bytes) => bytes.as_ref(),
            RawSpanStorage::Shared(bytes) => bytes.as_ref(),
        }
    }

    /// Return one byte by index.
    pub(crate) fn get(&self, index: usize) -> Option<u8> {
        self.as_slice().get(index).copied()
    }

    /// Convert one owned span payload into shared immutable backing.
    fn share(&mut self) {
        if let RawSpanStorage::Owned(bytes) = &mut self.storage {
            let bytes = std::mem::take(bytes);
            self.storage = RawSpanStorage::Shared(Arc::from(bytes));
        }
    }

    /// Capture one immutable image for this raw span.
    pub(crate) fn image(&mut self) -> Arc<[u8]> {
        self.share();

        match &self.storage {
            RawSpanStorage::Owned(_) => unreachable!(),
            RawSpanStorage::Shared(bytes) => bytes.clone(),
        }
    }

    /// Restore one raw span from one immutable image.
    pub(crate) fn from_image(bytes: Arc<[u8]>) -> Self {
        Self {
            storage: RawSpanStorage::Shared(bytes),
        }
    }

    /// Write one byte at the given index.
    pub(crate) fn set(&mut self, index: usize, byte: u8) -> bool {
        let bytes = self.bytes_mut();
        let Some(slot) = bytes.get_mut(index) else {
            return false;
        };
        *slot = byte;
        true
    }

    /// Replace the entire byte payload.
    pub(crate) fn replace(&mut self, bytes: &[u8]) {
        self.storage = RawSpanStorage::Owned(bytes.to_vec().into_boxed_slice());
    }

    /// Return mutable byte storage, detaching shared backing on first write.
    fn bytes_mut(&mut self) -> &mut [u8] {
        if let RawSpanStorage::Shared(bytes) = &self.storage {
            self.storage = RawSpanStorage::Owned(bytes.as_ref().to_vec().into_boxed_slice());
        }

        match &mut self.storage {
            RawSpanStorage::Owned(bytes) => bytes.as_mut(),
            RawSpanStorage::Shared(_) => unreachable!(),
        }
    }

    /// Return the retained heap bytes owned by this raw span outside its inline form.
    pub(crate) fn retained_bytes(&self) -> usize {
        match &self.storage {
            RawSpanStorage::Owned(bytes) => std::mem::size_of_val(bytes.as_ref()),
            RawSpanStorage::Shared(bytes) => std::mem::size_of_val(bytes.as_ref()),
        }
    }
}

/// Raw heap storage for either value slots or byte buffers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum RawAllocationStorage {
    /// Slot storage for value based allocations.
    Values(ValueCell),
    /// Span storage for raw payloads.
    Bytes(RawSpanReference),
}

/// A raw heap allocation record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawAllocation {
    /// The raw storage backing this allocation.
    pub(crate) storage: RawAllocationStorage,
}

impl Default for RawAllocation {
    /// Create one empty raw allocation.
    fn default() -> Self {
        Self::new()
    }
}

impl RawAllocation {
    /// Create a new empty value allocation.
    pub fn new() -> Self {
        Self {
            storage: RawAllocationStorage::Values(ValueCell::default()),
        }
    }

    /// Create a value allocation with the given slot count.
    pub fn with_slots(count: usize) -> Self {
        Self {
            storage: RawAllocationStorage::Values(ValueCell::with_values_len(count)),
        }
    }

    /// Create a value allocation with the given slot values.
    pub fn with_values(values: Vec<Value>) -> Self {
        Self {
            storage: RawAllocationStorage::Values(ValueCell::with_values(values)),
        }
    }

    /// Return the retained heap bytes owned by this allocation outside its inline form.
    pub fn retained_bytes(&self) -> usize {
        match &self.storage {
            RawAllocationStorage::Values(values) => values.retained_bytes(),
            RawAllocationStorage::Bytes(_) => 0,
        }
    }

    /// Report whether this allocation stores raw bytes.
    pub fn is_bytes(&self) -> bool {
        matches!(self.storage, RawAllocationStorage::Bytes(_))
    }

    /// Report whether this allocation stores value slots.
    pub fn is_values(&self) -> bool {
        matches!(self.storage, RawAllocationStorage::Values(_))
    }

    /// Return the inline value storage when this allocation stores values.
    pub fn values(&self) -> Option<&ValueCell> {
        match &self.storage {
            RawAllocationStorage::Values(values) => Some(values),
            RawAllocationStorage::Bytes(_) => None,
        }
    }

    /// Return the mutable inline value storage when this allocation stores values.
    pub fn values_mut(&mut self) -> Option<&mut ValueCell> {
        match &mut self.storage {
            RawAllocationStorage::Values(values) => Some(values),
            RawAllocationStorage::Bytes(_) => None,
        }
    }
}

impl RetainedBytes for RawAllocation {
    fn retained_bytes(&self) -> usize {
        self.retained_bytes()
    }
}
