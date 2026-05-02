use serde::{Deserialize, Serialize};

use crate::Relocation;

/// Native text section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Text {
    /// The generated machine code bytes.
    pub bytes: Vec<u8>,
    /// The relocations applied when loading the code.
    pub relocations: Vec<Relocation>,
}
