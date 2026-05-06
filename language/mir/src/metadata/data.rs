use serde::{Deserialize, Serialize};

/// Target data layout for one MIR module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataLayout {
    /// Pointer size in bytes.
    pub pointer_bytes: u8,
}

impl Default for DataLayout {
    fn default() -> Self {
        Self { pointer_bytes: 8 }
    }
}

impl DataLayout {
    /// Return pointer width in bits.
    pub fn pointer_bits(self) -> u16 {
        u16::from(self.pointer_bytes) * 8
    }
}
