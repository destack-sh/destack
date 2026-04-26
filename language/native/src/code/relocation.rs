use crate::{CodeOffset, Import};

/// Machine relocation encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RelocationKind {
    /// Write one absolute 64-bit address.
    Absolute64,
    /// Write one 32-bit program-counter-relative displacement.
    Relative32,
}

/// Relocation target referenced by generated code.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RelocationTarget {
    /// One runtime helper call.
    Runtime(Import),
    /// One function entry inside the same program.
    Function(destack_engine::FunctionId),
    /// One static region address.
    Static(destack_engine::StaticId),
}

/// One relocation applied while loading native code.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
