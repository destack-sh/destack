/// Shared runtime entry descriptor for all engines.
#[derive(Debug, Clone)]
pub enum Entry {
    /// VM entrypoint resolved by name.
    Vm {
        /// Fully qualified entry name.
        name: String,
    },
    /// Native entrypoint resolved by symbol.
    Native {
        /// Fully qualified entry name.
        name: String,
        /// Raw function symbol pointer.
        symbol: *const (),
    },
}

impl Entry {
    /// Create one VM entry descriptor by name.
    pub fn vm(name: impl Into<String>) -> Self {
        Self::Vm { name: name.into() }
    }

    /// Create one native entry descriptor by symbol.
    pub fn native(name: impl Into<String>, symbol: *const ()) -> Self {
        Self::Native {
            name: name.into(),
            symbol,
        }
    }

    /// Return the fully qualified entry name.
    pub fn name(&self) -> &str {
        match self {
            Self::Vm { name } | Self::Native { name, .. } => name,
        }
    }
}
