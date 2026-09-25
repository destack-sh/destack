use serde::{Deserialize, Serialize};
use tspp_core::SectionEntry;
use tspp_serde::Reflect;

/// One platform function imported by native code.
#[repr(u32)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub enum Import {
    /// Platform unwind personality function used by generated frames.
    UnwindPersonality = 0x00,

    /// C `memcpy`.
    Memcpy = 0x10,
    /// C `memmove`.
    Memmove = 0x11,
    /// C `memset`.
    Memset = 0x12,
    /// C `memcmp`.
    Memcmp = 0x13,

    /// Single-precision ceiling.
    CeilF32 = 0x20,
    /// Double-precision ceiling.
    CeilF64 = 0x21,
    /// Single-precision floor.
    FloorF32 = 0x22,
    /// Double-precision floor.
    FloorF64 = 0x23,
    /// Single-precision truncation toward zero.
    TruncF32 = 0x24,
    /// Double-precision truncation toward zero.
    TruncF64 = 0x25,
    /// Single-precision rounding to nearest, ties to even.
    NearestF32 = 0x26,
    /// Double-precision rounding to nearest, ties to even.
    NearestF64 = 0x27,
    /// Single-precision fused multiply-add.
    FmaF32 = 0x28,
    /// Double-precision fused multiply-add.
    FmaF64 = 0x29,
    /// Single-precision floating-point remainder.
    RemainderF32 = 0x2a,
    /// Double-precision floating-point remainder.
    RemainderF64 = 0x2b,

    /// Single-precision sine.
    SinF32 = 0x40,
    /// Double-precision sine.
    SinF64 = 0x41,
    /// Single-precision cosine.
    CosF32 = 0x42,
    /// Double-precision cosine.
    CosF64 = 0x43,
    /// Single-precision tangent.
    TanF32 = 0x44,
    /// Double-precision tangent.
    TanF64 = 0x45,
    /// Single-precision arc sine.
    AsinF32 = 0x46,
    /// Double-precision arc sine.
    AsinF64 = 0x47,
    /// Single-precision arc cosine.
    AcosF32 = 0x48,
    /// Double-precision arc cosine.
    AcosF64 = 0x49,
    /// Single-precision arc tangent.
    AtanF32 = 0x4a,
    /// Double-precision arc tangent.
    AtanF64 = 0x4b,
    /// Single-precision two-argument arc tangent.
    Atan2F32 = 0x4c,
    /// Double-precision two-argument arc tangent.
    Atan2F64 = 0x4d,
    /// Single-precision natural exponential.
    ExpF32 = 0x4e,
    /// Double-precision natural exponential.
    ExpF64 = 0x4f,
    /// Single-precision base-two exponential.
    Exp2F32 = 0x50,
    /// Double-precision base-two exponential.
    Exp2F64 = 0x51,
    /// Single-precision natural logarithm.
    LogF32 = 0x52,
    /// Double-precision natural logarithm.
    LogF64 = 0x53,
    /// Single-precision base-two logarithm.
    Log2F32 = 0x54,
    /// Double-precision base-two logarithm.
    Log2F64 = 0x55,
    /// Single-precision base-ten logarithm.
    Log10F32 = 0x56,
    /// Double-precision base-ten logarithm.
    Log10F64 = 0x57,
    /// Single-precision power.
    PowF32 = 0x58,
    /// Double-precision power.
    PowF64 = 0x59,
    /// Single-precision cube root.
    CbrtF32 = 0x5a,
    /// Double-precision cube root.
    CbrtF64 = 0x5b,
    /// Single-precision natural exponential minus one.
    Expm1F32 = 0x5c,
    /// Double-precision natural exponential minus one.
    Expm1F64 = 0x5d,
    /// Single-precision natural logarithm after adding one.
    Log1pF32 = 0x5e,
    /// Double-precision natural logarithm after adding one.
    Log1pF64 = 0x5f,
}

/// One platform import pointer inside linked native code.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct ImportRelocation {
    /// Byte offset of the target pointer inside the linked image.
    pub offset: u32,
    /// Platform function written into the target pointer.
    pub import: Import,
}

impl ImportRelocation {
    /// Create one platform import relocation.
    pub const fn new(offset: u32, import: Import) -> Self {
        Self { offset, import }
    }

    /// Return whether the target pointer lies inside its native image.
    pub fn is_within(self, byte_len: usize) -> bool {
        self.offset
            .checked_add(size_of::<usize>() as u32)
            .is_some_and(|end| end as usize <= byte_len)
    }
}
