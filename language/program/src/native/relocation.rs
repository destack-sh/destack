use crate::StaticId;
use serde::{Deserialize, Serialize};

/// One native relocation applied when materializing code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relocation {
    /// Byte offset patched by this relocation.
    pub offset: u32,
    /// Relocation encoding.
    pub kind: RelocationKind,
    /// Relocation target.
    pub target: RelocationTarget,
    /// Target addend in bytes.
    pub addend: i64,
}

impl Relocation {
    /// Create one native relocation.
    pub fn new(offset: u32, kind: RelocationKind, target: RelocationTarget, addend: i64) -> Self {
        Self {
            offset,
            kind,
            target,
            addend,
        }
    }
}

/// Native relocation encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelocationKind {
    /// Write one absolute 64-bit address.
    Absolute64,
    /// Write one 32-bit program-counter-relative displacement.
    Relative32,
}

/// Native relocation target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelocationTarget {
    /// One imported runtime or host symbol by import index.
    Import(u32),
    /// One native function by function index.
    Function(u32),
    /// One static region address.
    Static(StaticId),
}
