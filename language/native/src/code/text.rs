use crate::Relocation;

/// Object file text bytes and relocations.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Text {
    /// The generated machine code bytes.
    pub bytes: Vec<u8>,
    /// The relocations applied when loading the code.
    pub relocations: Vec<Relocation>,
}
