use std::num::NonZeroU32;

use destack_mir::LayoutId;
use serde::{Deserialize, Serialize};

/// One stable storage layout identifier shared across heap and VM boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct StorageLayoutId(NonZeroU32);

impl StorageLayoutId {
    /// Create one storage layout identifier from one raw value.
    #[inline]
    pub const fn new(raw: u32) -> Option<Self> {
        match NonZeroU32::new(raw) {
            Some(raw) => Some(Self(raw)),
            None => None,
        }
    }

    /// Return the raw storage layout identifier value.
    #[inline]
    pub const fn raw(self) -> u32 {
        self.0.get()
    }

    /// Encode one concrete layout identifier as one storage layout identifier.
    #[inline]
    pub const fn from_layout(layout_id: LayoutId) -> Self {
        // safety: MIR layout identifiers are backed by NonZeroU32
        Self(unsafe { NonZeroU32::new_unchecked(layout_id.raw()) })
    }
}
