use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::RegisterSpan;

/// One register value placed inside a packed aggregate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Placement {
    /// The register words containing the source value.
    pub registers: RegisterSpan,
    /// The destination byte offset inside the aggregate.
    pub byte_offset: u32,
    /// The exact number of source bytes to place.
    pub byte_len: u32,
}

impl Placement {
    /// The encoded byte length of one placement.
    pub(crate) const BYTE_LEN: usize = size_of::<u16>() * 2 + size_of::<u32>() * 2;

    /// Create one physical aggregate placement.
    pub const fn new(registers: RegisterSpan, byte_offset: u32, byte_len: u32) -> Self {
        Self {
            registers,
            byte_offset,
            byte_len,
        }
    }
}
