/// Shared runtime entry descriptor for all execution backends.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    /// The fully qualified entry name.
    name: String,
}

impl Entry {
    /// Create one entry descriptor by name.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Return the fully qualified entry name.
    pub fn name(&self) -> &str {
        &self.name
    }
}
