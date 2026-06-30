use destack_core::{EntryRange, EntryStore, SectionEntry};
use serde::{Deserialize, Serialize};

use destack_serde::Reflect;

/// Constant value stored in lowered instructions.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ConstValue {
    /// Constant payload bytes.
    pub bytes: EntryRange<u8>,
}

/// Build-time constant value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ConstValueBuilder {
    /// Constant payload bytes.
    pub bytes: Box<[u8]>,
}

impl ConstValueBuilder {
    /// Create one aggregate constant.
    pub fn aggregate(bytes: Box<[u8]>) -> Self {
        Self { bytes }
    }

    /// Build this constant into one section entry.
    pub(crate) fn build(self, bytes: &mut EntryStore<u8>) -> ConstValue {
        ConstValue {
            bytes: bytes.append(self.bytes),
        }
    }
}

// SAFETY: constant entries contain only section ranges.
unsafe impl SectionEntry for ConstValue {}
