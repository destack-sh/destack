use destack_engine::StaticId;
use serde::{Deserialize, Serialize};

use crate::{CodeOffset, NativeImport};

/// One native function id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct NativeFunctionId(pub u32);

/// Machine relocation encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelocationKind {
    /// Write one absolute 64-bit address.
    Absolute64,
    /// Write one 32-bit program-counter-relative displacement.
    Relative32,
}

/// Relocation target referenced by generated code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelocationTarget {
    /// One native import.
    Import(NativeImport),
    /// One function inside the same program.
    Function(NativeFunctionId),
    /// One static region address.
    Static(StaticId),
}

/// One relocation applied while loading native code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relocation {
    /// The code offset patched by this relocation.
    pub offset: CodeOffset,
    /// The relocation encoding.
    pub kind: RelocationKind,
    /// The relocation target.
    pub target: RelocationTarget,
    /// The target addend in bytes.
    pub addend: i64,
}
