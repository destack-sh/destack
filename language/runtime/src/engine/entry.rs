/// Entry point handle for runtime engines.
#[derive(Debug, Clone)]
pub enum EntryPoint {
    /// VM entry point resolved by name.
    Vm { name: String },
    /// Native entry point resolved by symbol.
    Native {
        /// Fully qualified entry name.
        name: String,
        /// Raw function symbol pointer.
        symbol: *const (),
    },
}

impl EntryPoint {
    /// Create a VM entry point by name.
    pub fn vm(name: impl Into<String>) -> Self {
        Self::Vm { name: name.into() }
    }

    /// Create a native entry point by symbol.
    pub fn native(name: impl Into<String>, symbol: *const ()) -> Self {
        Self::Native {
            name: name.into(),
            symbol,
        }
    }

    /// Return the entry point name.
    pub fn name(&self) -> &str {
        match self {
            EntryPoint::Vm { name } => name,
            EntryPoint::Native { name, .. } => name,
        }
    }
}
