use serde::{Deserialize, Serialize};

/// Linker-visible native symbol.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NativeSymbol {
    /// The symbol name.
    name: String,
}

impl NativeSymbol {
    /// Create one native symbol.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Return the symbol name.
    pub fn name(&self) -> &str {
        &self.name
    }
}
